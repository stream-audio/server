macro_rules! try_ffmpeg {
    ($e:expr, $ctx:expr) => {{
        let res = $e;
        if res < 0 {
            return Err(crate::ffmpeg::InternalError::new(res, $ctx).into());
        }
        res
    }};
}
