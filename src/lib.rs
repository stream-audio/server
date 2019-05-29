#[macro_use(select)]
extern crate crossbeam_channel as channel;

use portaudio as pa;
use portaudio::PortAudio;

pub mod audio_saver;
pub mod error;
mod exit_listener;

use crate::error::*;

const SAMPLE_RATE: f64 = 44_100.0;
const CHANNELS: i32 = 1;
const FRAMES: u32 = 256;
const INTERLEAVED: bool = true;

type Sample = i16;

pub fn run() -> Result<(), Error> {
    let pa = PortAudio::new()?;

    println!("PortAudio version : {:?}", pa.version());
    println!("PortAudio version text : {:?}", pa.version_text());
    println!("PortAudio devices : {:?}", pa.device_count()?);
    println!("PortAudio : {:?}", pa);

    let input_idx = pa.default_input_device()?;
    let input_dev = pa.device_info(input_idx)?;

    println!("PortAudio default input device: {:?}", input_idx);

    for dev in pa.devices()? {
        let (dev_idx, dev_info) = dev?;
        println!(
            "PortAudio devices : {:?} {:?} {}",
            dev_idx, dev_info, dev_info.name
        );
    }

    let latency = input_dev.default_low_input_latency;
    let input_params =
        pa::StreamParameters::<Sample>::new(input_idx, CHANNELS, INTERLEAVED, latency);
    let settings = pa::InputStreamSettings::new(input_params, SAMPLE_RATE, FRAMES);

    save_input_to_file(&pa, settings)?;

    Result::Ok(())
}

fn save_input_to_file(
    pa: &PortAudio,
    pa_settings: pa::InputStreamSettings<Sample>,
) -> Result<(), Error> {
    let fname = "/tmp/audio.dump";
    let writer_settings = audio_saver::Settings {
        channels: 1,
        sample_rate: pa_settings.sample_rate,
        output: audio_saver::OutputType::File(fname.to_owned()),
    };
    let mut writer =
        audio_saver::create_factory(audio_saver::AudioType::Wav)?.create_writer(writer_settings)?;

    let (err_sender, err_receiver) = channel::unbounded::<Error>();

    let callback = move |args: pa::InputStreamCallbackArgs<Sample>| {
        let r = writer.write_samples_slice(args.buffer);
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
            pa::StreamAvailable::Frames(frames) => return Result::Ok(frames as u32),
            pa::StreamAvailable::InputOverflowed => eprintln!("Input Overflowed"),
            pa::StreamAvailable::OutputUnderflowed => eprintln!("Output Underflowed"),
        }
    }
}

#[allow(type_alias_bounds)]
type Stream<F: pa::stream::Reader> = pa::Stream<pa::Blocking<F::Buffer>, F>;

const FILE_BUFFER_SIZE: usize = 4 * 1024;

fn save_input_to_file<F>(
    stream: &mut Stream<F>,
    settings: pa::InputStreamSettings<Sample>,
) -> Result<(), Error>
where
    F: pa::stream::Reader<Sample = Sample>,
{
    stream.start()?;
    let mut buffer = Vec::<Sample>::new();

    let fname = "/tmp/audio.dump";
    let writer_settings = audio_saver::Settings {
        channels: 1,
        sample_rate: settings.sample_rate,
        output: audio_saver::OutputType::File(fname.to_owned()),
    };

    let mut writer =
        audio_saver::create_factory(audio_saver::AudioType::Wav)?.create_writer(writer_settings)?;

    let mut cnt: usize = 0;
    loop {
        let frames_num = wait_for_stream(|| stream.read_available())?;
        if frames_num == 0 {
            continue;
        }

        buffer.extend(stream.read(frames_num)?.into_iter());
        cnt += frames_num as usize;

        if buffer.len() >= FILE_BUFFER_SIZE {
            writer.write_samples_vec(&buffer)?;
            buffer.clear();
        }

        if cnt > 200000 {
            break;
        }
    }

    eprintln!("cnt: {}", cnt);

    Ok(())
}
*/
