use crate::alsa;
use crate::audio_saver;
use crate::channel;
use std::borrow::Cow;
use std::io;

#[derive(Debug)]
pub struct Error {
    repr: Box<ErrorRepr>,
}

#[derive(Debug)]
pub enum ErrorRepr {
    FileError(FileError),
    IoError(IoError),
    FromUtf8(std::string::FromUtf8Error),
    Utf8(std::str::Utf8Error),
    AudioSaverError(audio_saver::Error),
    ChannelError(&'static str),
    ChannelRecv(channel::RecvError),
    Alsa(alsa::AlsaError),
    Nul(std::ffi::NulError),
}

#[derive(Debug)]
pub struct FileError {
    fname: String,
    error: io::Error,
}

#[derive(Debug)]
pub struct IoError {
    context: Cow<'static, str>,
    error: io::Error,
}

impl Error {
    fn new(repr: ErrorRepr) -> Self {
        Self {
            repr: Box::new(repr),
        }
    }
    pub fn get_repr(&self) -> &ErrorRepr {
        &self.repr
    }
}

impl FileError {
    pub fn create(fname: String, error: io::Error) -> Self {
        Self { fname, error }
    }
}

impl IoError {
    pub(crate) fn new<S: Into<Cow<'static, str>>>(context: S, error: io::Error) -> Self {
        Self {
            context: context.into(),
            error,
        }
    }
}

impl ::std::fmt::Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self.repr {
            ErrorRepr::FileError(ref e) => write!(f, "{} in file '{}'", e.error, e.fname),
            ErrorRepr::IoError(ref e) => write!(f, "{}. During {}", e.error, e.context),
            ErrorRepr::FromUtf8(ref e) => write!(f, "{}", e),
            ErrorRepr::Utf8(e) => write!(f, "From UTF8 conversion error {}", e),
            ErrorRepr::AudioSaverError(ref e) => write!(f, "{}", e),
            ErrorRepr::ChannelError(e) => write!(f, "Channel Error {}", e),
            ErrorRepr::ChannelRecv(e) => write!(f, "Channel Error {}", e),
            ErrorRepr::Alsa(ref e) => write!(f, "Alsa Error {}", e),
            ErrorRepr::Nul(ref e) => write!(f, "There is null byte in the string. {}", e),
        }
    }
}
impl ::std::error::Error for Error {}

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
impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::new(ErrorRepr::FromUtf8(e))
    }
}
impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Self::new(ErrorRepr::Utf8(e))
    }
}
impl From<alsa::AlsaError> for Error {
    fn from(e: alsa::AlsaError) -> Self {
        Self::new(ErrorRepr::Alsa(e))
    }
}
impl From<std::ffi::NulError> for Error {
    fn from(e: std::ffi::NulError) -> Self {
        Self::new(ErrorRepr::Nul(e))
    }
}
impl From<ErrorRepr> for Error {
    fn from(e: ErrorRepr) -> Self {
        Self::new(e)
    }
}
