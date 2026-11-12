//! Turning bytes into text, and comparing text the way a bank header needs.
//!
//! A bank export arrives in whatever encoding the bank chose. UTF-8 with or
//! without a byte-order mark and UTF-16 with one are the cases that can be
//! recognised without guessing; anything else is refused rather than decoded
//! with a wrong code page, because a wrong decode silently changes merchant
//! names and, worse, could change an amount.

use crate::ImportError;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

/// Decodes the bytes of a statement, stripping a byte-order mark.
pub(crate) fn decode(bytes: &[u8]) -> Result<String, ImportError> {
    if let Some(rest) = bytes.strip_prefix(UTF8_BOM) {
        return std::str::from_utf8(rest)
            .map(str::to_owned)
            .map_err(|error| invalid_utf8(error.valid_up_to()));
    }
    if let Some(rest) = bytes.strip_prefix(UTF16_LE_BOM) {
        return decode_utf16(rest, true);
    }
    if let Some(rest) = bytes.strip_prefix(UTF16_BE_BOM) {
        return decode_utf16(rest, false);
    }

    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|error| invalid_utf8(error.valid_up_to()))
}

fn invalid_utf8(offset: usize) -> ImportError {
    ImportError::Encoding {
        detail: format!("invalid UTF-8 at byte {offset}"),
    }
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Result<String, ImportError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(ImportError::Encoding {
            detail: "an odd number of bytes in UTF-16 text".to_owned(),
        });
    }

    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes(*pair)
            } else {
                u16::from_be_bytes(*pair)
            }
        })
        .collect();

    String::from_utf16(&units).map_err(|_| ImportError::Encoding {
        detail: "invalid UTF-16".to_owned(),
    })
}

/// Trims, collapses internal whitespace and upper-cases ASCII.
///
/// Bank headers and descriptions vary in spacing and case between exports of
/// the same account, and a duplicate check that misses because of a double space
/// is worse than useless. Upper-casing is ASCII-only so a non-ASCII merchant
/// name keeps its characters.
pub(crate) fn normalise(text: &str) -> String {
    let mut normalised = String::with_capacity(text.len());
    for word in text.split_whitespace() {
        if !normalised.is_empty() {
            normalised.push(' ');
        }
        normalised.push_str(word);
    }
    normalised.to_ascii_uppercase()
}

/// True when two strings are the same after [`normalise`].
pub(crate) fn same(left: &str, right: &str) -> bool {
    normalise(left) == normalise(right)
}
