#[allow(dead_code, unused_attributes, bad_style)]
mod ffmpeg_ffi;
#[macro_use]
mod macroses;
mod codec;
pub mod error;
mod ffmpeg_const;
mod resample;

#[link(name = "avutil")]
#[link(name = "avcodec")]
#[link(name = "swresample")]
extern "C" {}

pub use codec::{Codec, Decoder, Encoder, Params as CodecParams};
pub use error::{Error, ErrorRepr, InternalError};
pub use resample::Resampler;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioParams {
    pub rate: i32,
    pub format: AudioSampleFormat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AudioSampleFormat {
    U8,
    S16Le,
    FloatLe,
    U8P,
    S16LeP,
    FloatLeP,
}

impl AudioSampleFormat {
    fn from_raw(raw: ffmpeg_ffi::AVSampleFormat) -> Option<Self> {
        match raw {
            ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_U8 => Some(AudioSampleFormat::U8),
            ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_S16 => Some(AudioSampleFormat::S16Le),
            ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_FLT => Some(AudioSampleFormat::FloatLe),
            ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_U8P => Some(AudioSampleFormat::U8P),
            ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_S16P => Some(AudioSampleFormat::S16LeP),
            ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_FLTP => Some(AudioSampleFormat::FloatLeP),
            _ => None,
        }
    }

    fn to_raw(self) -> ffmpeg_ffi::AVSampleFormat {
        match self {
            AudioSampleFormat::U8 => ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_U8,
            AudioSampleFormat::S16Le => ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_S16,
            AudioSampleFormat::FloatLe => ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_FLT,
            AudioSampleFormat::U8P => ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_U8P,
            AudioSampleFormat::S16LeP => ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_S16P,
            AudioSampleFormat::FloatLeP => ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_FLTP,
        }
    }

    fn get_sample_size(self) -> usize {
        match self {
            AudioSampleFormat::U8 | AudioSampleFormat::U8P => 1,
            AudioSampleFormat::S16Le | AudioSampleFormat::S16LeP => 2,
            AudioSampleFormat::FloatLe | AudioSampleFormat::FloatLeP => 4,
        }
    }
}

impl AudioParams {
    fn get_channels_qty(&self) -> i32 {
        unsafe { ffmpeg_ffi::av_get_channel_layout_nb_channels(resample::LAYOUT as _) as _ }
    }
    fn get_samples_qty_in_buffer(&self, len: usize) -> usize {
        len / self.get_channels_qty() as usize / self.format.get_sample_size()
    }
}

// Errors:
