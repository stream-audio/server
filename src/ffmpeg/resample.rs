use super::{ffmpeg_ffi, AudioParams, Error};
use libc::uint8_t;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;
use std::slice;

pub struct Resampler {
    swr_ctx: SwrContext,

    src_params: AudioParams,
    dst_params: AudioParams,

    src_samples_qty: usize,
    src_data: *mut *mut uint8_t,
    src_line_size: i32,

    dst_samples_qty: usize,
    dst_data: *mut *mut uint8_t,
    dst_line_size: i32,
}

impl Resampler {
    pub fn new(src_params: AudioParams, dst_params: AudioParams) -> Result<Self, Error> {
        Ok(Self {
            swr_ctx: SwrContext::new(src_params, dst_params)?,
            src_params,
            dst_params,
            src_samples_qty: 0,
            src_data: ptr::null_mut(),
            src_line_size: 0,
            dst_samples_qty: 0,
            dst_data: ptr::null_mut(),
            dst_line_size: 0,
        })
    }

    pub fn resample(&mut self, src: &[u8]) -> Result<&[u8], Error> {
        let in_samples_qty = self.src_params.get_samples_qty_in_buffer(src.len());
        self.realloc_buffers_if_required(in_samples_qty)?;

        unsafe {
            let ffmpeg_buf = slice::from_raw_parts_mut(*self.src_data, src.len());
            ffmpeg_buf.copy_from_slice(src);
        }

        let ret_samples = try_ffmpeg!(
            unsafe {
                ffmpeg_ffi::swr_convert(
                    self.swr_ctx.raw_ptr,
                    self.dst_data,
                    self.dst_samples_qty as i32,
                    self.src_data as *mut *const u8,
                    in_samples_qty as i32,
                )
            },
            "resampling"
        );

        let dst_buf_size = try_ffmpeg!(
            unsafe {
                ffmpeg_ffi::av_samples_get_buffer_size(
                    &mut self.dst_line_size,
                    self.dst_params.get_channels_qty(),
                    ret_samples,
                    self.dst_params.format.to_raw(),
                    1,
                )
            },
            "calculating destination buffer size"
        ) as usize;

        let res = unsafe { slice::from_raw_parts(*self.dst_data, dst_buf_size) };

        Ok(res)
    }

    fn realloc_buffers_if_required(&mut self, required_samples_qty: usize) -> Result<(), Error> {
        if self.src_samples_qty >= required_samples_qty {
            return Ok(());
        }

        self.dealloc_buffers();
        self.src_samples_qty = required_samples_qty;

        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::av_samples_alloc_array_and_samples(
                    &mut self.src_data,
                    &mut self.src_line_size,
                    self.src_params.get_channels_qty(),
                    self.src_samples_qty as _,
                    self.src_params.format.to_raw(),
                    0,
                ),
                "allocating source samples"
            );
        }

        self.dst_samples_qty = unsafe {
            ffmpeg_ffi::av_rescale_rnd(
                self.src_samples_qty as i64,
                self.dst_params.rate as i64,
                self.src_params.rate as i64,
                ffmpeg_ffi::AVRounding_AV_ROUND_UP,
            ) as usize
        };

        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::av_samples_alloc_array_and_samples(
                    &mut self.dst_data,
                    &mut self.dst_line_size,
                    self.dst_params.get_channels_qty(),
                    self.dst_samples_qty as _,
                    self.dst_params.format.to_raw(),
                    0,
                ),
                "allocating destination samples"
            );
        }

        Ok(())
    }

    fn dealloc_buffers(&mut self) {
        if !self.src_data.is_null() {
            unsafe {
                ffmpeg_ffi::av_free(*self.src_data as *mut c_void);
            }
        }
        unsafe {
            ffmpeg_ffi::av_free(self.src_data as *mut c_void);
        }
        if !self.dst_data.is_null() {
            unsafe {
                ffmpeg_ffi::av_free(*self.dst_data as *mut c_void);
            }
        }
        unsafe {
            ffmpeg_ffi::av_free(self.dst_data as *mut c_void);
        }

        self.src_data = ptr::null_mut();
        self.dst_data = ptr::null_mut();
    }
}
impl Drop for Resampler {
    fn drop(&mut self) {
        self.dealloc_buffers();
    }
}
unsafe impl Send for Resampler {}

pub const LAYOUT: u32 = ffmpeg_ffi::AV_CH_LAYOUT_STEREO;

struct SwrContext {
    raw_ptr: *mut ffmpeg_ffi::SwrContext,
}

impl SwrContext {
    fn new(in_params: AudioParams, out_params: AudioParams) -> Result<Self, Error> {
        let raw_ptr = unsafe { ffmpeg_ffi::swr_alloc() };

        let res = Self { raw_ptr };

        res.set_opt(LAYOUT as i64, b"in_channel_layout\0")?;
        res.set_opt(in_params.rate as i64, b"in_sample_rate\0")?;
        res.set_opt(in_params.format.to_raw() as i64, b"in_sample_fmt\0")?;

        res.set_opt(LAYOUT as i64, b"out_channel_layout\0")?;
        res.set_opt(out_params.rate as i64, b"out_sample_rate\0")?;
        res.set_opt(out_params.format.to_raw() as i64, b"out_sample_fmt\0")?;

        res.init()?;

        Ok(res)
    }

    fn init(&self) -> Result<(), Error> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::swr_init(self.raw_ptr),
                "initialize the resampling context"
            )
        };
        Ok(())
    }

    fn set_opt(&self, value: i64, name: &[u8]) -> Result<(), Error> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::av_opt_set_int(
                    self.void_ptr(),
                    name.as_ptr() as *const c_char,
                    value,
                    0,
                ),
                format!(
                    "Setting {}",
                    CStr::from_bytes_with_nul(name)?.to_string_lossy()
                )
            );
        }
        Ok(())
    }

    fn void_ptr(&self) -> *mut c_void {
        self.raw_ptr as _
    }
}

impl Drop for SwrContext {
    fn drop(&mut self) {
        if !self.raw_ptr.is_null() {
            unsafe {
                ffmpeg_ffi::swr_free(&mut self.raw_ptr);
            }
        }
    }
}
unsafe impl Send for SwrContext {}
