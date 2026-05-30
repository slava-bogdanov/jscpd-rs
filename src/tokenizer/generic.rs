use crate::cli::Options;

use super::scan::scan_block_comment;
use super::{ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, push_token};

pub(super) fn tokenize_generic(
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
        if ch.is_whitespace() {
            start_byte += ch.len_utf8();
            continue;
        }

        let (end_byte, kind) = if let Some(comment_end) =
            generic_comment_span_end(content, format, start_byte, content.len())
        {
            (comment_end, TokenKind::Comment)
        } else {
            (scan_generic_token(content, start_byte), TokenKind::Default)
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

pub(super) fn scan_generic_token(content: &str, start: usize) -> usize {
    let mut end = start;
    while end < content.len() {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    end
}

pub(super) fn generic_comment_span_end(
    content: &str,
    format: &str,
    start: usize,
    limit: usize,
) -> Option<usize> {
    let bytes = content.as_bytes();
    let rest = &bytes[start..limit];
    if rest.starts_with(b"<!--") {
        return Some(scan_html_comment(bytes, start, limit));
    }
    if rest.starts_with(b"/*") {
        return Some(scan_block_comment(bytes, start, limit));
    }
    if rest.starts_with(b"//") {
        return Some(scan_to_line_end(bytes, start, limit));
    }
    if bytes[start] == b'#' && generic_hash_comment_format(format) {
        return Some(scan_to_line_end(bytes, start, limit));
    }
    None
}

fn generic_hash_comment_format(format: &str) -> bool {
    matches!(
        format,
        "apacheconf"
            | "bash"
            | "cmake"
            | "docker"
            | "editorconfig"
            | "git"
            | "ignore"
            | "ini"
            | "julia"
            | "makefile"
            | "nginx"
            | "nix"
            | "perl"
            | "powershell"
            | "properties"
            | "python"
            | "r"
            | "ruby"
            | "shell-session"
            | "tcl"
            | "toml"
            | "vim"
            | "yaml"
    )
}

fn scan_to_line_end(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start;
    while idx < limit && bytes[idx] != b'\n' {
        idx += 1;
    }
    idx
}

fn scan_html_comment(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start + 4;
    while idx + 2 < limit {
        if bytes[idx] == b'-' && bytes[idx + 1] == b'-' && bytes[idx + 2] == b'>' {
            return idx + 3;
        }
        idx += 1;
    }
    limit
}
