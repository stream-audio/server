use hound;
use std::fmt;
use std::fs;
use std::io;
use std::mem::size_of;
use std::path::Path;

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
    pub format: Format,
    pub output: OutputType,
}

#[derive(Clone, Debug)]
pub enum Format {
    U8,
    S16Le,
    S32Le,
    FloatLe,
}

pub trait AudioFactory {
    fn create_writer(&self, settings: Settings) -> Result<Box<dyn AudioWriter>, Error>;
}

pub trait AudioWriter: Send {
    fn write_bytes_slice(&mut self, data: &[u8]) -> Result<(), Error>;
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

impl Format {
    fn bits_per_sample(&self) -> u16 {
        match self {
            Format::U8 => 8,
            Format::S16Le => 16,
            Format::S32Le | Format::FloatLe => 32,
        }
    }

    fn hound_format(&self) -> hound::SampleFormat {
        match self {
            Format::U8 | Format::S16Le | Format::S32Le => hound::SampleFormat::Int,
            Format::FloatLe => hound::SampleFormat::Float,
        }
    }
}

impl AudioFactory for WavHoundFactory {
    fn create_writer(&self, settings: Settings) -> Result<Box<dyn AudioWriter>, Error> {
        Ok(WavHoundWriter::create(settings)?)
    }
}

struct WavHoundWriter {
    repr: hound::WavWriter<io::BufWriter<fs::File>>,
    settings: Settings,
}
impl WavHoundWriter {
    fn create(settings: Settings) -> Result<Box<WavHoundWriter>, Error> {
        let spec = hound::WavSpec {
            channels: settings.channels,
            sample_rate: settings.sample_rate as u32,
            bits_per_sample: settings.format.bits_per_sample(),
            sample_format: settings.format.hound_format(),
        };

        let repr = match &settings.output {
            OutputType::File(fname) => hound::WavWriter::create(Path::new(&fname), spec)
                .map_err(|e| Error::new(e.into(), settings.output.clone()))?,
        };
        Ok(Box::new(WavHoundWriter { repr, settings }))
    }

    fn write_sample<S>(&mut self, sample: S) -> Result<(), Error>
    where
        S: hound::Sample + Copy,
    {
        self.repr
            .write_sample(sample)
            .map_err(|e| Error::new(e.into(), self.settings.output.clone()))
    }

    fn write_samples_slice<S>(&mut self, samples: &[S]) -> Result<(), Error>
    where
        S: hound::Sample + Copy,
    {
        for sample in samples {
            self.write_sample(*sample)?;
        }
        Ok(())
    }

    fn write_bytes_as_samples<S>(&mut self, data: &[u8]) -> Result<(), Error>
    where
        S: hound::Sample + Copy,
    {
        if data.len() % size_of::<S>() != 0 {
            return Err(Error::new(
                ErrorKind::FormatError("Size of data is not multiple of sample size"),
                self.settings.output.clone(),
            ));
        }

        let samples = unsafe {
            std::slice::from_raw_parts(data.as_ptr() as *const S, data.len() / size_of::<S>())
        };
        self.write_samples_slice(samples)
    }
}
impl AudioWriter for WavHoundWriter {
    fn write_bytes_slice(&mut self, data: &[u8]) -> Result<(), Error> {
        match self.settings.format {
            Format::U8 => self.write_bytes_as_samples::<i8>(data),
            Format::S16Le => self.write_bytes_as_samples::<i16>(data),
            Format::S32Le => self.write_bytes_as_samples::<i32>(data),
            Format::FloatLe => self.write_bytes_as_samples::<f32>(data),
        }
    }
}
