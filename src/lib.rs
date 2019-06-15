extern crate crossbeam_channel as channel;

use crate::error::*;
use portaudio as pa;

pub mod alsa;
pub mod audio_saver;
pub mod error;
mod exit_listener;
pub mod net_server;
mod thread_buffer;

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

    // save_input_to_file(&pa, settings)?;
    //save_input_to_file_sync(&pa, settings)?;

    Ok(())
}
