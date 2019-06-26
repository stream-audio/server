use super::{
    error::UnsupportedSampleFormatError, ffmpeg_const, ffmpeg_ffi, format, AudioParams,
    AudioSampleFormat, Error, InternalError,
};
use std::ptr;
use std::slice;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Codec {
    Aac,
    AacLd,
    Mp2,
}

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub codec: Codec,
    pub bit_rate: u32,
    pub audio_params: AudioParams,
}

pub struct Encoder {
    ctx: AvCodecContext,
    pkt: AvPacket,
    frame: AvFrameEncoder,
    write_buf: Vec<u8>,
    formatter: format::SinglePacketFormatter,
}

pub struct Decoder {
    ctx: AvCodecContext,
    parser: AvCodecParserContext,
    pkt: AvPacket,
    frame: AvFrameDecoder,
    write_buf: Vec<u8>,
    read_buf: Vec<u8>,
}

impl Encoder {
    pub fn new(params: Params) -> Result<Self, Error> {
        let ctx = AvCodecContext::new_encoder(params)?;
        let pkt = AvPacket::new()?;
        let frame = AvFrameEncoder::new(&ctx)?;
        let formatter = format::SinglePacketFormatter::new(&ctx, params.codec)?;

        Ok(Self {
            ctx,
            pkt,
            frame,
            write_buf: Vec::new(),
            formatter,
        })
    }

    pub fn write(&mut self, in_buf: &[u8]) -> Result<(), Error> {
        self.write_buf.extend_from_slice(in_buf);

        let mut total_copied = 0;
        loop {
            let res = self
                .frame
                .copy_from_slice(&self.write_buf[total_copied..])?;
            match res {
                Some(copied) => total_copied += copied,
                None => break,
            }

            self.ctx.send_frame(&self.frame)?;
        }

        self.write_buf.drain(..total_copied);

        Ok(())
    }

    pub fn read(&mut self) -> Result<Option<&[u8]>, Error> {
        self.pkt.unref();

        let res = self.ctx.receive_pkt(&mut self.pkt);
        match res {
            Ok(()) => {
                // Ok(Some(self.pkt.get_buf_ref()))
                self.formatter.write_pkt(&self.pkt)?;
                Ok(Some(self.formatter.get_buf()))
            }
            Err(e) => {
                if e.is_again_or_eof() {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }
}

impl Decoder {
    pub fn new(codec: Codec) -> Result<Self, Error> {
        let ctx = AvCodecContext::new_decoder(codec)?;
        let parser = AvCodecParserContext::new(codec)?;
        let pkt = AvPacket::new()?;
        let frame = AvFrameDecoder::new()?;

        Ok(Self {
            ctx,
            parser,
            pkt,
            frame,
            write_buf: Vec::new(),
            read_buf: Vec::new(),
        })
    }

    pub fn write(&mut self, in_buf: &[u8]) -> Result<(), Error> {
        const BUFFER_PADDING_SIZE: usize = ffmpeg_ffi::AV_INPUT_BUFFER_PADDING_SIZE as _;

        self.write_buf.extend_from_slice(in_buf);
        self.write_buf.reserve(BUFFER_PADDING_SIZE);

        let mut written_bytes = 0;

        while written_bytes < self.write_buf.len() {
            let parsed =
                self.parser
                    .parse(&self.ctx, &mut self.pkt, &self.write_buf[written_bytes..])?;

            written_bytes += parsed;

            if !self.pkt.is_empty() {
                self.ctx.send_pkt(&self.pkt)?;
            }
        }

        self.write_buf.clear();

        Ok(())
    }

    pub fn read(&mut self) -> Result<Option<&[u8]>, Error> {
        let res = self.ctx.receive_frame(&mut self.frame);
        if let Err(e) = res {
            return if e.is_again_or_eof() {
                Ok(None)
            } else {
                Err(e.into())
            };
        }

        self.read_buf.clear();
        self.frame.copy_data_to(&self.ctx, &mut self.read_buf);

        Ok(Some(&self.read_buf))
    }
}

const LAYOUT: u32 = super::resample::LAYOUT;

pub struct AvCodecContext {
    pub raw_ptr: *mut ffmpeg_ffi::AVCodecContext,
    av_codec: AvCodec,
}

struct AvCodecParserContext {
    raw_ptr: *mut ffmpeg_ffi::AVCodecParserContext,
}

struct AvCodec {
    raw_ptr: *const ffmpeg_ffi::AVCodec,
}

pub struct AvPacket {
    pub raw_ptr: *mut ffmpeg_ffi::AVPacket,
}

struct AvFrameEncoder {
    raw_ptr: *mut ffmpeg_ffi::AVFrame,
    size: usize,
    is_planar: bool,
    channels: usize,
}

struct AvFrameDecoder {
    raw_ptr: *mut ffmpeg_ffi::AVFrame,
}

impl Codec {
    fn to_raw(self) -> ffmpeg_ffi::AVCodecID {
        match self {
            Codec::Aac | Codec::AacLd => ffmpeg_ffi::AVCodecID_AV_CODEC_ID_AAC,
            Codec::Mp2 => ffmpeg_ffi::AVCodecID_AV_CODEC_ID_MP2,
        }
    }

    fn get_profile(self) -> Option<i32> {
        match self {
            Codec::AacLd => Some(ffmpeg_ffi::FF_PROFILE_AAC_LD as _),
            _ => None,
        }
    }

    pub fn get_default_muxer_name(self) -> &'static str {
        match self {
            Codec::Aac | Codec::AacLd => "adts",
            Codec::Mp2 => "data",
        }
    }
}

impl AvCodecContext {
    fn new_encoder(params: Params) -> Result<Self, Error> {
        unsafe {
            let av_codec = AvCodec::find_encoder(params.codec)?;
            let raw_ptr = ffmpeg_ffi::avcodec_alloc_context3(av_codec.raw_ptr);

            if raw_ptr.is_null() {
                return Err(Error::new_cannot_allocate("audio coder context (encoder)").into());
            }

            let mut res = Self { raw_ptr, av_codec };

            res.set_and_validate_sample_fmt(params.audio_params.format)?;
            (*res.raw_ptr).bit_rate = params.bit_rate as _;
            (*res.raw_ptr).sample_rate = params.audio_params.rate;
            (*res.raw_ptr).channel_layout = LAYOUT as _;
            (*res.raw_ptr).channels = ffmpeg_ffi::av_get_channel_layout_nb_channels(LAYOUT as _);
            if let Some(profile) = params.codec.get_profile() {
                (*res.raw_ptr).profile = profile;
            }

            try_ffmpeg!(
                ffmpeg_ffi::avcodec_open2(res.raw_ptr, res.av_codec.raw_ptr, ptr::null_mut()),
                "opening codec (encoder)"
            );

            eprintln!("frame_size: {}", (*res.raw_ptr).frame_size);

            Ok(res)
        }
    }

    fn new_decoder(codec: Codec) -> Result<Self, Error> {
        unsafe {
            let av_codec = AvCodec::find_decoder(codec)?;
            let raw_ptr = ffmpeg_ffi::avcodec_alloc_context3(av_codec.raw_ptr);

            if raw_ptr.is_null() {
                return Err(Error::new_cannot_allocate("audio coder context (decoder)").into());
            }

            let res = Self { raw_ptr, av_codec };

            try_ffmpeg!(
                ffmpeg_ffi::avcodec_open2(res.raw_ptr, res.av_codec.raw_ptr, ptr::null_mut()),
                "opening codec (decoder)"
            );

            Ok(res)
        }
    }

    #[allow(dead_code)]
    fn get_sample_format(&self) -> Option<AudioSampleFormat> {
        unsafe { AudioSampleFormat::from_raw((*self.raw_ptr).sample_fmt) }
    }

    fn set_sample_format(&mut self, format: AudioSampleFormat) {
        unsafe {
            (*self.raw_ptr).sample_fmt = format.to_raw();
        }
    }

    fn get_frame_size_in_bytes(&self) -> usize {
        unsafe {
            let raw_ptr = &*self.raw_ptr;
            let res = raw_ptr.frame_size
                * raw_ptr.channels
                * ffmpeg_ffi::av_get_bytes_per_sample(raw_ptr.sample_fmt);
            res as _
        }
    }

    fn get_channels_qty(&self) -> usize {
        unsafe { (*self.raw_ptr).channels as _ }
    }

    fn get_bytes_per_sample(&self) -> usize {
        unsafe { ffmpeg_ffi::av_get_bytes_per_sample((*self.raw_ptr).sample_fmt) as _ }
    }

    fn send_frame(&mut self, frame: &AvFrameEncoder) -> Result<(), Error> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::avcodec_send_frame(self.raw_ptr, frame.raw_ptr),
                "sending frame"
            );
        }

        Ok(())
    }

    fn send_pkt(&mut self, pkt: &AvPacket) -> Result<(), Error> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::avcodec_send_packet(self.raw_ptr, pkt.raw_ptr),
                "sending packet"
            );
        }
        Ok(())
    }

    fn receive_frame(&mut self, frame: &mut AvFrameDecoder) -> Result<(), InternalError> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::avcodec_receive_frame(self.raw_ptr, frame.raw_ptr),
                "receiving frame"
            );
        }
        Ok(())
    }

    fn receive_pkt(&mut self, pkt: &mut AvPacket) -> Result<(), InternalError> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::avcodec_receive_packet(self.raw_ptr, pkt.raw_ptr),
                "receiving packet"
            );
        }
        Ok(())
    }

    fn set_and_validate_sample_fmt(&mut self, format: AudioSampleFormat) -> Result<(), Error> {
        let supported_formats = self.av_codec.get_supported_sample_formats();
        eprintln!("{:?}", supported_formats);

        let supported_interleaved_formats = || {
            return supported_formats
                .iter()
                .map(|f| f.to_interleaved())
                .collect::<Vec<AudioSampleFormat>>();
        };

        if format.is_planar() {
            Err(UnsupportedSampleFormatError {
                fmt: format,
                supported: supported_interleaved_formats(),
            }
            .into())
        } else if supported_formats.contains(&format) {
            self.set_sample_format(format);
            Ok(())
        } else if supported_formats.contains(&format.to_planar()) {
            self.set_sample_format(format.to_planar());
            Ok(())
        } else {
            Err(UnsupportedSampleFormatError {
                fmt: format,
                supported: supported_interleaved_formats(),
            }
            .into())
        }
    }
}
impl Drop for AvCodecContext {
    fn drop(&mut self) {
        unsafe {
            ffmpeg_ffi::avcodec_free_context(&mut self.raw_ptr);
        }
    }
}

impl AvCodecParserContext {
    fn new(codec: Codec) -> Result<Self, Error> {
        unsafe {
            let raw_ptr = ffmpeg_ffi::av_parser_init(codec.to_raw() as _);
            if raw_ptr.is_null() {
                Err(Error::new_cannot_find_parser(codec))
            } else {
                Ok(Self { raw_ptr })
            }
        }
    }

    fn parse(&self, ctx: &AvCodecContext, pkt: &mut AvPacket, buf: &[u8]) -> Result<usize, Error> {
        unsafe {
            let pkt_raw = &mut *pkt.raw_ptr;

            let ret = try_ffmpeg!(
                ffmpeg_ffi::av_parser_parse2(
                    self.raw_ptr,
                    ctx.raw_ptr,
                    &mut pkt_raw.data,
                    &mut pkt_raw.size,
                    buf.as_ptr(),
                    buf.len() as _,
                    ffmpeg_const::AV_NOPTS_VALUE,
                    ffmpeg_const::AV_NOPTS_VALUE,
                    0,
                ),
                "parsing"
            );

            Ok(ret as usize)
        }
    }
}
impl Drop for AvCodecParserContext {
    fn drop(&mut self) {
        unsafe {
            ffmpeg_ffi::av_parser_close(self.raw_ptr);
        }
    }
}

impl AvCodec {
    fn find_encoder(codec: Codec) -> Result<Self, Error> {
        let raw_ptr = unsafe { ffmpeg_ffi::avcodec_find_encoder(codec.to_raw()) };

        if raw_ptr.is_null() {
            Err(Error::new_cannot_find_codec(codec))
        } else {
            Ok(Self { raw_ptr })
        }
    }

    fn find_decoder(codec: Codec) -> Result<Self, Error> {
        let raw_ptr = unsafe { ffmpeg_ffi::avcodec_find_decoder(codec.to_raw()) };

        if raw_ptr.is_null() {
            Err(Error::new_cannot_find_codec(codec))
        } else {
            Ok(Self { raw_ptr })
        }
    }

    fn get_supported_sample_formats(&self) -> Vec<AudioSampleFormat> {
        let mut res = Vec::new();

        unsafe {
            let mut p = (*self.raw_ptr).sample_fmts;

            while *p != ffmpeg_ffi::AVSampleFormat_AV_SAMPLE_FMT_NONE {
                if let Some(format) = AudioSampleFormat::from_raw(*p) {
                    res.push(format);
                }

                p = p.offset(1);
            }
        }

        res
    }
}

impl AvPacket {
    fn new() -> Result<Self, Error> {
        unsafe {
            let raw_ptr = ffmpeg_ffi::av_packet_alloc();
            if raw_ptr.is_null() {
                Err(Error::new_cannot_allocate("packet").into())
            } else {
                Ok(Self { raw_ptr })
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.raw_ptr).size as _ }
    }

    #[allow(dead_code)]
    fn copy_to(&self, out_buf: &mut Vec<u8>) -> usize {
        unsafe {
            let raw_ptr = &*self.raw_ptr;

            let from_buf = slice::from_raw_parts(raw_ptr.data, raw_ptr.size as usize);
            out_buf.extend_from_slice(from_buf);

            raw_ptr.size as _
        }
    }

    #[allow(dead_code)]
    fn get_buf_ref(&self) -> &[u8] {
        unsafe {
            let raw_ptr = &*self.raw_ptr;
            slice::from_raw_parts(raw_ptr.data, raw_ptr.size as usize)
        }
    }

    fn unref(&mut self) {
        unsafe { ffmpeg_ffi::av_packet_unref(self.raw_ptr) }
    }
}

impl Drop for AvPacket {
    fn drop(&mut self) {
        unsafe {
            ffmpeg_ffi::av_packet_free(&mut self.raw_ptr);
        }
    }
}

impl AvFrameEncoder {
    fn new(context: &AvCodecContext) -> Result<Self, Error> {
        unsafe {
            let raw_ptr = ffmpeg_ffi::av_frame_alloc();
            if raw_ptr.is_null() {
                return Err(Error::new_cannot_allocate("audio frame for encoder").into());
            }

            let res = Self {
                raw_ptr,
                size: context.get_frame_size_in_bytes(),
                is_planar: context.get_sample_format().map_or(false, |f| f.is_planar()),
                channels: context.get_channels_qty(),
            };

            (*res.raw_ptr).nb_samples = (*context.raw_ptr).frame_size;
            (*res.raw_ptr).format = (*context.raw_ptr).sample_fmt;
            (*res.raw_ptr).channel_layout = (*context.raw_ptr).channel_layout;

            try_ffmpeg!(
                ffmpeg_ffi::av_frame_get_buffer(res.raw_ptr, 0),
                "allocating audio data buffers"
            );

            Ok(res)
        }
    }

    fn make_writable(&mut self) -> Result<(), Error> {
        unsafe {
            try_ffmpeg!(
                ffmpeg_ffi::av_frame_make_writable(self.raw_ptr),
                "making frame writable"
            );
        }

        Ok(())
    }

    fn copy_from_slice(&mut self, buf: &[u8]) -> Result<Option<usize>, Error> {
        if buf.len() < self.size {
            return Ok(None);
        }

        self.make_writable()?;

        if self.is_planar {
            self.copy_planar_from_slice(&buf[..self.size]);
        } else {
            self.copy_interleaved_from_slice(&buf[..self.size]);
        }

        Ok(Some(self.size))
    }

    fn copy_planar_from_slice(&mut self, buf: &[u8]) {
        unsafe {
            let to_data = &(*self.raw_ptr).data;
            let mut written_len = 0isize;

            let format = AudioSampleFormat::from_raw((*self.raw_ptr).format).unwrap();
            let sample_size = format.get_sample_size();

            for from_channels in buf.chunks_exact(sample_size * self.channels) {
                for (ch, from) in from_channels.chunks_exact(sample_size).enumerate() {
                    let to_buf =
                        slice::from_raw_parts_mut(to_data[ch].offset(written_len), sample_size);

                    to_buf.copy_from_slice(from);
                }
                written_len += sample_size as isize;
            }

            assert_eq!(written_len as usize * self.channels, self.size);
        }
    }

    fn copy_interleaved_from_slice(&mut self, buf: &[u8]) {
        let to_buf = unsafe { slice::from_raw_parts_mut((*self.raw_ptr).data[0], self.size) };
        to_buf.copy_from_slice(buf);
    }
}
impl Drop for AvFrameEncoder {
    fn drop(&mut self) {
        unsafe {
            ffmpeg_ffi::av_frame_free(&mut self.raw_ptr);
        }
    }
}

impl AvFrameDecoder {
    fn new() -> Result<Self, Error> {
        unsafe {
            let raw_ptr = ffmpeg_ffi::av_frame_alloc();
            if raw_ptr.is_null() {
                return Err(Error::new_cannot_allocate("audio frame for decoder").into());
            }

            Ok(Self { raw_ptr })
        }
    }

    fn get_samples_qty(&self) -> usize {
        unsafe { (*self.raw_ptr).nb_samples as _ }
    }

    fn copy_data_to(&self, ctx: &AvCodecContext, buf_to: &mut Vec<u8>) {
        let data_size = ctx.get_bytes_per_sample();
        let channels_qty = ctx.get_channels_qty();

        unsafe {
            let raw_frame = &*self.raw_ptr;

            for sample_i in 0..self.get_samples_qty() {
                let offset = (data_size * sample_i) as isize;
                for ch in 0..channels_qty {
                    let frame_buf =
                        slice::from_raw_parts(raw_frame.data[ch].offset(offset), data_size);
                    buf_to.extend_from_slice(frame_buf);
                }
            }
        }
    }
}
impl Drop for AvFrameDecoder {
    fn drop(&mut self) {
        unsafe {
            ffmpeg_ffi::av_frame_free(&mut self.raw_ptr);
        }
    }
}
