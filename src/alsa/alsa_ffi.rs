use libc::{c_char, c_int, c_long, c_uint, c_ulong, c_void, size_t};

#[repr(C)]
pub struct snd_pcm_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_ctl_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_ctl_card_info_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_pcm_info_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_pcm_hw_params_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_pcm_sw_params_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_pcm_status_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct snd_output_t {
    _private: [u8; 0],
}

pub const EAGAIN: c_int = 11;
pub const EPIPE: c_int = 32;

pub type snd_pcm_uframes_t = c_ulong;
pub type snd_pcm_sframes_t = c_long;

pub type snd_pcm_stream_t = c_uint;
pub const SND_PCM_STREAM_PLAYBACK: c_uint = 0;
pub const SND_PCM_STREAM_CAPTURE: c_uint = 1;

pub type snd_pcm_format_t = c_int;
pub const SND_PCM_FORMAT_UNKNOWN: c_int = -1;
pub const SND_PCM_FORMAT_S8: c_int = 0;
pub const SND_PCM_FORMAT_U8: c_int = 1;
pub const SND_PCM_FORMAT_S16_LE: c_int = 2;
pub const SND_PCM_FORMAT_S16_BE: c_int = 3;
pub const SND_PCM_FORMAT_U16_LE: c_int = 4;
pub const SND_PCM_FORMAT_U16_BE: c_int = 5;
pub const SND_PCM_FORMAT_S24_LE: c_int = 6;
pub const SND_PCM_FORMAT_S24_BE: c_int = 7;
pub const SND_PCM_FORMAT_U24_LE: c_int = 8;
pub const SND_PCM_FORMAT_U24_BE: c_int = 9;
pub const SND_PCM_FORMAT_S32_LE: c_int = 10;
pub const SND_PCM_FORMAT_S32_BE: c_int = 11;
pub const SND_PCM_FORMAT_U32_LE: c_int = 12;
pub const SND_PCM_FORMAT_U32_BE: c_int = 13;
pub const SND_PCM_FORMAT_FLOAT_LE: c_int = 14;
pub const SND_PCM_FORMAT_FLOAT_BE: c_int = 15;
pub const SND_PCM_FORMAT_FLOAT64_LE: c_int = 16;
pub const SND_PCM_FORMAT_FLOAT64_BE: c_int = 17;
pub const SND_PCM_FORMAT_IEC958_SUBFRAME_LE: c_int = 18;
pub const SND_PCM_FORMAT_IEC958_SUBFRAME_BE: c_int = 19;
pub const SND_PCM_FORMAT_MU_LAW: c_int = 20;
pub const SND_PCM_FORMAT_A_LAW: c_int = 21;
pub const SND_PCM_FORMAT_IMA_ADPCM: c_int = 22;
pub const SND_PCM_FORMAT_MPEG: c_int = 23;
pub const SND_PCM_FORMAT_GSM: c_int = 24;
pub const SND_PCM_FORMAT_SPECIAL: c_int = 31;
pub const SND_PCM_FORMAT_S24_3LE: c_int = 32;
pub const SND_PCM_FORMAT_S24_3BE: c_int = 33;
pub const SND_PCM_FORMAT_U24_3LE: c_int = 34;
pub const SND_PCM_FORMAT_U24_3BE: c_int = 35;
pub const SND_PCM_FORMAT_S20_3LE: c_int = 36;
pub const SND_PCM_FORMAT_S20_3BE: c_int = 37;
pub const SND_PCM_FORMAT_U20_3LE: c_int = 38;
pub const SND_PCM_FORMAT_U20_3BE: c_int = 39;
pub const SND_PCM_FORMAT_S18_3LE: c_int = 40;
pub const SND_PCM_FORMAT_S18_3BE: c_int = 41;
pub const SND_PCM_FORMAT_U18_3LE: c_int = 42;
pub const SND_PCM_FORMAT_U18_3BE: c_int = 43;
pub const SND_PCM_FORMAT_G723_24: c_int = 44;
pub const SND_PCM_FORMAT_G723_24_1B: c_int = 45;
pub const SND_PCM_FORMAT_G723_40: c_int = 46;
pub const SND_PCM_FORMAT_G723_40_1B: c_int = 47;
pub const SND_PCM_FORMAT_DSD_U8: c_int = 48;
pub const SND_PCM_FORMAT_DSD_U16_LE: c_int = 49;
pub const SND_PCM_FORMAT_DSD_U32_LE: c_int = 50;
pub const SND_PCM_FORMAT_DSD_U16_BE: c_int = 51;
pub const SND_PCM_FORMAT_DSD_U32_BE: c_int = 52;
pub const SND_PCM_FORMAT_S16: c_int = 2;
pub const SND_PCM_FORMAT_U16: c_int = 4;
pub const SND_PCM_FORMAT_S24: c_int = 6;
pub const SND_PCM_FORMAT_U24: c_int = 8;
pub const SND_PCM_FORMAT_S32: c_int = 10;
pub const SND_PCM_FORMAT_U32: c_int = 12;
pub const SND_PCM_FORMAT_FLOAT: c_int = 14;
pub const SND_PCM_FORMAT_FLOAT64: c_int = 16;
pub const SND_PCM_FORMAT_IEC958_SUBFRAME: c_int = 18;
pub const SND_PCM_FORMAT_LAST: c_int = 52;

pub type snd_pcm_access_t = c_uint;
pub const SND_PCM_ACCESS_MMAP_INTERLEAVED: c_uint = 0;
pub const SND_PCM_ACCESS_MMAP_NONINTERLEAVED: c_uint = 1;
pub const SND_PCM_ACCESS_MMAP_COMPLEX: c_uint = 2;
pub const SND_PCM_ACCESS_RW_INTERLEAVED: c_uint = 3;
pub const SND_PCM_ACCESS_RW_NONINTERLEAVED: c_uint = 4;
pub const SND_PCM_ACCESS_LAST: c_uint = 4;

pub type snd_pcm_state_t = c_uint;
pub const SND_PCM_STATE_OPEN: ::libc::c_uint = 0;
pub const SND_PCM_STATE_SETUP: ::libc::c_uint = 1;
pub const SND_PCM_STATE_PREPARED: ::libc::c_uint = 2;
pub const SND_PCM_STATE_RUNNING: ::libc::c_uint = 3;
pub const SND_PCM_STATE_XRUN: ::libc::c_uint = 4;
pub const SND_PCM_STATE_DRAINING: ::libc::c_uint = 5;
pub const SND_PCM_STATE_PAUSED: ::libc::c_uint = 6;
pub const SND_PCM_STATE_SUSPENDED: ::libc::c_uint = 7;
pub const SND_PCM_STATE_DISCONNECTED: ::libc::c_uint = 8;
pub const SND_PCM_STATE_LAST: ::libc::c_uint = 8;

pub const SND_CTL_NONBLOCK: c_int = 1;
pub const SND_CTL_ASYNC: c_int = 2;
pub const SND_CTL_READONLY: c_int = 4;

#[link(name = "asound")]
extern "C" {
    pub fn snd_strerror(errnum: c_int) -> *const c_char;

    pub fn snd_pcm_open(
        pcmp: *mut *mut snd_pcm_t,
        name: *const c_char,
        stream: snd_pcm_stream_t,
        mode: c_int,
    ) -> c_int;

    pub fn snd_pcm_drain(pcm: *mut snd_pcm_t) -> c_int;
    pub fn snd_pcm_close(pcm: *mut snd_pcm_t) -> c_int;

    pub fn snd_pcm_set_params(
        pcm: *mut snd_pcm_t,
        format: snd_pcm_format_t,
        access: snd_pcm_access_t,
        channels: c_uint,
        rate: c_uint,
        soft_resample: c_int,
        latency: c_uint,
    ) -> c_int;

    pub fn snd_pcm_hw_params(pcm: *mut snd_pcm_t, params: *mut snd_pcm_hw_params_t) -> c_int;
    pub fn snd_pcm_prepare(pcm: *mut snd_pcm_t) -> c_int;

    pub fn snd_ctl_pcm_next_device(ctl: *mut snd_ctl_t, device: *mut c_int) -> c_int;

    pub fn snd_pcm_info_malloc(ptr: *mut *mut snd_pcm_info_t) -> c_int;
    pub fn snd_pcm_info_free(ptr: *mut snd_pcm_info_t);
    pub fn snd_pcm_info(pcm: *mut snd_pcm_t, info: *mut snd_pcm_info_t) -> c_int;
    pub fn snd_pcm_info_set_device(info: *mut snd_pcm_info_t, dev: c_uint);
    pub fn snd_pcm_info_set_subdevice(info: *mut snd_pcm_info_t, sub_dev: c_uint);
    pub fn snd_pcm_info_set_stream(info: *mut snd_pcm_info_t, stream: snd_pcm_stream_t);

    pub fn snd_pcm_readi(
        pcm: *mut snd_pcm_t,
        buffer: *mut c_void,
        size: snd_pcm_uframes_t,
    ) -> snd_pcm_sframes_t;

    pub fn snd_pcm_writei(
        pcm: *mut snd_pcm_t,
        buffer: *const c_void,
        size: snd_pcm_uframes_t,
    ) -> snd_pcm_sframes_t;

    pub fn snd_pcm_wait(pcm: *mut snd_pcm_t, timeout: c_int) -> c_int;

    pub fn snd_pcm_info_get_device(info: *const snd_pcm_info_t) -> c_uint;
    pub fn snd_pcm_info_get_id(info: *const snd_pcm_info_t) -> *const c_char;
    pub fn snd_pcm_info_get_name(info: *const snd_pcm_info_t) -> *const c_char;
    pub fn snd_pcm_info_get_subdevices_count(info: *const snd_pcm_info_t) -> c_uint;

    pub fn snd_pcm_hw_params_malloc(ptr: *mut *mut snd_pcm_hw_params_t) -> c_int;
    pub fn snd_pcm_hw_params_free(ptr: *mut snd_pcm_hw_params_t);
    pub fn snd_pcm_hw_params_any(pcm: *mut snd_pcm_t, params: *mut snd_pcm_hw_params_t) -> c_int;
    pub fn snd_pcm_hw_params_set_access(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_hw_params_t,
        access: snd_pcm_access_t,
    ) -> c_int;
    pub fn snd_pcm_hw_params_set_format(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_hw_params_t,
        format: snd_pcm_format_t,
    ) -> c_int;
    pub fn snd_pcm_hw_params_set_channels(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_hw_params_t,
        val: c_uint,
    ) -> c_int;
    pub fn snd_pcm_hw_params_set_rate_near(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_hw_params_t,
        val: *mut c_uint,
        dir: *mut c_int,
    ) -> c_int;
    pub fn snd_pcm_hw_params_set_period_time_near(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_hw_params_t,
        val: *mut c_uint,
        dir: *mut c_int,
    ) -> c_int;
    pub fn snd_pcm_hw_params_set_buffer_time_near(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_hw_params_t,
        val: *mut c_uint,
        dir: *mut c_int,
    ) -> c_int;

    pub fn snd_pcm_hw_params_get_period_size(
        params: *const snd_pcm_hw_params_t,
        val: *mut snd_pcm_uframes_t,
        dir: *mut c_int,
    ) -> c_int;
    pub fn snd_pcm_hw_params_get_buffer_size(
        params: *const snd_pcm_hw_params_t,
        val: *mut snd_pcm_uframes_t,
    ) -> c_int;
    pub fn snd_pcm_hw_params_get_buffer_time_max(
        params: *const snd_pcm_hw_params_t,
        val: *mut c_uint,
        dir: *mut c_int,
    ) -> c_int;

    pub fn snd_pcm_sw_params_malloc(ptr: *mut *mut snd_pcm_sw_params_t) -> c_int;
    pub fn snd_pcm_sw_params_free(ptr: *mut snd_pcm_sw_params_t);
    pub fn snd_pcm_sw_params_current(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_sw_params_t,
    ) -> c_int;
    pub fn snd_pcm_sw_params(pcm: *mut snd_pcm_t, params: *mut snd_pcm_sw_params_t) -> c_int;
    pub fn snd_pcm_sw_params_set_start_threshold(
        pcm: *mut snd_pcm_t,
        params: *mut snd_pcm_sw_params_t,
        val: snd_pcm_uframes_t,
    ) -> c_int;

    pub fn snd_pcm_status_malloc(ptr: *mut *mut snd_pcm_status_t) -> c_int;
    pub fn snd_pcm_status_free(ptr: *mut snd_pcm_status_t);
    pub fn snd_pcm_status(pcm: *mut snd_pcm_t, status: *mut snd_pcm_status_t) -> c_int;
    pub fn snd_pcm_status_get_state(status: *const snd_pcm_status_t) -> snd_pcm_state_t;

    pub fn snd_ctl_pcm_info(ctl: *mut snd_ctl_t, info: *mut snd_pcm_info_t) -> c_int;

    pub fn snd_ctl_open(ctlp: *mut *mut snd_ctl_t, name: *const c_char, mode: c_int) -> c_int;
    pub fn snd_ctl_close(ctl: *mut snd_ctl_t) -> c_int;

    pub fn snd_card_next(rcard: *mut c_int) -> c_int;

    pub fn snd_ctl_card_info_malloc(infop: *mut *mut snd_ctl_card_info_t) -> c_int;
    pub fn snd_ctl_card_info_free(info: *mut snd_ctl_card_info_t);
    pub fn snd_ctl_card_info(ctl: *mut snd_ctl_t, info: *mut snd_ctl_card_info_t) -> c_int;
    pub fn snd_ctl_card_info_get_id(info: *mut snd_ctl_card_info_t) -> *const c_char;
    pub fn snd_ctl_card_info_get_name(info: *mut snd_ctl_card_info_t) -> *const c_char;

    pub fn snd_output_buffer_open(outputp: *mut *mut snd_output_t) -> c_int;
    pub fn snd_output_close(output: *mut snd_output_t) -> c_int;
    pub fn snd_output_buffer_string(output: *mut snd_output_t, buf: *mut *mut c_char) -> size_t;
    pub fn snd_output_flush(output: *mut snd_output_t) -> c_int;

    pub fn snd_pcm_dump(pcm: *mut snd_pcm_t, out: *mut snd_output_t) -> c_int;
}
