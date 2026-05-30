#![allow(dead_code)]

use anyhow::Result;

use crate::cli::Options;
use crate::formats;
use crate::tokenizer::TokenMap;

pub fn tokenize_with_fallback(
    _content: &str,
    format: &str,
    _options: &Options,
) -> Result<Option<Vec<TokenMap>>> {
    let _grammar_format = formats::tokenizer_format(format);
    Ok(None)
}
