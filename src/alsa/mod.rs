#[allow(dead_code, unused_attributes, bad_style)]
mod alsa_ffi;

use crate::audio_saver;
use crate::error::*;
use crate::ffmpeg;
use alsa_ffi::{snd_pcm_sframes_t, snd_pcm_uframes_t};
use libc::{c_int, c_uint, c_void};
use std::borrow::Cow;
use std::cmp;
use std::ffi;
use std::ptr;

#[derive(Debug)]
pub struct AlsaError {
    errnum: i32,
    description: String,
    context: Cow<'static, str>,
}

macro_rules! try_snd {
    ($e:expr) => {{
        let err = $e;
        if err < 0 {
            return Err(AlsaError::new(err as i32, format!("calling {}", stringify!($e))).into());
        }
        err
    }};
    ($e:expr, $ctx:expr) => {{
        let err = $e;
        if err < 0 {
            return Err(AlsaError::new(err as i32, $ctx).into());
        }
        err
    }};
}

impl AlsaError {
    fn new<S: Into<Cow<'static, str>>>(errnum: i32, context: S) -> Self {
        Self {
            errnum,
            description: snd_strerror(errnum),
            context: context.into(),
        }
    }
}

impl std::fmt::Display for AlsaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "Error Id:{}. Description: {}",
            self.errnum, self.description
        )?;
        if !self.context.is_empty() {
            write!(f, ". In context: {}", self.context)?;
        }
        Ok(())
    }
}
impl std::error::Error for AlsaError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub format: Format,
    pub channels: u32,
    pub rate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Format {
    U8,
    S16Le,
    FloatLe,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stream {
    Playback,
    Capture,
}

pub struct SndPcm {
    raw_ptr: *mut alsa_ffi::snd_pcm_t,
    stream: Stream,
    params: Params,
    buffer_size: snd_pcm_uframes_t,
}

impl Into<ffmpeg::AudioParams> for Params {
    fn into(self) -> ffmpeg::AudioParams {
        ffmpeg::AudioParams {
            rate: self.rate as _,
            format: self.format.into(),
        }
    }
}
impl Into<ffmpeg::AudioSampleFormat> for Format {
    fn into(self) -> ffmpeg::AudioSampleFormat {
        match self {
            Format::U8 => ffmpeg::AudioSampleFormat::U8,
            Format::S16Le => ffmpeg::AudioSampleFormat::S16Le,
            Format::FloatLe => ffmpeg::AudioSampleFormat::FloatLe,
        }
    }
}

impl SndPcm {
    pub fn open(name: String, stream: Stream, params: Params) -> Result<Self, Error> {
        unsafe {
            let mut res: *mut alsa_ffi::snd_pcm_t = ptr::null_mut();

            let name = ffi::CString::new(name)?;

            try_snd!(alsa_ffi::snd_pcm_open(
                &mut res,
                name.as_ptr(),
                stream.to_ffi(),
                0
            ));

            let mut res = Self {
                raw_ptr: res,
                stream,
                params,
                buffer_size: 0,
            };

            let mut hw_params = SndPcmHwParams::new()?;
            hw_params.set_any(&res)?;
            hw_params.set_interleaved(&res)?;
            res.params.format = hw_params.set_format(&res, params.format)?;
            hw_params.set_channels(&res, params.channels)?;
            res.params.rate = hw_params.set_rate_near(&res, params.rate)?;

            let buffer_time = cmp::min(hw_params.get_buffer_time_max()?, 500000);
            let period_time = buffer_time / 4;

            hw_params.set_period_time_near(&res, period_time)?;
            hw_params.set_buffer_time_near(&res, buffer_time)?;

            res.set_hw_params(&hw_params)?;

            let mut sw_params = SndPcmSwParams::new()?;
            sw_params.set_current(&res)?;
            sw_params.set_start_threshold(&res, res.calc_start_threshold(&hw_params)?)?;

            res.set_sw_params(&sw_params)?;

            res.buffer_size = hw_params.get_period_size_in_frames()?;
            dbg!(res.buffer_size);

            Ok(res)
        }
    }

    pub fn info(&self) -> Result<SndPcmInfo, Error> {
        let res = SndPcmInfo::new()?;
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_info(self.raw_ptr, res.raw_ptr));
        }
        Ok(res)
    }

    pub fn read_interleaved(&self, buffer: &mut [u8]) -> Result<usize, Error> {
        let total_bytes = self.frames_qty_to_bytes_qty(
            self.bytes_qty_to_frames_qty(buffer.len()) as snd_pcm_sframes_t
        );
        let mut bytes_to_read = total_bytes;

        while bytes_to_read > 0 {
            let frames_to_read = self.bytes_qty_to_frames_qty(bytes_to_read);

            let total_bytes_read = total_bytes - bytes_to_read;
            let empty_buffer_part = &mut buffer[total_bytes_read..];

            let res = unsafe {
                alsa_ffi::snd_pcm_readi(
                    self.raw_ptr,
                    empty_buffer_part.as_mut_ptr() as *mut c_void,
                    frames_to_read,
                )
            };
            if res == -alsa_ffi::EAGAIN as snd_pcm_sframes_t {
                self.wait_for_data(100)?;
                continue;
            }
            if res < 0 {
                dbg!(res);
            }
            try_snd!(res);

            let bytes_read = self.frames_qty_to_bytes_qty(res);

            bytes_to_read -= bytes_read;

            if bytes_to_read > 0 {
                self.wait_for_data(100)?;
            }
        }

        Ok(total_bytes)
    }

    pub fn write_interleaved(&self, buffer: &[u8]) -> Result<(), Error> {
        let total_bytes = self.frames_qty_to_bytes_qty(
            self.bytes_qty_to_frames_qty(buffer.len()) as snd_pcm_sframes_t
        );
        let mut bytes_to_write = total_bytes;

        let buf_size = self.frames_qty_to_bytes_qty(self.buffer_size as snd_pcm_sframes_t);

        while bytes_to_write > 0 {
            let total_bytes_written = total_bytes - bytes_to_write;

            let buf_size = cmp::min(buf_size, bytes_to_write);

            let sub_buffer = &buffer[total_bytes_written..total_bytes_written + buf_size];
            self.write_i(sub_buffer)?;
            bytes_to_write -= buf_size;
        }

        Ok(())
    }

    /// Stops a PCM preserving pending frames.
    pub fn stop(&self) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_drain(self.raw_ptr));
        }
        Ok(())
    }

    pub fn dump_settings(&self) -> Result<String, Error> {
        let buffer = SndOutput::new()?;
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_dump(self.raw_ptr, buffer.raw_ptr));
        }

        buffer.get_string()
    }

    pub fn get_params(&self) -> Params {
        self.params
    }

    fn write_i(&self, buffer: &[u8]) -> Result<(), Error> {
        let mut bytes_to_write = buffer.len();

        while bytes_to_write > 0 {
            let frames_to_write = self.bytes_qty_to_frames_qty(bytes_to_write);

            let total_bytes_written = buffer.len() - bytes_to_write;
            let buffer_part_to_write = &buffer[total_bytes_written..];

            let res = unsafe {
                alsa_ffi::snd_pcm_writei(
                    self.raw_ptr,
                    buffer_part_to_write.as_ptr() as *mut c_void,
                    frames_to_write,
                )
            };
            if res == -alsa_ffi::EAGAIN as snd_pcm_sframes_t {
                self.wait_for_data(100)?;
                continue;
            } else if res == -alsa_ffi::EPIPE as snd_pcm_sframes_t {
                self.xrun(res as c_int)?;
                //self.wait_for_data(100)?;
                continue;
            }

            let bytes_written = self.frames_qty_to_bytes_qty(res);

            bytes_to_write -= bytes_written;

            if bytes_to_write > 0 {
                self.wait_for_data(100)?;
            }
        }

        Ok(())
    }

    fn set_hw_params(&self, hw_params: &SndPcmHwParams) -> Result<(), Error> {
        unsafe {
            try_snd!(
                alsa_ffi::snd_pcm_hw_params(self.raw_ptr, hw_params.raw_ptr),
                "Setting hardware parameters"
            );
        }
        Ok(())
    }

    fn set_sw_params(&self, sw_params: &SndPcmSwParams) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_sw_params(self.raw_ptr, sw_params.raw_ptr));
        }
        Ok(())
    }

    fn bytes_per_frame(&self) -> usize {
        self.params.format.bytes_per_sample() * self.params.channels as usize
    }

    fn bytes_qty_to_frames_qty(&self, bytes_qty: usize) -> alsa_ffi::snd_pcm_uframes_t {
        (bytes_qty / self.bytes_per_frame()) as alsa_ffi::snd_pcm_uframes_t
    }

    fn frames_qty_to_bytes_qty(&self, frames_qty: snd_pcm_sframes_t) -> usize {
        (frames_qty as usize * self.bytes_per_frame())
    }

    fn wait_for_data(&self, timeout: i32) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_wait(self.raw_ptr, timeout as c_int));
        }

        Ok(())
    }

    fn xrun(&self, errnum: c_int) -> Result<(), Error> {
        let pcm_status = SndPcmStatus::new(self)?;
        if pcm_status.get_state() == PcmState::XRun {
            eprintln!("Underrun or Overrun are detected");
            self.prepare()?;
            Ok(())
        } else {
            Err(AlsaError::new(errnum as i32, "Writing samples").into())
        }
    }

    fn prepare(&self) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_prepare(self.raw_ptr));
        }

        Ok(())
    }

    fn calc_start_threshold(&self, hw_params: &SndPcmHwParams) -> Result<snd_pcm_uframes_t, Error> {
        match self.stream {
            Stream::Playback => {
                let buffer_size = hw_params.get_buffer_size_in_frames()?;
                let start_threshold = buffer_size as f64 + self.params.rate as f64 / 1000000.;
                Ok(start_threshold as snd_pcm_uframes_t)
            }
            Stream::Capture => Ok(1),
        }
    }
}
impl Drop for SndPcm {
    fn drop(&mut self) {
        unsafe {
            if !self.raw_ptr.is_null() {
                alsa_ffi::snd_pcm_drain(self.raw_ptr);
                alsa_ffi::snd_pcm_close(self.raw_ptr);
            }
        }
    }
}
unsafe impl Send for SndPcm {}

impl Stream {
    fn to_ffi(&self) -> alsa_ffi::snd_pcm_stream_t {
        match self {
            Stream::Playback => alsa_ffi::SND_PCM_STREAM_PLAYBACK,
            Stream::Capture => alsa_ffi::SND_PCM_STREAM_CAPTURE,
        }
    }
}

impl Format {
    fn to_ffi(&self) -> alsa_ffi::snd_pcm_format_t {
        match self {
            Format::U8 => alsa_ffi::SND_PCM_FORMAT_U8,
            Format::S16Le => alsa_ffi::SND_PCM_FORMAT_S16_LE,
            Format::FloatLe => alsa_ffi::SND_PCM_FORMAT_FLOAT_LE,
        }
    }

    fn bytes_per_sample(&self) -> usize {
        match self {
            Format::U8 => 1,
            Format::S16Le => 2,
            Format::FloatLe => 4,
        }
    }

    pub fn to_audio_saver_format(&self) -> audio_saver::Format {
        match self {
            Format::U8 => audio_saver::Format::U8,
            Format::S16Le => audio_saver::Format::S16Le,
            Format::FloatLe => audio_saver::Format::FloatLe,
        }
    }
}

pub struct SndCtl {
    raw_ptr: *mut alsa_ffi::snd_ctl_t,
    card: i32,
}
impl SndCtl {
    pub fn open(card: i32) -> Result<Self, Error> {
        unsafe {
            let mut raw_ptr: *mut alsa_ffi::snd_ctl_t = ptr::null_mut();

            let name = format!("hw:{}", card);
            let name = ffi::CString::new(name)?;

            try_snd!(alsa_ffi::snd_ctl_open(
                &mut raw_ptr,
                name.as_ptr(),
                0 as c_int
            ));

            Ok(Self { raw_ptr, card })
        }
    }

    pub fn list_cards() -> SndCtlIterator {
        SndCtlIterator { card: 0 }
    }

    pub fn get_card_num(&self) -> i32 {
        self.card
    }

    pub fn card_info(&self) -> Result<SndCtlCardInfo, Error> {
        let info = SndCtlCardInfo::new()?;
        unsafe {
            try_snd!(alsa_ffi::snd_ctl_card_info(self.raw_ptr, info.raw_ptr));
        }
        Ok(info)
    }

    pub fn list_devices_info(&self) -> SndDeviceInfoIterator {
        SndDeviceInfoIterator {
            dev: -1,
            raw_ctl: self.raw_ptr,
        }
    }
}
impl Drop for SndCtl {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_ctl_close(self.raw_ptr);
        }
    }
}

pub struct SndCtlIterator {
    card: c_int,
}
impl Iterator for SndCtlIterator {
    type Item = Result<SndCtl, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let err = unsafe { alsa_ffi::snd_card_next(&mut self.card) };
        if err < 0 || self.card < 0 {
            return None;
        }

        Some(SndCtl::open(self.card))
    }
}

pub struct SndDeviceInfoIterator {
    dev: c_int,
    raw_ctl: *mut alsa_ffi::snd_ctl_t,
}
impl Iterator for SndDeviceInfoIterator {
    type Item = Result<SndPcmInfo, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let err = unsafe { alsa_ffi::snd_ctl_pcm_next_device(self.raw_ctl, &mut self.dev) };
            if err < 0 || self.dev < 0 {
                return None;
            }

            let res = SndPcmInfo::open_device(self.dev, self.raw_ctl);

            if let Err(ref err) = res {
                if let ErrorRepr::Alsa(err) = err.get_repr() {
                    if err.errnum == -2 {
                        continue;
                    }
                }
            }

            return Some(res);
        }
    }
}

pub struct SndPcmInfo {
    raw_ptr: *mut alsa_ffi::snd_pcm_info_t,
}
impl SndPcmInfo {
    fn new() -> Result<Self, Error> {
        let mut res: *mut alsa_ffi::snd_pcm_info_t = ptr::null_mut();

        unsafe {
            try_snd!(alsa_ffi::snd_pcm_info_malloc(&mut res));
        }
        Ok(Self { raw_ptr: res })
    }

    fn open_device(dev: c_int, raw_ctl: *mut alsa_ffi::snd_ctl_t) -> Result<Self, Error> {
        unsafe {
            let res = Self::new()?;

            alsa_ffi::snd_pcm_info_set_device(res.raw_ptr, dev as c_uint);
            alsa_ffi::snd_pcm_info_set_subdevice(res.raw_ptr, 0);
            alsa_ffi::snd_pcm_info_set_stream(res.raw_ptr, alsa_ffi::SND_PCM_STREAM_CAPTURE);

            try_snd!(alsa_ffi::snd_ctl_pcm_info(raw_ctl, res.raw_ptr));
            Ok(res)
        }
    }

    pub fn get_dev_num(&self) -> i32 {
        unsafe { alsa_ffi::snd_pcm_info_get_device(self.raw_ptr) as i32 }
    }

    pub fn get_id(&self) -> &'static str {
        unsafe {
            let res = alsa_ffi::snd_pcm_info_get_id(self.raw_ptr);
            ffi::CStr::from_ptr(res)
                .to_str()
                .expect("Wrong UTF8 sequence")
        }
    }

    pub fn get_name(&self) -> &'static str {
        unsafe {
            let res = alsa_ffi::snd_pcm_info_get_name(self.raw_ptr);
            ffi::CStr::from_ptr(res)
                .to_str()
                .expect("Wrong UTF8 sequence")
        }
    }

    pub fn get_subdevice_count(&self) -> usize {
        unsafe { alsa_ffi::snd_pcm_info_get_subdevices_count(self.raw_ptr) as usize }
    }
}
impl Drop for SndPcmInfo {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_pcm_info_free(self.raw_ptr);
        }
    }
}

pub struct SndCtlCardInfo {
    raw_ptr: *mut alsa_ffi::snd_ctl_card_info_t,
}
impl SndCtlCardInfo {
    fn new() -> Result<Self, Error> {
        unsafe {
            let mut raw_ptr: *mut alsa_ffi::snd_ctl_card_info_t = std::ptr::null_mut();
            try_snd!(alsa_ffi::snd_ctl_card_info_malloc(&mut raw_ptr));

            Ok(Self { raw_ptr })
        }
    }

    pub fn get_id(&self) -> &'static str {
        unsafe {
            let id = alsa_ffi::snd_ctl_card_info_get_id(self.raw_ptr);
            ffi::CStr::from_ptr(id)
                .to_str()
                .expect("Wrong UTF8 sequence")
        }
    }

    pub fn get_name(&self) -> &'static str {
        unsafe {
            let name = alsa_ffi::snd_ctl_card_info_get_name(self.raw_ptr);
            ffi::CStr::from_ptr(name)
                .to_str()
                .expect("Wrong UTF8 sequence")
        }
    }
}
impl Drop for SndCtlCardInfo {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_ctl_card_info_free(self.raw_ptr);
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
enum PcmState {
    Open,
    Setup,
    Prepared,
    Running,
    XRun,
    Draining,
    Paused,
    Suspended,
    Disconnected,
}

struct SndPcmStatus {
    raw_ptr: *mut alsa_ffi::snd_pcm_status_t,
}
impl SndPcmStatus {
    fn new(pcm: &SndPcm) -> Result<Self, Error> {
        unsafe {
            let mut res = ptr::null_mut();

            try_snd!(alsa_ffi::snd_pcm_status_malloc(&mut res));
            let res = Self { raw_ptr: res };

            try_snd!(alsa_ffi::snd_pcm_status(pcm.raw_ptr, res.raw_ptr));

            Ok(res)
        }
    }

    fn get_state(&self) -> PcmState {
        unsafe {
            let raw_res = alsa_ffi::snd_pcm_status_get_state(self.raw_ptr);
            match raw_res {
                alsa_ffi::SND_PCM_STATE_OPEN => PcmState::Open,
                alsa_ffi::SND_PCM_STATE_SETUP => PcmState::Setup,
                alsa_ffi::SND_PCM_STATE_PREPARED => PcmState::Prepared,
                alsa_ffi::SND_PCM_STATE_RUNNING => PcmState::Running,
                alsa_ffi::SND_PCM_STATE_XRUN => PcmState::XRun,
                alsa_ffi::SND_PCM_STATE_DRAINING => PcmState::Draining,
                alsa_ffi::SND_PCM_STATE_PAUSED => PcmState::Paused,
                alsa_ffi::SND_PCM_STATE_SUSPENDED => PcmState::Suspended,
                alsa_ffi::SND_PCM_STATE_DISCONNECTED => PcmState::Disconnected,
                _ => unreachable!(),
            }
        }
    }
}
impl Drop for SndPcmStatus {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_pcm_status_free(self.raw_ptr);
        }
    }
}

struct SndPcmHwParams {
    raw_ptr: *mut alsa_ffi::snd_pcm_hw_params_t,
}

impl SndPcmHwParams {
    fn new() -> Result<Self, Error> {
        let mut raw_ptr = ptr::null_mut();
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_malloc(&mut raw_ptr));
        }

        Ok(Self { raw_ptr })
    }

    fn set_any(&mut self, pcm: &SndPcm) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_any(pcm.raw_ptr, self.raw_ptr));
        }
        Ok(())
    }

    fn set_interleaved(&mut self, pcm: &SndPcm) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_set_access(
                pcm.raw_ptr,
                self.raw_ptr,
                alsa_ffi::SND_PCM_ACCESS_RW_INTERLEAVED,
            ));
        }
        Ok(())
    }

    fn set_format(&mut self, pcm: &SndPcm, format: Format) -> Result<Format, Error> {
        let available = self.get_available_formats(pcm);
        let format = if available.contains(&format) {
            format
        } else if let Some(format) = available.first() {
            *format
        } else {
            return Err(AlsaError::new(
                -22,
                format!(
                    "Setting audio format to {:?}. No formats are available.",
                    format
                ),
            )
            .into());
        };

        unsafe {
            try_snd!(
                alsa_ffi::snd_pcm_hw_params_set_format(pcm.raw_ptr, self.raw_ptr, format.to_ffi(),),
                format!(
                    "Setting audio format to {:?}, although it should be available. \
                     Available formats: {:?}",
                    format, available
                )
            );
        }
        Ok(format)
    }

    fn set_channels(&mut self, pcm: &SndPcm, channels: u32) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_set_channels(
                pcm.raw_ptr,
                self.raw_ptr,
                channels as c_uint,
            ));
        }
        Ok(())
    }

    fn set_rate_near(&mut self, pcm: &SndPcm, mut rate: u32) -> Result<u32, Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_set_rate_near(
                pcm.raw_ptr,
                self.raw_ptr,
                &mut rate,
                ptr::null_mut(),
            ));
        }
        Ok(rate)
    }

    fn set_period_time_near(&self, pcm: &SndPcm, mut period_time: u32) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_set_period_time_near(
                pcm.raw_ptr,
                self.raw_ptr,
                &mut period_time,
                ptr::null_mut(),
            ));
        }

        Ok(())
    }

    fn set_buffer_time_near(&self, pcm: &SndPcm, mut buffer_time: u32) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_set_buffer_time_near(
                pcm.raw_ptr,
                self.raw_ptr,
                &mut buffer_time,
                ptr::null_mut(),
            ));
        }

        Ok(())
    }

    fn get_buffer_time_max(&self) -> Result<u32, Error> {
        unsafe {
            let mut res = 0;
            try_snd!(alsa_ffi::snd_pcm_hw_params_get_buffer_time_max(
                self.raw_ptr,
                &mut res,
                ptr::null_mut(),
            ));
            Ok(res as u32)
        }
    }

    fn get_period_size_in_frames(&self) -> Result<snd_pcm_uframes_t, Error> {
        let mut res: snd_pcm_uframes_t = 0;

        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_get_period_size(
                self.raw_ptr,
                &mut res,
                ptr::null_mut()
            ));
        }

        Ok(res)
    }

    fn get_buffer_size_in_frames(&self) -> Result<snd_pcm_uframes_t, Error> {
        let mut res: snd_pcm_uframes_t = 0;
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_hw_params_get_buffer_size(
                self.raw_ptr,
                &mut res
            ));
        }

        Ok(res)
    }

    fn get_available_formats(&self, pcm: &SndPcm) -> Vec<Format> {
        let all_formats = [Format::S16Le, Format::FloatLe, Format::U8];

        let mut res = Vec::new();
        for format in &all_formats {
            let test_res = unsafe {
                alsa_ffi::snd_pcm_hw_params_test_format(pcm.raw_ptr, self.raw_ptr, format.to_ffi())
            };

            if test_res == 0 {
                res.push(*format)
            }
        }

        res
    }
}
impl Drop for SndPcmHwParams {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_pcm_hw_params_free(self.raw_ptr);
        }
    }
}

struct SndPcmSwParams {
    raw_ptr: *mut alsa_ffi::snd_pcm_sw_params_t,
}
impl SndPcmSwParams {
    fn new() -> Result<SndPcmSwParams, Error> {
        unsafe {
            let mut raw_ptr = ptr::null_mut();
            try_snd!(alsa_ffi::snd_pcm_sw_params_malloc(&mut raw_ptr));
            Ok(Self { raw_ptr })
        }
    }

    fn set_current(&mut self, pcm: &SndPcm) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_sw_params_current(
                pcm.raw_ptr,
                self.raw_ptr
            ));
        }

        Ok(())
    }

    fn set_start_threshold(
        &mut self,
        pcm: &SndPcm,
        start_threshold: snd_pcm_uframes_t,
    ) -> Result<(), Error> {
        unsafe {
            try_snd!(alsa_ffi::snd_pcm_sw_params_set_start_threshold(
                pcm.raw_ptr,
                self.raw_ptr,
                start_threshold
            ));
        }
        Ok(())
    }
}
impl Drop for SndPcmSwParams {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_pcm_sw_params_free(self.raw_ptr);
        }
    }
}

struct SndOutput {
    raw_ptr: *mut alsa_ffi::snd_output_t,
}
impl SndOutput {
    fn new() -> Result<Self, Error> {
        let mut raw_ptr = ptr::null_mut();
        unsafe {
            try_snd!(alsa_ffi::snd_output_buffer_open(&mut raw_ptr));
        }

        Ok(Self { raw_ptr })
    }

    fn get_string(&self) -> Result<String, Error> {
        unsafe {
            let mut raw_str = ptr::null_mut();
            let size = alsa_ffi::snd_output_buffer_string(self.raw_ptr, &mut raw_str);

            let bytes = std::slice::from_raw_parts(raw_str as *const u8, size).to_vec();

            Ok(String::from_utf8(bytes)?)
        }
    }
}
impl Drop for SndOutput {
    fn drop(&mut self) {
        unsafe {
            alsa_ffi::snd_output_close(self.raw_ptr);
        }
    }
}

fn snd_strerror(errnum: c_int) -> String {
    unsafe {
        let res = alsa_ffi::snd_strerror(errnum);
        ffi::CStr::from_ptr(res)
            .to_str()
            .expect("Wrong UTF8 sequence")
            .to_owned()
    }
}
