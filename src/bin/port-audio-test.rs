use audio_sharing_pc::alsa;
use audio_sharing_pc::audio_saver;
use audio_sharing_pc::error::*;
use audio_sharing_pc::exit_listener;
use audio_sharing_pc::net_server;
use audio_sharing_pc::thread_buffer;
use clap;
use std::process::exit;
use std::sync::atomic::Ordering;
use stream_audio_ffmpeg as ffmpeg;

pub fn list_alsa_devices() -> Result<(), Error> {
    for ctl in alsa::SndCtl::list_cards() {
        let ctl = ctl?;
        let info = ctl.card_info()?;

        for dev_info in ctl.list_devices_info() {
            let dev_info = dev_info?;

            println!(
                "Card: #{}, {} [{}]. Device: #{}, {}, [{}]. Subdevices qty: {}",
                ctl.get_card_num(),
                info.get_id(),
                info.get_name(),
                dev_info.get_dev_num(),
                dev_info.get_id(),
                dev_info.get_name(),
                dev_info.get_subdevice_count(),
            );
        }
    }

    Ok(())
}

struct ThreadPlayer {
    _file_writer: Box<dyn audio_saver::AudioWriter>,
    player: alsa::SndPcm,
}

struct ThreadServerWriter {
    server: net_server::NetServer,
    encoder: ffmpeg::Encoder,
}

impl thread_buffer::DataReceiver for ThreadPlayer {
    fn new_slice(&mut self, data: &[u8]) -> Result<(), Error> {
        // self.file_writer.write_bytes_slice(data)?;
        self.player.write_interleaved(data)?;
        Ok(())
    }
}

impl thread_buffer::DataReceiver for ThreadServerWriter {
    fn new_slice(&mut self, data: &[u8]) -> Result<(), Error> {
        self.encoder.write(data)?;
        while let Some(data) = self.encoder.read()? {
            self.server.send_to_all(data)?;
        }
        Ok(())
    }
}

fn record(name: String, params: alsa::Params, should_play_locally: bool) -> Result<(), Error> {
    let pcm_recorder = alsa::SndPcm::open(name, alsa::Stream::Capture, params)?;
    let record_params = pcm_recorder.get_params();
    let pcm_player = if should_play_locally {
        Some(alsa::SndPcm::open(
            "default".to_owned(),
            alsa::Stream::Playback,
            params,
        )?)
    } else {
        None
    };

    println!("Opened '{}'", pcm_recorder.info()?.get_id());
    println!("Capture settings: {}", pcm_recorder.dump_settings()?);
    if let Some(p) = &pcm_player {
        println!("Player settings: {}", p.dump_settings()?);
    }

    let mut resampler = if params != record_params {
        Some(ffmpeg::Resampler::new(record_params.into(), params.into())?)
    } else {
        None
    };

    let encoder_params = ffmpeg::CodecParams {
        codec: ffmpeg::Codec::Aac,
        bit_rate: 96000,
        audio_params: params.into(),
    };
    let encoder = ffmpeg::Encoder::new(encoder_params)?;

    let on_exit_receiver = exit_listener::listen_on_exit()?;
    let on_exit_flag = on_exit_receiver.signal_flag.clone();

    let server = net_server::NetServer::new("0.0.0.0:25204".parse().unwrap(), on_exit_receiver)?;

    let writer_settings = audio_saver::Settings {
        channels: params.channels as u16,
        sample_rate: params.rate as f64,
        format: params.format.to_audio_saver_format(),
        output: audio_saver::OutputType::File("/tmp/audio.dump".to_owned()),
    };

    let file_writer =
        audio_saver::create_factory(audio_saver::AudioType::Wav)?.create_writer(writer_settings)?;

    let mut thread_player = if let Some(p) = pcm_player {
        Some(thread_buffer::ThreadBuffer::new(Box::new(ThreadPlayer {
            _file_writer: file_writer,
            player: p,
        })))
    } else {
        None
    };

    let mut thread_server_writer =
        thread_buffer::ThreadBuffer::new(Box::new(ThreadServerWriter { server, encoder }));

    let mut buffer = vec![0; 4068];
    pcm_recorder.reset()?;
    loop {
        if on_exit_flag.load(Ordering::SeqCst) {
            eprintln!("Caught Signal, finishing job");
            break;
        }

        let read = pcm_recorder.read_interleaved(buffer.as_mut_slice())?;
        let data = &buffer[..read];
        let data = match &mut resampler {
            Some(resampler) => resampler.resample(data)?,
            None => data,
        };

        if let Some(t) = &thread_player {
            t.write_data(data)?;
        }
        thread_server_writer.write_data(data)?;
    }

    pcm_recorder.stop()?;
    thread_player.as_mut().map(|t| t.stop_and_join());
    thread_server_writer.stop_and_join();

    Ok(())
}

fn real_main() -> Result<(), Error> {
    let matches = clap::App::new("Audio Streaming Server")
        .version("1.0")
        .author("Anton R.")
        .about("Captures audio using snd-aloop kernel module, and streams it to the android device")
        .arg(
            clap::Arg::with_name("play_locally")
                .short("p")
                .long("play-locally")
                .takes_value(false)
                .help("Replay captured audio locally to the default alsa device"),
        )
        .arg(
            clap::Arg::with_name("list_devices")
                .short("l")
                .long("-list-devices")
                .takes_value(false)
                .help("List alsa devices and exit"),
        )
        .arg(
            clap::Arg::with_name("hw_name")
                .short("d")
                .long("alsa-aloop-device")
                .takes_value(true)
                .default_value("hw:3,1")
                .help("Name of the alsa aloop device to listen audio from"),
        )
        .get_matches();

    if matches.is_present("list_devices") {
        list_alsa_devices()?;
        return Ok(());
    }

    let should_play_locally = matches.is_present("play_locally");
    let hw_name = matches.value_of("hw_name").unwrap();

    let params = alsa::Params {
        format: alsa::Format::FloatLe,
        channels: 2,
        rate: 44100,
    };

    record(hw_name.to_owned(), params, should_play_locally)?;

    Ok(())
}

fn main() {
    let ret_code = match real_main() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error occurred: {}", e);
            1
        }
    };

    exit(ret_code);
}
