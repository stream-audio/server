use portaudio;

use channel;

use std::error::Error as StdError;
use std::io;

use audio_saver;

#[derive(Debug)]
pub struct Error {
    repr: Box<ErrorRepr>,
}

#[derive(Debug)]
pub enum ErrorRepr {
    PortAudio(portaudio::Error),
    FileError(FileError),
    IoError(IoError),
    AudioSaverError(audio_saver::Error),
    ChannelError(&'static str),
    ChannelRecv(channel::RecvError),
}

#[derive(Debug)]
pub struct FileError {
    fname: String,
    error: io::Error,
}

#[derive(Debug)]
pub struct IoError {
    context: &'static str,
    error: io::Error,
}

impl Error {
    fn new(repr: ErrorRepr) -> Self {
        Self {
            repr: Box::new(repr),
        }
    }
}

impl FileError {
    pub fn create(fname: String, error: io::Error) -> Self {
        Self { fname, error }
    }
}

impl IoError {
    pub(crate) fn new(context: &'static str, error: io::Error) -> Self {
        Self { context, error }
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self.repr {
            ErrorRepr::PortAudio(ref e) => write!(f, "PortAudio Error: {}", e.description()),
            ErrorRepr::FileError(ref e) => write!(f, "{} in file '{}'", e.error, e.fname),
            ErrorRepr::IoError(ref e) => write!(f, "{}. during {}", e.error, e.context),
            ErrorRepr::AudioSaverError(ref e) => write!(f, "{}", e),
            ErrorRepr::ChannelError(e) => write!(f, "Channel Error {}", e),
            ErrorRepr::ChannelRecv(e) => write!(f, "Channel Error {}", e),
        }
    }
}
impl ::std::error::Error for Error {}

impl From<portaudio::Error> for Error {
    fn from(pa_error: portaudio::Error) -> Self {
        Self::new(ErrorRepr::PortAudio(pa_error))
    }
}
impl From<FileError> for Error {
    fn from(fe: FileError) -> Self {
        Self::new(ErrorRepr::FileError(fe))
    }
}
impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Self::new(ErrorRepr::IoError(e))
    }
}
impl From<audio_saver::Error> for Error {
    fn from(e: audio_saver::Error) -> Self {
        Self::new(ErrorRepr::AudioSaverError(e))
    }
}
impl From<channel::RecvError> for Error {
    fn from(e: channel::RecvError) -> Self {
        Self::new(ErrorRepr::ChannelRecv(e))
    }
}
impl From<ErrorRepr> for Error {
    fn from(e: ErrorRepr) -> Self {
        Self::new(e)
    }
}
