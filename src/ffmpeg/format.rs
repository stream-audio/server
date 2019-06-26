use super::{codec, ffmpeg_const, ffmpeg_ffi, Error};
use libc::{c_int, c_void, uint8_t};
use std::ffi::CString;
use std::ptr;
use std::slice;

pub struct SinglePacketFormatter {
    ctx: AvFormatContext,
    buf: Box<Vec<u8>>,
}

impl SinglePacketFormatter {
    pub fn new(codec_ctx: &codec::AvCodecContext, codec: codec::Codec) -> Result<Self, Error> {
        let fmt_ctx = AvFormatContext::new(codec_ctx, codec)?;

        let mut res = Self {
            ctx: fmt_ctx,
            buf: Box::new(Vec::new()),
        };
        res.assign_callbacks()?;

        Ok(res)
    }

    pub fn write_pkt(&mut self, pkt: &codec::AvPacket) -> Result<(), Error> {
        self.buf.clear();
        self.ctx.write_pkt(pkt)
    }

    pub fn get_buf(&self) -> &[u8] {
        &*self.buf
    }

    fn assign_callbacks(&mut self) -> Result<(), Error> {
        unsafe {
            unsafe extern "C" fn read(_: *mut c_void, _: *mut uint8_t, _: c_int) -> c_int {
                ffmpeg_const::av_error(ffmpeg_ffi::EACCES)
            }

            unsafe extern "C" fn write(
                opaque: *mut c_void,
                buf: *mut uint8_t,
                buf_size: c_int,
            ) -> c_int {
                let to_buf = opaque as *mut Vec<u8>;
                let to_buf = &mut *to_buf;

                let form_buf = slice::from_raw_parts(buf, buf_size as usize);
                to_buf.extend_from_slice(form_buf);

                buf_size
            }

            let buf_ptr = &mut *self.buf;

            let buf_size = 4 * 1024;
            let buffer = ffmpeg_ffi::av_mallocz(buf_size) as *mut u8;

            let avio_ctx = ffmpeg_ffi::avio_alloc_context(
                buffer,
                buf_size as i32,
                1,
                (buf_ptr as *mut Vec<u8>) as *mut c_void,
                Some(read),
                Some(write),
                None,
            );
            if avio_ctx.is_null() {
                return Err(Error::new_cannot_allocate("AVIO context"));
            }

            (*avio_ctx).direct = 1;

            (*self.ctx.raw_ptr).pb = avio_ctx;

            try_ffmpeg!(
                ffmpeg_ffi::avformat_write_header(self.ctx.raw_ptr, ptr::null_mut()),
                "writing header and initializing the formatter"
            );
        }

        Ok(())
    }
}

struct AvFormatContext {
    raw_ptr: *mut ffmpeg_ffi::AVFormatContext,
}

impl AvFormatContext {
    fn new(codec_ctx: &codec::AvCodecContext, codec: codec::Codec) -> Result<Self, Error> {
        unsafe {
            let muxer_name = CString::new(codec.get_default_muxer_name()).unwrap();

            let mut raw_ptr = ptr::null_mut();
            try_ffmpeg!(
                ffmpeg_ffi::avformat_alloc_output_context2(
                    &mut raw_ptr,
                    ptr::null_mut(),
                    muxer_name.as_ptr(),
                    ptr::null_mut(),
                ),
                "allocating format output context"
            );
            assert!(!raw_ptr.is_null());

            let res = Self { raw_ptr };

            res.add_audio_stream(codec_ctx)?;

            Ok(res)
        }
    }

    fn add_audio_stream(&self, codec_ctx: &codec::AvCodecContext) -> Result<(), Error> {
        unsafe {
            let stream = ffmpeg_ffi::avformat_new_stream(self.raw_ptr, ptr::null());

            if stream.is_null() {
                return Err(Error::new_cannot_allocate(
                    "cannot allocate stream for the formatter",
                ));
            }

            (*stream).id = ((*self.raw_ptr).nb_streams - 1) as c_int;

            try_ffmpeg!(
                ffmpeg_ffi::avcodec_parameters_from_context((*stream).codecpar, codec_ctx.raw_ptr),
                "setting stream parameters from the codec context"
            );

            Ok(())
        }
    }

    fn write_pkt(&mut self, pkt: &codec::AvPacket) -> Result<(), Error> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::av_write_frame(self.raw_ptr, pkt.raw_ptr),
                "writing packet into the formatter"
            );
        }

        Ok(())
    }
}
impl Drop for AvFormatContext {
    fn drop(&mut self) {
        unsafe {
            let pb = &mut (*self.raw_ptr).pb;
            if !pb.is_null() {
                ffmpeg_ffi::avio_context_free(pb);
            }
            ffmpeg_ffi::avformat_free_context(self.raw_ptr);
        }
    }
}
