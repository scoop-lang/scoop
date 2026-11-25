use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::bytes;

#[unsafe(no_mangle)]
extern "C" fn scoop_charset_scalar_width(scalar: u32, ambiguous_as_wide: bool) -> i32 {
    if scalar == 0 {
        return -1;
    }
    char::from_u32(scalar)
        .and_then(|character| {
            if ambiguous_as_wide {
                character.width_cjk()
            } else {
                character.width()
            }
        })
        .map_or(-1, |width| width as i32)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn scoop_charset_text_width(
    data: *const u8,
    size: i64,
    ambiguous_as_wide: bool,
) -> i64 {
    // SAFETY: The caller retains the readable UTF-8 span for this call only.
    let Ok(text) = (unsafe { bytes::input(data, size) }) else {
        return -1;
    };
    if text.chars().any(|ch| ch == '\0' || ch.width().is_none()) {
        return -1;
    }
    let width = if ambiguous_as_wide {
        text.width_cjk()
    } else {
        text.width()
    };
    i64::try_from(width).unwrap_or(-1)
}

#[unsafe(no_mangle)]
extern "C" fn scoop_charset_terminal_unicode_version() -> u32 {
    // The locked ICU4X segmenter data also uses Unicode 17.0.0.
    let (major, minor, patch) = unicode_width::UNICODE_VERSION;
    (u32::from(major) << 16) | (u32::from(minor) << 8) | u32::from(patch)
}
