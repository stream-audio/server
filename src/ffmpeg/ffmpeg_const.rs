pub const AVERROR_EOF: i32 = ff_err_tag('E', 'O', 'F', ' ');

pub const AV_NOPTS_VALUE: i64 = (0x8000000000000000 as u64) as i64;

pub const fn av_error(e: u32) -> i32 {
    -(e as i32)
}

const fn ff_err_tag(a: char, b: char, c: char, d: char) -> i32 {
    -mk_tag(a, b, c, d)
}

const fn mk_tag(a: char, b: char, c: char, d: char) -> i32 {
    let (a, b, c, d) = (a as u32, b as u32, c as u32, d as u32);
    let res = a | (b << 8) | (c << 16) | (d << 24);
    res as _
}
