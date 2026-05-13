use std::borrow::Cow;

use encoding_rs::WINDOWS_1252;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEncoding {
    Utf8,
    Windows1252,
}

pub(crate) fn decode_ldt_bytes(input: &[u8]) -> (Cow<'_, str>, TextEncoding) {
    if let Ok(text) = std::str::from_utf8(input) {
        return (Cow::Borrowed(text), TextEncoding::Utf8);
    }

    let (decoded, _, _) = WINDOWS_1252.decode(input);
    (decoded, TextEncoding::Windows1252)
}
