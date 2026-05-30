use crate::cli::{Mode, Options};

use super::scan::scan_block_comment;
use super::{
    ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, push_strict_whitespace_tokens,
    push_token,
};

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
            let whitespace_end = scan_whitespace(content, start_byte);
            if options.mode == Mode::Strict {
                push_strict_whitespace_tokens(
                    &mut tokens,
                    &context,
                    ByteSpan {
                        start: start_byte,
                        end: whitespace_end,
                    },
                    &line_index,
                );
            }
            start_byte = whitespace_end.max(start_byte + ch.len_utf8());
            continue;
        }

        let (end_byte, kind) = if let Some((special_end, special_kind)) =
            generic_multiline_span_end(content, format, start_byte, content.len())
        {
            (special_end, special_kind)
        } else if let Some(comment_end) =
            generic_comment_span_end(content, format, start_byte, content.len())
        {
            (comment_end, TokenKind::Comment)
        } else if punctuation_split_format(format) {
            scan_punctuation_split_token(content, format, start_byte)
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

fn scan_punctuation_split_token(content: &str, format: &str, start: usize) -> (usize, TokenKind) {
    let ch = content[start..].chars().next().unwrap_or('\0');
    if is_split_punctuation(format, ch) {
        return (start + ch.len_utf8(), TokenKind::Punctuation);
    }
    if code_like_format(format) && is_operator_start(ch) {
        return (scan_operator_token(content, start), TokenKind::Operator);
    }

    let mut end = start;
    while end < content.len() {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if ch.is_whitespace()
            || is_split_punctuation(format, ch)
            || (code_like_format(format) && is_operator_start(ch))
        {
            break;
        }
        end += ch.len_utf8();
    }
    (end, TokenKind::Default)
}

fn scan_operator_token(content: &str, start: usize) -> usize {
    let mut end = start;
    while end < content.len() {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if !is_operator_start(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    end
}

fn generic_multiline_span_end(
    content: &str,
    format: &str,
    start: usize,
    limit: usize,
) -> Option<(usize, TokenKind)> {
    match format {
        "haml" => haml_multiline_comment_span_end(content, start, limit)
            .map(|end| (end, TokenKind::Comment)),
        "pug" => pug_dot_block_span_end(content, start, limit).map(|end| (end, TokenKind::Default)),
        _ => None,
    }
}

fn haml_multiline_comment_span_end(content: &str, start: usize, limit: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let line_start = line_start(bytes, start);
    if !line_prefix_is_indent(bytes, line_start, start) {
        return None;
    }

    let rest = &bytes[start..limit];
    if !(rest.starts_with(b"-#") || rest.starts_with(b"/")) {
        return None;
    }

    Some(scan_indented_block_end(
        bytes, line_start, start, limit, false,
    ))
}

fn pug_dot_block_span_end(content: &str, start: usize, limit: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let line_start = line_start(bytes, start);
    if !line_prefix_is_indent(bytes, line_start, start) {
        return None;
    }

    let line_end = line_content_end(bytes, start, limit);
    if !is_pug_dot_block_opener(&content[start..line_end]) {
        return None;
    }

    let end = scan_indented_block_end(bytes, line_start, start, limit, true);
    (end > line_end).then_some(end)
}

fn scan_indented_block_end(
    bytes: &[u8],
    line_start: usize,
    start: usize,
    limit: usize,
    include_blank_lines: bool,
) -> usize {
    let base_indent = start.saturating_sub(line_start);
    let mut end = line_content_end(bytes, start, limit);
    let mut next_start = next_line_start(bytes, end, limit);

    while next_start < limit {
        let line_end = line_content_end(bytes, next_start, limit);
        let indent_end = scan_indent(bytes, next_start, line_end);
        let is_blank = indent_end == line_end;
        let is_child = indent_end.saturating_sub(next_start) > base_indent;
        if is_child || (include_blank_lines && is_blank) {
            end = line_end;
            next_start = next_line_start(bytes, line_end, limit);
        } else {
            break;
        }
    }

    end
}

fn is_pug_dot_block_opener(line: &str) -> bool {
    let trimmed = line.trim_end_matches([' ', '\t']);
    let Some(head) = trimmed.strip_suffix('.') else {
        return false;
    };
    !head.eq_ignore_ascii_case("script")
        && !head.is_empty()
        && head
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'#' | b'.'))
}

fn is_split_punctuation(format: &str, ch: char) -> bool {
    matches!(ch, '{' | '}' | '(' | ')' | '[' | ']' | ':' | ';' | ',')
        || (code_like_format(format) && ch == '.')
}

fn is_operator_start(ch: char) -> bool {
    matches!(
        ch,
        '+' | '-' | '*' | '/' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '^' | '~' | '?'
    )
}

pub(super) fn scan_whitespace(content: &str, start: usize) -> usize {
    let mut end = start;
    while end < content.len() {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if !ch.is_whitespace() {
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
    if rest.starts_with(b"--") && generic_double_dash_comment_format(format) {
        return Some(scan_to_line_end(bytes, start, limit));
    }
    if bytes[start] == b'#' && generic_hash_comment_format(format) {
        return Some(scan_to_line_end(bytes, start, limit));
    }
    if bytes[start] == b';' && generic_semicolon_comment_format(format) {
        return Some(scan_to_line_end(bytes, start, limit));
    }
    None
}

fn generic_hash_comment_format(format: &str) -> bool {
    matches!(
        format,
        "apacheconf"
            | "applescript"
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

fn generic_double_dash_comment_format(format: &str) -> bool {
    matches!(
        format,
        "ada" | "applescript" | "elm" | "haskell" | "lua" | "plsql" | "sql"
    )
}

fn generic_semicolon_comment_format(format: &str) -> bool {
    matches!(
        format,
        "asm6502"
            | "autoit"
            | "autohotkey"
            | "clojure"
            | "ini"
            | "lisp"
            | "llvm"
            | "nasm"
            | "racket"
            | "scheme"
    )
}

fn punctuation_split_format(format: &str) -> bool {
    css_like_format(format) || code_like_format(format)
}

fn css_like_format(format: &str) -> bool {
    matches!(format, "css" | "less" | "sass" | "scss" | "stylus")
}

fn code_like_format(format: &str) -> bool {
    matches!(
        format,
        "ada"
            | "apex"
            | "aspnet"
            | "c"
            | "c-header"
            | "clike"
            | "cpp"
            | "cpp-header"
            | "csharp"
            | "cfml"
            | "cfscript"
            | "dart"
            | "eiffel"
            | "go"
            | "java"
            | "kotlin"
            | "haxe"
            | "objectivec"
            | "ocaml"
            | "perl"
            | "php"
            | "plsql"
            | "properties"
            | "purescript"
            | "python"
            | "r"
            | "rescript"
            | "rust"
            | "scala"
            | "solidity"
            | "swift"
            | "tcl"
            | "tt2"
            | "turtle"
            | "twig"
            | "verilog"
            | "wgsl"
            | "zig"
    )
}

fn scan_to_line_end(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start;
    while idx < limit && bytes[idx] != b'\n' {
        idx += 1;
    }
    idx
}

fn line_start(bytes: &[u8], start: usize) -> usize {
    let mut idx = start;
    while idx > 0 && !matches!(bytes[idx - 1], b'\n' | b'\r') {
        idx -= 1;
    }
    idx
}

fn line_prefix_is_indent(bytes: &[u8], line_start: usize, start: usize) -> bool {
    bytes[line_start..start]
        .iter()
        .all(|byte| matches!(byte, b' ' | b'\t'))
}

fn line_content_end(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start;
    while idx < limit && !matches!(bytes[idx], b'\n' | b'\r') {
        idx += 1;
    }
    idx
}

fn next_line_start(bytes: &[u8], line_end: usize, limit: usize) -> usize {
    if line_end >= limit {
        return limit;
    }
    if bytes[line_end] == b'\r' && line_end + 1 < limit && bytes[line_end + 1] == b'\n' {
        line_end + 2
    } else {
        line_end + 1
    }
}

fn scan_indent(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start;
    while idx < limit && matches!(bytes[idx], b' ' | b'\t') {
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
