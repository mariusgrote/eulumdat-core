use std::borrow::Cow;

use encoding_rs::WINDOWS_1252;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The text encoding used to decode input bytes.
pub enum TextEncoding {
    /// UTF-8 input.
    Utf8,
    /// Windows-1252 input, used as the fallback for legacy `.ldt` files.
    Windows1252,
}

pub(crate) fn decode_ldt_bytes(input: &[u8]) -> (Cow<'_, str>, TextEncoding) {
    if let Ok(text) = std::str::from_utf8(input) {
        return (Cow::Borrowed(text), TextEncoding::Utf8);
    }

    let (decoded, _, _) = WINDOWS_1252.decode(input);
    (decoded, TextEncoding::Windows1252)
}
