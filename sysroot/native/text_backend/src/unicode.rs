use icu_casemap::CaseMapper;
use icu_locale_core::LanguageIdentifier;
use icu_properties::{CodePointSetData, props::Alnum};

use crate::bytes::{self, TextResult};

#[unsafe(no_mangle)]
extern "C" fn scoop_charset_alphanumeric(scalar: u32) -> bool {
    char::from_u32(scalar)
        .is_some_and(|character| CodePointSetData::new::<Alnum>().contains(character))
}

#[unsafe(no_mangle)]
unsafe extern "C" fn scoop_charset_case(data: *const u8, size: i64, mode: i32) -> TextResult {
    let result = (|| {
        // SAFETY: The C caller keeps the input readable during this call.
        let text = unsafe { bytes::input(data, size) }.map_err(|_| 9)?;
        let mapper = CaseMapper::new();
        let locale = LanguageIdentifier::UNKNOWN;
        Ok(match mode {
            0 => mapper.lowercase_to_string(text, &locale),
            1 => mapper.uppercase_to_string(text, &locale),
            2 => mapper.fold_string(text),
            _ => return Err(9),
        }
        .into_owned())
    })();
    TextResult::from_text(result)
}
