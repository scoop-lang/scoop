use icu_segmenter::{GraphemeClusterSegmenter, WordSegmenter, options::WordBreakInvariantOptions};

use crate::bytes;

#[unsafe(no_mangle)]
unsafe extern "C" fn scoop_charset_grapheme_breaks(
    data: *const u8,
    size: i64,
    breaks: *mut u8,
) -> i32 {
    // SAFETY: The C caller provides disjoint input/output spans for this call.
    unsafe { segment(data, size, breaks, false) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn scoop_charset_word_breaks(data: *const u8, size: i64, breaks: *mut u8) -> i32 {
    // SAFETY: The C caller provides disjoint input/output spans for this call.
    unsafe { segment(data, size, breaks, true) }
}

unsafe fn segment(data: *const u8, size: i64, breaks: *mut u8, words: bool) -> i32 {
    // SAFETY: The C caller provides a readable UTF-8 span for this call.
    let Ok(text) = (unsafe { bytes::input(data, size) }) else {
        return 9;
    };
    if text.is_empty() {
        return 0;
    }
    if breaks.is_null() {
        return 9;
    }
    // SAFETY: The caller provides a disjoint writable span of size bytes.
    let output = unsafe { core::slice::from_raw_parts_mut(breaks, text.len()) };
    output.fill(0);
    let mut mark = |offset| {
        if offset < output.len() {
            output[offset] = 1;
        }
    };
    if words {
        for offset in
            WordSegmenter::new_dictionary(WordBreakInvariantOptions::default()).segment_str(text)
        {
            mark(offset);
        }
    } else {
        for offset in GraphemeClusterSegmenter::new().segment_str(text) {
            mark(offset);
        }
    }
    0
}
