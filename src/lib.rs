#[macro_use(select)]
extern crate crossbeam_channel as channel;

use crate::error::*;
use portaudio as pa;

pub mod alsa;
#[allow(dead_code, unused_attributes, bad_style)]
pub mod alsa_ffi;
pub mod audio_saver;
pub mod error;
mod exit_listener;

const CHANNELS: i32 = 1;
const FRAMES: u32 = 256; //4096; //8192;
const INTERLEAVED: bool = true;

type Sample = f32;

pub fn run() -> Result<(), Error> {
    let pa = pa::PortAudio::new()?;

    println!("PortAudio version : {:?}", pa.version());
    println!("PortAudio version text : {:?}", pa.version_text());
    println!("PortAudio devices : {:?}", pa.device_count()?);
    println!("PortAudio : {:?}", pa);

    let mut input_idx = pa.default_input_device()?;

    println!("PortAudio default input device: {:?}", input_idx);

    for dev in pa.devices()? {
        let (dev_idx, dev_info) = dev?;
        println!(
            "PortAudio devices: {:?} {} {}",
            dev_idx, dev_info.name, dev_info.default_sample_rate
        );
        if dev_info.name.contains("Loopback") {
            input_idx = dev_idx;
        }
    }

    println!("Choose input device: {:?}", input_idx);

    let input_dev = pa.device_info(input_idx)?;
    let sample_rate = input_dev.default_sample_rate; //48000 as f64;
    let latency = input_dev.default_low_input_latency;

    let input_params =
        pa::StreamParameters::<Sample>::new(input_idx, CHANNELS, INTERLEAVED, latency);

    let settings = pa::InputStreamSettings::new(input_params, sample_rate, FRAMES);

    if pa
        .is_input_format_supported(input_params, sample_rate)
        .is_ok()
    {
        println!("Input format is supported. sample_rate: {}", sample_rate);
    }

    save_input_to_file(&pa, settings)?;
    //save_input_to_file_sync(&pa, settings)?;

    Ok(())
}

fn save_input_to_file(
    pa: &pa::PortAudio,
    pa_settings: pa::InputStreamSettings<Sample>,
) -> Result<(), Error> {
    let fname = "/tmp/audio.dump";
    let writer_settings = audio_saver::Settings {
        channels: CHANNELS as u16,
        sample_rate: pa_settings.sample_rate,
        output: audio_saver::OutputType::File(fname.to_owned()),
    };
    let writer =
        audio_saver::create_factory(audio_saver::AudioType::Wav)?.create_writer(writer_settings)?;

    let (err_sender, err_receiver) = channel::unbounded::<Error>();

    let thread_writer = audio_saver::ThreadAudioWriter::new(writer);

    let callback = move |args: pa::InputStreamCallbackArgs<Sample>| {
        let r = thread_writer.write_samples_slice(args.buffer);
        if let Err(e) = r {
            let send_res = err_sender.send(e.into());
            if send_res.is_err() {
                return pa::Complete;
            }
        }
        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(pa_settings, callback)?;

    let on_exit_receiver = exit_listener::listen_on_exit()?;

    stream.start()?;

    let mut res: Result<(), Error> = Ok(());

    select! {
        recv(on_exit_receiver) -> _ => eprintln!("On exit signal received"),
        recv(err_receiver) -> msg => {
            res = Err(msg?);
        },
    }

    stream.stop()?;
    stream.close()?;
    res
}

/*
fn wait_for_stream(f: impl Fn() -> Result<pa::StreamAvailable, pa::Error>) -> Result<u32, Error> {
    loop {
        match f()? {
            pa::StreamAvailable::Frames(frames) => return Ok(frames as u32),
            pa::StreamAvailable::InputOverflowed => eprintln!("Input Overflowed"),
            pa::StreamAvailable::OutputUnderflowed => eprintln!("Output Underflowed"),
        }
    }
}
//
//#[allow(type_alias_bounds)]
//type Stream<F: pa::stream::Reader> = pa::Stream<pa::Blocking<F::Buffer>, F>;

const FILE_BUFFER_SIZE: usize = 4 * 1024;

fn save_input_to_file_sync(
    pa: &pa::PortAudio,
    pa_settings: pa::InputStreamSettings<Sample>,
) -> Result<(), Error> {
    let mut stream = pa.open_blocking_stream(pa_settings)?;

    stream.start()?;
    let mut buffer = Vec::<Sample>::new();

    let fname = "/tmp/audio.dump";
    let writer_settings = audio_saver::Settings {
        channels: CHANNELS as u16,
        sample_rate: pa_settings.sample_rate,
        output: audio_saver::OutputType::File(fname.to_owned()),
    };

    let mut writer =
        audio_saver::create_factory(audio_saver::AudioType::Wav)?.create_writer(writer_settings)?;

    let on_exit_receiver = exit_listener::listen_on_exit()?;

    loop {
        let frames_num = wait_for_stream(|| stream.read_available())?;
        if frames_num == 0 {
            continue;
        }

        buffer.extend_from_slice(stream.read(frames_num)?);

        if on_exit_receiver.try_recv().is_ok() {
            break;
        }

        if buffer.len() >= FILE_BUFFER_SIZE {
            writer.write_samples_slice(&buffer)?;
            buffer.clear();
        }
    }

    stream.stop()?;

    dbg!();
    stream.close()?;
    dbg!();

    Ok(())
}

*/
