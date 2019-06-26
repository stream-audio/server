use audio_sharing_pc::alsa;
use audio_sharing_pc::audio_saver;
use audio_sharing_pc::error::*;
use audio_sharing_pc::exit_listener;
use audio_sharing_pc::ffmpeg;
use audio_sharing_pc::net_server;
use audio_sharing_pc::thread_buffer;
use std::process::exit;
use std::sync::atomic::Ordering;

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

struct AudioWriter {
    file_writer: Box<dyn audio_saver::AudioWriter>,
    player: alsa::SndPcm,
}

impl thread_buffer::DataReceiver for AudioWriter {
    fn new_slice(&mut self, data: &[u8]) -> Result<(), Error> {
        self.file_writer.write_bytes_slice(data)?;
        self.player.write_interleaved(data)?;
        Ok(())
    }
}

fn record(name: String, params: alsa::Params) -> Result<(), Error> {
    let pcm_recorder = alsa::SndPcm::open(name, alsa::Stream::Capture, params)?;
    let record_params = pcm_recorder.get_params();
    let pcm_player = alsa::SndPcm::open("default".to_owned(), alsa::Stream::Playback, params)?;

    println!("Opened '{}'", pcm_recorder.info()?.get_id());
    println!("Capture settings: {}", pcm_recorder.dump_settings()?);
    println!("Player settings: {}", pcm_player.dump_settings()?);

    let mut resampler = if params != record_params {
        Some(ffmpeg::Resampler::new(record_params.into(), params.into())?)
    } else {
        None
    };

    let encoder_params = ffmpeg::CodecParams {
        codec: ffmpeg::Codec::Aac,
        bit_rate: 128000,
        audio_params: params.into(),
    };
    let mut encoder = ffmpeg::Encoder::new(encoder_params)?;
    let mut decoder = ffmpeg::Decoder::new(encoder_params.codec)?;

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

    let mut thread_writer = thread_buffer::ThreadBuffer::new(Box::new(AudioWriter {
        file_writer,
        player: pcm_player,
    }));

    let mut buffer = vec![0; 1024];
    pcm_recorder.reset()?;
    loop {
        if on_exit_flag.load(Ordering::SeqCst) {
            eprintln!("Caught Signal, finishing job");
            break;
        }

        let read = pcm_recorder.read_interleaved(buffer.as_mut_slice())?;
        let data = &buffer[..read];
        let data = if let Some(resampler) = &mut resampler {
            resampler.resample(data)?
        } else {
            data
        };

        encoder.write(data)?;
        while let Some(encoded_data) = encoder.read()? {
            decoder.write(encoded_data)?;
            while let Some(data) = decoder.read()? {
                server.send_to_all(data)?;
                thread_writer.write_data(data)?;
            }
        }
    }

    pcm_recorder.stop()?;
    thread_writer.stop_and_join();

    Ok(())
}

fn real_main() -> Result<(), Error> {
    list_alsa_devices()?;

    let params = alsa::Params {
        format: alsa::Format::FloatLe,
        channels: 2,
        rate: 44100,
    };

    record("hw:3,1".to_owned(), params)?;

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
