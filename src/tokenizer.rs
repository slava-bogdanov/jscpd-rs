use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::{
    config::TokensLexerConfig,
    lexer::{Kind, Lexer},
};
use oxc_span::SourceType;
use serde::Serialize;
use xxhash_rust::xxh3::xxh3_128;

use crate::cli::{Mode, Options};

#[derive(Clone, Debug, Serialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub position: usize,
}

#[derive(Clone, Debug)]
pub struct DetectionToken {
    pub hash: u128,
    pub start: Location,
    pub end: Location,
    pub range: [usize; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    Comment,
    Default,
}

#[derive(Clone, Copy)]
struct ByteSpan {
    start: usize,
    end: usize,
}

struct TokenContext<'a> {
    content: &'a str,
    options: &'a Options,
    ignore_regions: &'a [[usize; 2]],
}

impl TokenContext<'_> {
    fn slice(&self, span: ByteSpan) -> &str {
        &self.content[span.start..span.end]
    }

    fn overlaps_ignore_region(&self, span: ByteSpan) -> bool {
        self.ignore_regions
            .iter()
            .any(|[region_start, region_end]| span.start < *region_end && span.end > *region_start)
    }
}

pub fn tokenize_for_detection(
    content: &str,
    format: &str,
    options: &Options,
) -> Vec<DetectionToken> {
    let ignore_regions = find_ignore_regions(content, options);
    if is_oxc_format(format) {
        tokenize_oxc(content, format, options, &ignore_regions)
    } else {
        tokenize_generic(content, options, &ignore_regions)
    }
}

fn is_oxc_format(format: &str) -> bool {
    matches!(format, "javascript" | "typescript" | "jsx" | "tsx" | "json")
}

fn tokenize_generic(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<DetectionToken> {
    let context = TokenContext {
        content,
        options,
        ignore_regions,
    };
    let mut tokens = Vec::new();
    let mut line = 1usize;
    let mut column = 1usize;
    let mut chars = content.char_indices().peekable();

    while let Some((start_byte, ch)) = chars.next() {
        if ch.is_whitespace() {
            advance_position(ch, &mut line, &mut column);
            continue;
        }

        let start = Location {
            line,
            column,
            position: start_byte,
        };
        let mut end_byte = start_byte + ch.len_utf8();
        let mut end_line;
        let mut end_column;
        advance_position(ch, &mut line, &mut column);
        end_line = line;
        end_column = column;

        while let Some((next_byte, next_ch)) = chars.peek().copied() {
            if next_ch.is_whitespace() {
                break;
            }
            chars.next();
            advance_position(next_ch, &mut line, &mut column);
            end_byte = next_byte + next_ch.len_utf8();
            end_line = line;
            end_column = column;
        }

        let kind = if is_commentish(&content[start_byte..end_byte]) {
            TokenKind::Comment
        } else {
            TokenKind::Default
        };
        push_token(
            &mut tokens,
            &context,
            kind,
            ByteSpan {
                start: start_byte,
                end: end_byte,
            },
            start,
            Location {
                line: end_line,
                column: end_column,
                position: end_byte,
            },
        );
    }

    tokens
}

fn tokenize_oxc(
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
    let allocator = Allocator::new();
    let source_type = source_type_for_format(format);
    let mut lexer = Lexer::new_for_benchmarks(&allocator, content, source_type, TokensLexerConfig);
    let line_index = LineIndex::new(content);
    let mut tokens = Vec::with_capacity(content.len().saturating_div(6));
    let mut previous_end = 0usize;
    let mut token = lexer.first_token();

    while !token.kind().is_eof() {
        let start_byte = (token.start() as usize).min(content.len());
        let end_byte = (token.end() as usize).min(content.len());
        if start_byte > previous_end {
            push_comments_in_gap(&mut tokens, &context, previous_end, start_byte, &line_index);
        }
        push_oxc_token(
            &mut tokens,
            &context,
            token.kind(),
            ByteSpan {
                start: start_byte,
                end: end_byte,
            },
            &line_index,
        );
        previous_end = previous_end.max(end_byte);
        token = lexer.next_token_for_benchmarks();
    }

    if previous_end < content.len() {
        push_comments_in_gap(
            &mut tokens,
            &context,
            previous_end,
            content.len(),
            &line_index,
        );
    }

    tokens
}

fn source_type_for_format(format: &str) -> SourceType {
    let filename = match format {
        "typescript" => "input.ts",
        "tsx" => "input.tsx",
        "jsx" => "input.jsx",
        _ => "input.js",
    };
    SourceType::from_path(Path::new(filename)).unwrap_or_else(|_| SourceType::default())
}

fn push_oxc_token(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    kind: Kind,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    if kind == Kind::Skip || span.start >= span.end {
        return;
    }
    if context.overlaps_ignore_region(span) {
        return;
    }
    tokens.push(DetectionToken {
        hash: hash_oxc_token(kind, context.slice(span), context.options.ignore_case),
        start: line_index.location(span.start),
        end: line_index.location(span.end),
        range: [span.start, span.end],
    });
}

fn push_comments_in_gap(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    gap_start: usize,
    gap_end: usize,
    line_index: &LineIndex,
) {
    if context.options.mode == Mode::Weak || gap_start >= gap_end {
        return;
    }

    let bytes = context.content.as_bytes();
    let mut idx = gap_start;
    while idx + 1 < gap_end {
        let is_line_comment = (idx == 0 && bytes[idx] == b'#' && bytes[idx + 1] == b'!')
            || (bytes[idx] == b'/' && bytes[idx + 1] == b'/')
            || bytes[idx..gap_end].starts_with(b"<!--");
        let comment_end = if is_line_comment {
            Some(scan_line_comment(bytes, idx, gap_end))
        } else if bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            Some(scan_block_comment(bytes, idx, gap_end))
        } else {
            None
        };

        if let Some(comment_end) = comment_end {
            push_comment_token(
                tokens,
                context,
                ByteSpan {
                    start: idx,
                    end: comment_end,
                },
                line_index,
            );
            idx = comment_end.max(idx + 1);
        } else {
            idx += 1;
        }
    }
}

fn push_comment_token(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    if span.start >= span.end || context.overlaps_ignore_region(span) {
        return;
    }
    tokens.push(DetectionToken {
        hash: hash_token(
            TokenKind::Comment,
            context.slice(span),
            context.options.ignore_case,
        ),
        start: line_index.location(span.start),
        end: line_index.location(span.end),
        range: [span.start, span.end],
    });
}

fn push_token(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    kind: TokenKind,
    span: ByteSpan,
    start: Location,
    end: Location,
) {
    if context.options.mode == Mode::Weak && kind == TokenKind::Comment {
        return;
    }
    if context.overlaps_ignore_region(span) {
        return;
    }
    tokens.push(DetectionToken {
        hash: hash_token(kind, context.slice(span), context.options.ignore_case),
        start,
        end,
        range: [span.start, span.end],
    });
}

fn advance_position(ch: char, line: &mut usize, column: &mut usize) {
    if ch == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}

struct LineIndex {
    newlines: Vec<usize>,
}

impl LineIndex {
    fn new(content: &str) -> Self {
        Self {
            newlines: content
                .bytes()
                .enumerate()
                .filter_map(|(idx, byte)| (byte == b'\n').then_some(idx))
                .collect(),
        }
    }

    fn location(&self, offset: usize) -> Location {
        let previous_newlines = self
            .newlines
            .partition_point(|newline_offset| *newline_offset < offset);
        let line_start = if previous_newlines == 0 {
            0
        } else {
            self.newlines[previous_newlines - 1] + 1
        };
        Location {
            line: previous_newlines + 1,
            column: offset - line_start + 1,
            position: offset,
        }
    }
}

fn is_commentish(value: &str) -> bool {
    value.starts_with("//")
        || value.starts_with("/*")
        || value.starts_with('*')
        || value.starts_with('#')
        || value.starts_with("<!--")
}

fn hash_token(kind: TokenKind, value: &str, ignore_case: bool) -> u128 {
    let kind_hash = match kind {
        TokenKind::Comment => 0x01_u128,
        TokenKind::Default => 0x07_u128,
    };
    hash_value(value, ignore_case) ^ kind_hash
}

fn hash_oxc_token(kind: Kind, value: &str, ignore_case: bool) -> u128 {
    hash_value(value, ignore_case) ^ ((kind as u8 as u128) << 120)
}

fn hash_value(value: &str, ignore_case: bool) -> u128 {
    if ignore_case {
        xxh3_128(value.to_lowercase().as_bytes())
    } else {
        xxh3_128(value.as_bytes())
    }
}

fn find_ignore_regions(content: &str, options: &Options) -> Vec<[usize; 2]> {
    let mut regions = Vec::new();
    let start_marker = "jscpd:ignore-start";
    let end_marker = "jscpd:ignore-end";
    let mut search_from = 0;

    while let Some(marker_start) = content[search_from..].find(start_marker) {
        let marker_start = search_from + marker_start;
        let line_start = content[..marker_start]
            .rfind('\n')
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let after_start = marker_start + start_marker.len();
        let Some(marker_end_rel) = content[after_start..].find(end_marker) else {
            break;
        };
        let marker_end = after_start + marker_end_rel;
        let line_end = content[marker_end..]
            .find('\n')
            .map(|idx| marker_end + idx)
            .unwrap_or(content.len());
        regions.push([line_start, line_end]);
        search_from = line_end;
    }

    for pattern in &options.ignore_pattern {
        regions.extend(pattern.find_iter(content).map(|m| [m.start(), m.end()]));
    }

    regions
}

fn scan_line_comment(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start + 2;
    while idx < limit && bytes[idx] != b'\n' {
        idx += 1;
    }
    idx
}

fn scan_block_comment(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start + 2;
    while idx + 1 < limit {
        if bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
            return idx + 2;
        }
        idx += 1;
    }
    limit
}

#[cfg(test)]
mod tests {
    use crate::cli::Options;

    #[test]
    fn tokenizes_non_whitespace_tokens_with_locations() {
        let tokens = super::tokenize_for_detection(
            "let a = 1;\nlet b = 2;",
            "javascript",
            &Options::default(),
        );
        assert_eq!(tokens[0].start.line, 1);
        assert_eq!(tokens[5].start.line, 2);
    }

    #[test]
    fn skips_ignore_regions() {
        let content = "keep\n// jscpd:ignore-start\nskip\n// jscpd:ignore-end\nkeep2\n";
        let tokens = super::tokenize_for_detection(content, "javascript", &Options::default());
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn detection_tokenizer_avoids_token_value_allocations() {
        let tokens = super::tokenize_for_detection(
            "let a = 1;\nlet b = 2;",
            "javascript",
            &Options::default(),
        );
        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[0].start.line, 1);
        assert_eq!(tokens[5].start.line, 2);
    }

    #[test]
    fn weak_mode_skips_js_comments() {
        let options = Options {
            mode: crate::cli::Mode::Weak,
            ..Options::default()
        };
        let strong = super::tokenize_for_detection(
            "const a = 1; // comment\n",
            "javascript",
            &Options::default(),
        );
        let weak =
            super::tokenize_for_detection("const a = 1; // comment\n", "javascript", &options);
        assert!(strong.len() > weak.len());
    }
}
