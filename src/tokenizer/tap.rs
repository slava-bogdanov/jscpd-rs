use crate::cli::Options;

use super::embedded::{assign_sequential_positions, blank_ranges_preserve_newlines, offset_tokens};
use super::{
    ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, TokenMap, find_ignore_regions,
    push_token, tokenize_generic,
};

pub(super) fn tokenize_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let blocks = tap_yaml_blocks(content);
    let mut maps = Vec::new();
    let sanitized = blank_ranges_preserve_newlines(
        content,
        blocks
            .iter()
            .map(|block| [block.start, block.end])
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let tap_tokens = tokenize_tap_outer(&sanitized, options, ignore_regions);
    if !tap_tokens.is_empty() {
        maps.push(TokenMap {
            format: "tap".to_string(),
            tokens: tap_tokens,
            positions_assigned: false,
        });
    }

    let line_index = LineIndex::new(content);
    let mut yaml_tokens = Vec::<DetectionToken>::new();
    for block in blocks {
        let inner = &content[block.start..block.end];
        let inner_ignore_regions = find_ignore_regions(inner, options);
        let mut tokens = tokenize_generic(inner, "yaml", options, &inner_ignore_regions);
        let start = line_index.location(block.start);
        offset_tokens(&mut tokens, block.start, &start);
        yaml_tokens.extend(tokens);
    }
    if !yaml_tokens.is_empty() {
        assign_sequential_positions(&mut yaml_tokens);
        maps.push(TokenMap {
            format: "yaml".to_string(),
            tokens: yaml_tokens,
            positions_assigned: true,
        });
    }

    maps
}

fn tokenize_tap_outer(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<DetectionToken> {
    let context = TokenContext {
        content,
        options,
        ignore_regions,
    };
    let line_index = LineIndex::new(content);
    let mut tokens = Vec::new();

    for span in line_spans(content) {
        let line = &content[span.start..span.end];
        let Some(start_offset) = first_non_whitespace(line) else {
            continue;
        };
        let end_offset = trim_line_end(line);
        if start_offset >= end_offset {
            continue;
        }
        let start = span.start + start_offset;
        let end = span.start + end_offset;
        push_token(
            &mut tokens,
            &context,
            TokenKind::Default,
            ByteSpan { start, end },
            line_index.location(start),
            line_index.location(end),
        );
    }

    tokens
}

#[derive(Clone, Copy)]
struct TapYamlBlock {
    start: usize,
    end: usize,
}

fn tap_yaml_blocks(content: &str) -> Vec<TapYamlBlock> {
    let lines = line_spans(content);
    let mut blocks = Vec::new();
    let mut idx = 0usize;

    while idx < lines.len() {
        let span = lines[idx];
        let line = &content[span.start..span.end];
        let Some(open_start) = tap_yaml_marker_start(line, "---") else {
            idx += 1;
            continue;
        };
        let Some(close_idx) = lines[idx + 1..]
            .iter()
            .position(|span| tap_yaml_marker_start(&content[span.start..span.end], "...").is_some())
            .map(|position| idx + 1 + position)
        else {
            idx += 1;
            continue;
        };
        let close_span = lines[close_idx];
        let close_line = &content[close_span.start..close_span.end];
        let close_start = tap_yaml_marker_start(close_line, "...").unwrap_or(0);

        blocks.push(TapYamlBlock {
            start: span.start + open_start,
            end: close_span.start + close_start + "...".len(),
        });
        idx = close_idx + 1;
    }

    blocks
}

fn tap_yaml_marker_start(line: &str, marker: &str) -> Option<usize> {
    let trimmed_start = line
        .bytes()
        .position(|byte| !matches!(byte, b' ' | b'\t'))
        .unwrap_or(line.len());
    (line[trimmed_start..].trim_end_matches([' ', '\t']) == marker).then_some(trimmed_start)
}

fn first_non_whitespace(line: &str) -> Option<usize> {
    line.bytes().position(|byte| !matches!(byte, b' ' | b'\t'))
}

fn trim_line_end(line: &str) -> usize {
    line.bytes()
        .rposition(|byte| !matches!(byte, b' ' | b'\t'))
        .map(|idx| idx + 1)
        .unwrap_or(0)
}

#[derive(Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
}

fn line_spans(content: &str) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    while start < content.len() {
        let rest = &content[start..];
        let newline = rest.find('\n');
        let end = newline
            .map(|offset| start + offset)
            .unwrap_or(content.len());
        let next_start = newline.map(|offset| start + offset + 1).unwrap_or(end);
        spans.push(LineSpan { start, end });
        start = next_start;
    }
    spans
}
