use std::cmp;
use std::collections::VecDeque;
use std::fmt;
use std::fs;
use std::io;
use std::mem::size_of;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use hound;

use std::thread::JoinHandle;

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

type Sample = f32;

trait SampleFormatGetter {
    fn sample_format() -> hound::SampleFormat;
}
impl SampleFormatGetter for i16 {
    fn sample_format() -> hound::SampleFormat {
        hound::SampleFormat::Int
    }
}
impl SampleFormatGetter for i32 {
    fn sample_format() -> hound::SampleFormat {
        hound::SampleFormat::Int
    }
}
impl SampleFormatGetter for f32 {
    fn sample_format() -> hound::SampleFormat {
        hound::SampleFormat::Float
    }
}

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
    fn create_writer(&self, settings: Settings) -> Result<Box<dyn AudioWriter + Send>, Error>;
}

pub trait AudioWriter {
    fn write_sample(&mut self, sample: Sample) -> Result<(), Error>;
    fn write_samples_slice(&mut self, samples: &[Sample]) -> Result<(), Error>;
    fn write_samples_vec_deq(&mut self, samples: &VecDeque<Sample>) -> Result<(), Error>;
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

impl AudioFactory for WavHoundFactory {
    fn create_writer(&self, settings: Settings) -> Result<Box<dyn AudioWriter + Send>, Error> {
        let hound_spec = hound::WavSpec {
            channels: settings.channels,
            sample_rate: settings.sample_rate as u32,
            bits_per_sample: (size_of::<Sample>() * 8) as u16,
            sample_format: Sample::sample_format(),
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

    fn write_samples_slice(&mut self, samples: &[Sample]) -> Result<(), Error> {
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

    fn write_bytes_slice(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.len() % size_of::<Sample>() != 0 {
            return Err(Error::new(
                ErrorKind::FormatError("Size of data is not multiple of sample size"),
                self.output.clone(),
            ));
        }

        let samples = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const Sample,
                data.len() / size_of::<Sample>(),
            )
        };

        self.write_samples_slice(samples)
    }
}

pub struct ThreadAudioWriter {
    que: Arc<ThreadAudioWriterQueue>,
    thread: Option<JoinHandle<()>>,
}

impl ThreadAudioWriter {
    pub fn new(writer: Box<dyn AudioWriter + Send>) -> Self {
        let que = Arc::new(ThreadAudioWriterQueue::default());
        let mut thread_self = ThreadAudioWriterData::new(que.clone(), writer);

        let thread = thread::Builder::new()
            .name("Audio Writer".to_owned())
            .spawn(move || thread_self.thread_loop())
            .expect("Error creating an audio writing thread");

        Self {
            que,
            thread: Some(thread),
        }
    }

    pub fn write_sample(&self, sample: Sample) -> Result<(), Error> {
        let mut que = self.que.que.lock().unwrap();
        if let Some(que) = que.as_mut() {
            que.push_back(sample);
            self.que.cvar.notify_one();
        }
        Ok(())
    }

    pub fn write_samples_slice(&self, samples: &[Sample]) -> Result<(), Error> {
        let mut que = self.que.que.lock().unwrap();
        if let Some(que) = que.as_mut() {
            que.extend(samples.iter());
            self.que.cvar.notify_one();
        }
        Ok(())
    }

    pub fn stop_and_join(&mut self) {
        dbg!("stop_and_join");
        *self.que.que.lock().unwrap() = None;
        self.que.cvar.notify_one();
        self.thread.take().unwrap().join().unwrap();
        dbg!("stop_and_join end");
    }
}
impl Drop for ThreadAudioWriter {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

struct ThreadAudioWriterQueue {
    que: Mutex<Option<VecDeque<Sample>>>,
    cvar: Condvar,
}
impl Default for ThreadAudioWriterQueue {
    fn default() -> Self {
        Self {
            que: Mutex::new(Some(VecDeque::with_capacity(4096))),
            cvar: Condvar::new(),
        }
    }
}

const BUFFER_SIZE: usize = 16384;
struct ThreadAudioWriterData {
    que: Arc<ThreadAudioWriterQueue>,
    writer: Box<dyn AudioWriter + Send>,
    buffer: Vec<Sample>,
}

impl ThreadAudioWriterData {
    fn new(que: Arc<ThreadAudioWriterQueue>, writer: Box<dyn AudioWriter + Send>) -> Self {
        Self {
            que,
            writer,
            buffer: Vec::with_capacity(BUFFER_SIZE),
        }
    }

    fn thread_loop(&mut self) {
        loop {
            let can_continue = self.wait_and_fill_buffer();
            if !can_continue {
                return;
            }

            let write_res = self.writer.write_samples_slice(&self.buffer);
            if let Err(err) = write_res {
                eprintln!("Error writing audio: {}", err);
                return;
            }
            self.buffer.clear();
        }
    }

    fn wait_and_fill_buffer(&mut self) -> bool {
        let mut lock = self.que.que.lock().unwrap();

        let que = loop {
            if let Some(ref mut que) = *lock {
                if !que.is_empty() {
                    break que;
                }
            } else {
                return false;
            }

            lock = self.que.cvar.wait(lock).unwrap();
        };

        let drain_len = cmp::min(que.len(), BUFFER_SIZE);
        let drained = que.drain(..drain_len);

        self.buffer.extend(drained.into_iter());

        true
    }
}
