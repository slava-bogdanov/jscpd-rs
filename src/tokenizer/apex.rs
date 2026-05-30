use crate::cli::Options;

use super::embedded::{
    assign_sequential_positions, offset_tokens, tokenize_generic_with_whitespace,
};
use super::{LineIndex, TokenMap, find_ignore_regions, tokenize_generic};

pub(super) fn tokenize_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let mut maps = Vec::new();
    let apex_tokens = tokenize_generic(content, "apex", options, ignore_regions);
    if !apex_tokens.is_empty() {
        maps.push(TokenMap {
            format: "apex".to_string(),
            tokens: apex_tokens,
            positions_assigned: false,
        });
    }

    let sql_blocks = soql_blocks(content);
    if sql_blocks.is_empty() {
        return maps;
    }

    let line_index = LineIndex::new(content);
    let mut sql_tokens = Vec::new();
    for block in sql_blocks {
        let inner = &content[block.start..block.end];
        let inner_ignore_regions = find_ignore_regions(inner, options);
        let mut tokens =
            tokenize_generic_with_whitespace(inner, "sql", options, &inner_ignore_regions);
        let block_start = line_index.location(block.start);
        offset_tokens(&mut tokens, block.start, &block_start);
        sql_tokens.extend(tokens);
    }

    if !sql_tokens.is_empty() {
        sql_tokens.sort_by_key(|token| (token.range[0], token.range[1]));
        assign_sequential_positions(&mut sql_tokens);
        maps.push(TokenMap {
            format: "sql".to_string(),
            tokens: sql_tokens,
            positions_assigned: true,
        });
    }

    maps
}

#[derive(Clone, Copy)]
struct SoqlBlock {
    start: usize,
    end: usize,
}

fn soql_blocks(content: &str) -> Vec<SoqlBlock> {
    let bytes = content.as_bytes();
    let mut blocks = Vec::new();
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] != b'[' {
            idx += 1;
            continue;
        }
        let Some(end) = find_closing_bracket(bytes, idx + 1) else {
            break;
        };
        if looks_like_soql(&content[idx + 1..end]) {
            blocks.push(SoqlBlock {
                start: idx,
                end: end + 1,
            });
        }
        idx = end + 1;
    }
    blocks
}

fn find_closing_bracket(bytes: &[u8], start: usize) -> Option<usize> {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b']')
        .map(|offset| start + offset)
}

fn looks_like_soql(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed
        .get(..6)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("select"))
        || trimmed
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("find"))
}
