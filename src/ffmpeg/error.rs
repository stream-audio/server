use super::{
    codec,
    ffmpeg_const::{self, av_error},
    ffmpeg_ffi, AudioSampleFormat,
};
use std::borrow::Cow;
use std::ffi::{CStr, FromBytesWithNulError};
use std::os::raw::c_char;

pub struct Error {
    pub repr: Box<ErrorRepr>,
}

#[derive(Debug)]
pub enum ErrorRepr {
    Internal(InternalError),
    NoCodec(NoCodecError),
    NoParser(NoParserError),
    UnsupportedSampleFormat(UnsupportedSampleFormatError),
    CannotAllocate(CannotAllocateError),
    BytesWithNull(std::ffi::FromBytesWithNulError),
}

#[derive(Debug)]
pub struct InternalError {
    pub err_num: i32,
    pub context: Cow<'static, str>,
}

#[derive(Debug)]
pub struct NoCodecError {
    pub codec: codec::Codec,
}

#[derive(Debug)]
pub struct NoParserError {
    pub codec: codec::Codec,
}

#[derive(Debug)]
pub struct UnsupportedSampleFormatError {
    pub fmt: AudioSampleFormat,
    pub supported: Vec<AudioSampleFormat>,
}

#[derive(Debug)]
pub struct CannotAllocateError {
    pub context: Cow<'static, str>,
}

impl Error {
    pub fn new<S: Into<Cow<'static, str>>>(err_num: i32, ctx: S) -> Self {
        Self {
            repr: Box::new(ErrorRepr::Internal(InternalError::new(err_num, ctx))),
        }
    }

    pub fn new_cannot_find_codec(codec: codec::Codec) -> Self {
        Self {
            repr: Box::new(ErrorRepr::NoCodec(NoCodecError { codec })),
        }
    }

    pub fn new_cannot_find_parser(codec: codec::Codec) -> Self {
        Self {
            repr: Box::new(ErrorRepr::NoParser(NoParserError { codec })),
        }
    }

    pub fn new_cannot_allocate<S: Into<Cow<'static, str>>>(context: S) -> Self {
        Self {
            repr: Box::new(ErrorRepr::CannotAllocate(CannotAllocateError {
                context: context.into(),
            })),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match &*self.repr {
            ErrorRepr::Internal(e) => e.fmt(f),
            ErrorRepr::NoCodec(e) => e.fmt(f),
            ErrorRepr::NoParser(e) => e.fmt(f),
            ErrorRepr::UnsupportedSampleFormat(e) => e.fmt(f),
            ErrorRepr::CannotAllocate(e) => e.fmt(f),
            ErrorRepr::BytesWithNull(e) => e.fmt(f),
        }
    }
}
impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        (self as &std::fmt::Display).fmt(f)
    }
}
impl std::error::Error for Error {}
impl From<InternalError> for Error {
    fn from(e: InternalError) -> Self {
        Self {
            repr: Box::new(ErrorRepr::Internal(e)),
        }
    }
}
impl From<UnsupportedSampleFormatError> for Error {
    fn from(e: UnsupportedSampleFormatError) -> Self {
        Self {
            repr: Box::new(ErrorRepr::UnsupportedSampleFormat(e)),
        }
    }
}
impl From<FromBytesWithNulError> for Error {
    fn from(e: FromBytesWithNulError) -> Self {
        Self {
            repr: Box::new(ErrorRepr::BytesWithNull(e)),
        }
    }
}

impl InternalError {
    pub fn new<S: Into<Cow<'static, str>>>(err_num: i32, ctx: S) -> Self {
        Self {
            err_num,
            context: ctx.into(),
        }
    }

    pub fn is_again(&self) -> bool {
        self.err_num == av_error(ffmpeg_ffi::EAGAIN)
    }

    pub fn is_eof(&self) -> bool {
        self.err_num == ffmpeg_const::AVERROR_EOF
    }

    pub fn is_again_or_eof(&self) -> bool {
        self.is_again() || self.is_eof()
    }
}

impl std::fmt::Display for InternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut err_buf = vec![0 as c_char; ffmpeg_ffi::AV_ERROR_MAX_STRING_SIZE as usize];

        let description = unsafe {
            ffmpeg_ffi::av_strerror(self.err_num, err_buf.as_mut_ptr(), err_buf.len());
            CStr::from_ptr(err_buf.as_mut_ptr())
        };
        write!(
            f,
            "Error Id: {}. Description: {}.",
            self.err_num,
            description.to_string_lossy()
        )?;
        if !self.context.is_empty() {
            write!(f, " While {}", self.context)?;
        }
        Ok(())
    }
}
impl std::error::Error for InternalError {}

impl std::fmt::Display for NoCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Cannot find codec: {:?}", self.codec)?;
        Ok(())
    }
}

impl std::fmt::Display for NoParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Cannot find parser for codec: {:?}", self.codec)?;
        Ok(())
    }
}

impl std::fmt::Display for UnsupportedSampleFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "Format {:?} is not supported. Supported: {:?}",
            self.fmt, self.supported
        )?;
        Ok(())
    }
}
impl std::error::Error for UnsupportedSampleFormatError {}

impl std::fmt::Display for CannotAllocateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "Cannot allocate {}", self.context)?;
        Ok(())
    }
}
