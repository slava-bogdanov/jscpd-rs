use std::collections::BTreeMap;

use crate::cli::Options;

use super::{ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, push_token};

#[derive(Clone, Copy, Debug)]
pub(super) struct InlineStyleAttr {
    pub(super) attr_start: usize,
    name_start: usize,
    name_end: usize,
    value_start: usize,
    value_end: usize,
    attr_end: usize,
}

pub(super) fn inline_style_attr_ranges(attrs: &[InlineStyleAttr]) -> Vec<[usize; 2]> {
    attrs
        .iter()
        .map(|attr| [attr.attr_start, attr.attr_end])
        .collect()
}

pub(super) fn append_inline_style_attr_tokens(
    grouped: &mut BTreeMap<String, Vec<DetectionToken>>,
    content: &str,
    attrs: &[InlineStyleAttr],
    options: &Options,
    ignore_regions: &[[usize; 2]],
    line_index: &LineIndex,
) {
    if attrs.is_empty() {
        return;
    }

    let context = TokenContext {
        content,
        options,
        ignore_regions,
    };
    let css_tokens = grouped.entry("css".to_string()).or_default();
    for attr in attrs {
        push_inline_style_token(
            css_tokens,
            &context,
            line_index,
            TokenKind::Default,
            attr.attr_start,
            attr.name_start,
        );
        push_inline_style_token(
            css_tokens,
            &context,
            line_index,
            TokenKind::Default,
            attr.name_start,
            attr.name_end,
        );
        push_inline_style_token(
            css_tokens,
            &context,
            line_index,
            TokenKind::Punctuation,
            attr.name_end,
            attr.value_start,
        );
        append_inline_css_value_tokens(css_tokens, &context, line_index, attr);
        push_inline_style_token(
            css_tokens,
            &context,
            line_index,
            TokenKind::Punctuation,
            attr.value_end,
            attr.attr_end,
        );
    }
}

fn append_inline_css_value_tokens(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    line_index: &LineIndex,
    attr: &InlineStyleAttr,
) {
    let mut cursor = attr.value_start;
    while cursor < attr.value_end {
        let ch = context.content[cursor..].chars().next().unwrap_or('\0');
        if inline_css_punctuation(ch) {
            let end = cursor + ch.len_utf8();
            push_inline_style_token(
                tokens,
                context,
                line_index,
                TokenKind::Punctuation,
                cursor,
                end,
            );
            cursor = end;
            continue;
        }

        let start = cursor;
        cursor += ch.len_utf8();
        while cursor < attr.value_end {
            let ch = context.content[cursor..].chars().next().unwrap_or('\0');
            if inline_css_punctuation(ch) {
                break;
            }
            cursor += ch.len_utf8();
        }
        push_inline_style_token(
            tokens,
            context,
            line_index,
            TokenKind::Default,
            start,
            cursor,
        );
    }
}

fn push_inline_style_token(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    line_index: &LineIndex,
    kind: TokenKind,
    start: usize,
    end: usize,
) {
    if start >= end {
        return;
    }
    push_token(
        tokens,
        context,
        kind,
        ByteSpan { start, end },
        line_index.location(start),
        line_index.location(end),
    );
}

fn inline_css_punctuation(ch: char) -> bool {
    matches!(ch, ':' | ';' | '{' | '}' | '(' | ')')
}

pub(super) fn find_inline_style_attrs(content: &str) -> Vec<InlineStyleAttr> {
    let bytes = content.as_bytes();
    let mut attrs = Vec::new();
    let mut cursor = 0usize;

    while let Some(open_offset) = content[cursor..].find('<') {
        let tag_start = cursor + open_offset;
        let tag_kind = bytes.get(tag_start + 1).copied();
        if tag_kind.is_some_and(|byte| matches!(byte, b'/' | b'!' | b'?')) {
            cursor = tag_start + 1;
            continue;
        }

        let Some(tag_end) = find_opening_tag_end(bytes, tag_start + 1) else {
            break;
        };
        collect_style_attrs_in_tag(content, tag_start + 1, tag_end, &mut attrs);
        cursor = tag_end + 1;
    }

    attrs
}

fn collect_style_attrs_in_tag(
    content: &str,
    tag_content_start: usize,
    tag_end: usize,
    attrs: &mut Vec<InlineStyleAttr>,
) {
    let bytes = content.as_bytes();
    let mut cursor = tag_content_start;

    while cursor < tag_end && is_html_name_byte(bytes[cursor]) {
        cursor += 1;
    }

    while cursor < tag_end {
        let attr_start = cursor;
        cursor = skip_ascii_whitespace_until(bytes, cursor, tag_end);
        if cursor >= tag_end || bytes[cursor] == b'/' {
            break;
        }

        let name_start = cursor;
        while cursor < tag_end && is_html_attr_name_byte(bytes[cursor]) {
            cursor += 1;
        }
        let name_end = cursor;
        if name_start == name_end {
            cursor += 1;
            continue;
        }

        cursor = skip_ascii_whitespace_until(bytes, cursor, tag_end);
        if bytes.get(cursor) != Some(&b'=') {
            continue;
        }
        cursor = skip_ascii_whitespace_until(bytes, cursor + 1, tag_end);
        let Some(quote) = bytes.get(cursor).copied() else {
            break;
        };
        if !matches!(quote, b'\'' | b'"') {
            cursor = scan_unquoted_attr_value(bytes, cursor, tag_end);
            continue;
        }

        let quote_start = cursor;
        let value_start = quote_start + 1;
        let Some(value_end) = find_quoted_attr_end(bytes, value_start, tag_end, quote) else {
            break;
        };
        let attr_end = value_end + 1;
        if bytes[name_start..name_end].eq_ignore_ascii_case(b"style") {
            attrs.push(InlineStyleAttr {
                attr_start,
                name_start,
                name_end,
                value_start,
                value_end,
                attr_end,
            });
        }
        cursor = attr_end;
    }
}

fn find_opening_tag_end(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    let mut quote = None;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if let Some(active_quote) = quote {
            if byte == b'\\' {
                cursor = (cursor + 2).min(bytes.len());
                continue;
            }
            if byte == active_quote {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if byte == b'>' {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn find_quoted_attr_end(bytes: &[u8], mut cursor: usize, limit: usize, quote: u8) -> Option<usize> {
    while cursor < limit {
        if bytes[cursor] == b'\\' {
            cursor = (cursor + 2).min(limit);
            continue;
        }
        if bytes[cursor] == quote {
            return Some(cursor);
        }
        cursor += 1;
    }
    None
}

fn scan_unquoted_attr_value(bytes: &[u8], mut cursor: usize, limit: usize) -> usize {
    while cursor < limit && !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'/' {
        cursor += 1;
    }
    cursor
}

fn is_html_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'-' | b'_')
}

fn is_html_attr_name_byte(byte: u8) -> bool {
    !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'/' | b'>')
}

fn skip_ascii_whitespace_until(bytes: &[u8], mut idx: usize, limit: usize) -> usize {
    while idx < limit && matches!(bytes[idx], b' ' | b'\t' | b'\n' | b'\r') {
        idx += 1;
    }
    idx
}
