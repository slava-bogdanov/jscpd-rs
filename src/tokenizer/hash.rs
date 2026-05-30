use xxhash_rust::xxh3::xxh3_64;

use super::TokenKind;

pub(super) fn hash_token(kind: TokenKind, value: &str, ignore_case: bool) -> u64 {
    let kind_hash = match kind {
        TokenKind::Comment => 0x01_u64,
        TokenKind::Constant => 0x08_u64,
        TokenKind::Keyword => 0x02_u64,
        TokenKind::Number => 0x03_u64,
        TokenKind::Operator => 0x04_u64,
        TokenKind::Punctuation => 0x05_u64,
        TokenKind::String => 0x06_u64,
        TokenKind::Default => 0x07_u64,
    };
    hash_value(value, ignore_case) ^ kind_hash
}

fn hash_value(value: &str, ignore_case: bool) -> u64 {
    if ignore_case {
        xxh3_64(value.to_lowercase().as_bytes())
    } else {
        xxh3_64(value.as_bytes())
    }
}
