use crate::cli::Options;

use super::generic::{generic_comment_span_end, scan_punctuation_split_token};
use super::{ByteSpan, DetectionToken, LineIndex, Location, TokenContext, TokenKind, push_token};

pub(super) fn blank_ranges_preserve_newlines(content: &str, ranges: &[[usize; 2]]) -> String {
    if ranges.is_empty() {
        return content.to_string();
    }
    let mut bytes = content.as_bytes().to_vec();
    for [start, end] in ranges {
        for byte in &mut bytes[*start..(*end).min(content.len())] {
            if !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(bytes).unwrap_or_else(|_| content.to_string())
}

pub(super) fn offset_tokens(
    tokens: &mut [DetectionToken],
    offset: usize,
    start_location: &Location,
) {
    for token in tokens {
        offset_location(&mut token.start, offset, start_location);
        offset_location(&mut token.end, offset, start_location);
        token.range[0] += offset;
        token.range[1] += offset;
    }
}

pub(super) fn assign_sequential_positions(tokens: &mut [DetectionToken]) {
    for (position, token) in tokens.iter_mut().enumerate() {
        token.start.position = position;
        token.end.position = position;
    }
}

pub(super) fn tokenize_generic_with_whitespace(
    content: &str,
    format: &str,
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
    let mut start_byte = 0usize;

    while start_byte < content.len() {
        let ch = content[start_byte..].chars().next().unwrap_or('\0');
        let (end_byte, kind) = if ch.is_whitespace() {
            (scan_whitespace(content, start_byte), TokenKind::Default)
        } else if let Some(comment_end) =
            generic_comment_span_end(content, format, start_byte, content.len())
        {
            (comment_end, TokenKind::Comment)
        } else {
            scan_punctuation_split_token(content, format, start_byte)
        };
        push_token(
            &mut tokens,
            &context,
            kind,
            ByteSpan {
                start: start_byte,
                end: end_byte,
            },
            line_index.location(start_byte),
            line_index.location(end_byte),
        );
        start_byte = end_byte.max(start_byte + ch.len_utf8());
    }

    tokens
}

fn offset_location(location: &mut Location, offset: usize, start_location: &Location) {
    if location.line == 1 {
        location.column += start_location.column.saturating_sub(1);
    }
    location.line += start_location.line.saturating_sub(1);
    location.position += offset;
}

fn scan_whitespace(content: &str, start: usize) -> usize {
    let bytes = content.as_bytes();
    if bytes[start] == b'\n' {
        return start + 1;
    }
    let mut end = start;
    while end < content.len() {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if ch == '\n' || !ch.is_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    end
}
