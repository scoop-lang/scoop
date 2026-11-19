use unicode_width::UnicodeWidthChar;

#[unsafe(no_mangle)]
extern "C" fn scoop_charset_scalar_width(scalar: u32) -> i32 {
    if scalar == 0 {
        return -1;
    }
    char::from_u32(scalar)
        .and_then(UnicodeWidthChar::width)
        .map_or(-1, |width| width as i32)
}
