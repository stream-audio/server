use std::collections::VecDeque;
use std::fmt;
use std::fs;
use std::io;
use std::mem::size_of;
use std::path::Path;

use hound;

#[derive(Debug)]
pub struct Error {
    repr: Box<ErrorRepr>,
}
impl Error {
    fn new(kind: ErrorKind, output: OutputType) -> Self {
        Self {
            repr: Box::new(ErrorRepr { kind, output }),
        }
    }
}

#[derive(Debug)]
pub struct ErrorRepr {
    pub kind: ErrorKind,
    pub output: OutputType,
}

#[derive(Debug)]
pub enum ErrorKind {
    IoError(io::Error),
    FormatError(&'static str),
    UnfinishedSample,
    AnotherError(String),
}
impl From<hound::Error> for ErrorKind {
    fn from(e: hound::Error) -> Self {
        match e {
            hound::Error::IoError(e) => ErrorKind::IoError(e),
            hound::Error::FormatError(s) => ErrorKind::FormatError(s),
            hound::Error::UnfinishedSample => ErrorKind::UnfinishedSample,
            hound::Error::TooWide
            | hound::Error::Unsupported
            | hound::Error::InvalidSampleFormat => ErrorKind::AnotherError(e.to_string()),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.repr.kind {
            ErrorKind::IoError(ref e) => write!(f, "{}", e),
            ErrorKind::FormatError(ref s) => write!(f, "{}", s),
            ErrorKind::UnfinishedSample => write!(f, "unfinished sample"),
            ErrorKind::AnotherError(ref s) => write!(f, "{}", s),
        }?;
        write!(f, ". Audio output: {}", self.repr.output)
    }
}
impl ::std::error::Error for Error {}

type Sample = i16;

#[derive(Clone, Debug)]
pub enum OutputType {
    File(String),
}
impl fmt::Display for OutputType {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            OutputType::File(ref fname) => write!(f, "file '{}'", fname),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub channels: u16,
    pub sample_rate: f64,
    pub output: OutputType,
}

pub trait AudioFactory {
    fn create_writer(&self, settings: Settings) -> Result<Box<dyn AudioWriter>, Error>;
}

pub trait AudioWriter {
    fn write_sample(&mut self, sample: Sample) -> Result<(), Error>;
    fn write_samples_slice(&mut self, samples: &[Sample]) -> Result<(), Error>;
    fn write_samples_vec_deq(&mut self, samples: &VecDeque<Sample>) -> Result<(), Error>;
}

pub enum AudioType {
    Wav,
}

pub fn create_factory(audio_type: AudioType) -> Result<Box<dyn AudioFactory>, Error> {
    match audio_type {
        AudioType::Wav => Ok(Box::new(WavHoundFactory {})),
    }
}

struct WavHoundFactory {}

impl AudioFactory for WavHoundFactory {
    fn create_writer(&self, settings: Settings) -> Result<Box<dyn AudioWriter>, Error> {
        let hound_spec = hound::WavSpec {
            channels: settings.channels,
            sample_rate: settings.sample_rate as u32,
            bits_per_sample: (size_of::<Sample>() * 8) as u16,
            sample_format: hound::SampleFormat::Int,
        };

        let w = WavHoundWriter::create(settings.output, hound_spec)?;
        Ok(w)
    }
}

struct WavHoundWriter {
    repr: hound::WavWriter<io::BufWriter<fs::File>>,
    output: OutputType,
}
impl WavHoundWriter {
    fn create(output: OutputType, spec: hound::WavSpec) -> Result<Box<WavHoundWriter>, Error> {
        let repr = match &output {
            OutputType::File(fname) => hound::WavWriter::create(Path::new(&fname), spec)
                .map_err(|e| Error::new(e.into(), output.clone()))?,
        };
        Ok(Box::new(WavHoundWriter { repr, output }))
    }
}
impl AudioWriter for WavHoundWriter {
    fn write_sample(&mut self, sample: Sample) -> Result<(), Error> {
        self.repr
            .write_sample(sample)
            .map_err(|e| Error::new(e.into(), self.output.clone()))
    }

    fn write_samples_slice(&mut self, samples: &[i16]) -> Result<(), Error> {
        for sample in samples {
            self.write_sample(*sample)?;
        }
        Ok(())
    }

    fn write_samples_vec_deq(&mut self, samples: &VecDeque<Sample>) -> Result<(), Error> {
        for sample in samples {
            self.write_sample(*sample)?;
        }
        Ok(())
    }
}
