use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::{Kind, Parser, config::TokensParserConfig};
use oxc_span::SourceType;
use serde::Serialize;
use xxhash_rust::xxh3::xxh3_64;

use crate::cli::{Mode, Options};

#[derive(Clone, Debug, Serialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub position: usize,
}

#[derive(Clone, Debug)]
pub struct DetectionToken {
    pub hash: u64,
    pub start: Location,
    pub end: Location,
    pub range: [usize; 2],
}

#[derive(Clone, Debug)]
pub struct TokenMap {
    pub format: String,
    pub tokens: Vec<DetectionToken>,
    positions_assigned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    Comment,
    Constant,
    Keyword,
    Number,
    Operator,
    Punctuation,
    String,
    Default,
}

#[derive(Clone, Copy)]
struct ByteSpan {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct RawOxcToken {
    kind: Kind,
    span: ByteSpan,
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

#[cfg(test)]
fn tokenize_for_detection(content: &str, format: &str, options: &Options) -> Vec<DetectionToken> {
    tokenize_maps_for_detection(content, format, options)
        .into_iter()
        .next()
        .map(|map| map.tokens)
        .unwrap_or_default()
}

pub fn tokenize_maps_for_detection(
    content: &str,
    format: &str,
    options: &Options,
) -> Vec<TokenMap> {
    let ignore_regions = find_ignore_regions(content, options);
    let mut maps = if is_oxc_format(format) {
        tokenize_oxc_maps(content, format, options, &ignore_regions)
    } else {
        vec![TokenMap {
            format: format.to_string(),
            tokens: tokenize_generic(content, format, options, &ignore_regions),
            positions_assigned: false,
        }]
    };
    for map in &mut maps {
        if !map.positions_assigned {
            assign_token_positions(content, &map.format, options, &mut map.tokens);
        }
    }
    maps
}

fn assign_token_positions(
    content: &str,
    format: &str,
    options: &Options,
    tokens: &mut [DetectionToken],
) {
    let needs_report_positions =
        options.reporters.iter().any(|reporter| reporter == "json") || !options.silent;
    if !needs_report_positions || !matches!(format, "javascript" | "typescript" | "jsx" | "tsx") {
        for (position, token) in tokens.iter_mut().enumerate() {
            token.start.position = position;
            token.end.position = position;
        }
        return;
    }

    let mut position = 0usize;
    let mut previous_end = 0usize;
    for token in tokens {
        if token.range[0] > previous_end {
            position += count_prism_whitespace_tokens(content, previous_end, token.range[0]);
        }
        token.start.position = position;
        token.end.position = position;
        position += 1;
        previous_end = previous_end.max(token.range[1]);
    }
}

fn is_oxc_format(format: &str) -> bool {
    matches!(format, "javascript" | "typescript" | "jsx" | "tsx" | "json")
}

fn tokenize_generic(
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

fn tokenize_oxc_maps(
    content: &str,
    format: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let context = TokenContext {
        content,
        options,
        ignore_regions,
    };
    let allocator = Allocator::new();
    let source_type = source_type_for_format(format);
    let parser_return = Parser::new(&allocator, content, source_type)
        .with_config(TokensParserConfig)
        .parse();
    let line_index = LineIndex::new(content);
    let mut tokens = Vec::with_capacity(content.len().saturating_div(6));
    let mut previous_end = 0usize;
    let parser_tokens = parser_return
        .tokens
        .iter()
        .map(|token| RawOxcToken {
            kind: token.kind(),
            span: ByteSpan {
                start: (token.start() as usize).min(content.len()),
                end: (token.end() as usize).min(content.len()),
            },
        })
        .collect::<Vec<_>>();
    let jsx_script_groups = if matches!(format, "jsx" | "tsx") {
        jsx_attribute_script_groups(&parser_tokens)
    } else {
        Vec::new()
    };
    let mut idx = 0usize;

    while idx < parser_tokens.len() {
        let token = &parser_tokens[idx];
        let start_byte = token.span.start;
        let mut end_byte = token.span.end;
        if start_byte > previous_end {
            push_comments_in_gap(&mut tokens, &context, previous_end, start_byte, &line_index);
        }
        if token.kind == Kind::RAngle {
            while idx + 1 < parser_tokens.len() {
                let next = &parser_tokens[idx + 1];
                if next.kind != Kind::RAngle || next.span.start != end_byte {
                    break;
                }
                idx += 1;
                end_byte = next.span.end;
            }
        }
        push_oxc_token(
            &mut tokens,
            &context,
            token.kind,
            ByteSpan {
                start: start_byte,
                end: end_byte,
            },
            &line_index,
        );
        previous_end = previous_end.max(end_byte);
        idx += 1;
    }

    if previous_end < content.len() {
        if has_code_in_gap(content, previous_end, content.len()) {
            tokenize_js_like_range(
                &mut tokens,
                &context,
                previous_end,
                content.len(),
                &line_index,
            );
        } else {
            push_comments_in_gap(
                &mut tokens,
                &context,
                previous_end,
                content.len(),
                &line_index,
            );
        }
    }

    let mut maps = vec![TokenMap {
        format: format.to_string(),
        tokens,
        positions_assigned: false,
    }];
    if matches!(format, "jsx" | "tsx") {
        let embedded = tokenize_jsx_attribute_scripts(
            &parser_tokens,
            &jsx_script_groups,
            &context,
            &line_index,
        );
        if !embedded.is_empty() {
            maps.push(TokenMap {
                format: "javascript".to_string(),
                tokens: embedded,
                positions_assigned: true,
            });
        }
    }
    maps
}

fn tokenize_jsx_attribute_scripts(
    parser_tokens: &[RawOxcToken],
    groups: &[(usize, usize)],
    context: &TokenContext<'_>,
    line_index: &LineIndex,
) -> Vec<DetectionToken> {
    let mut tokens = Vec::new();
    let mut next_position = 0usize;
    let mut previous_group_end = None;

    for &(group_start_idx, group_end_idx) in groups {
        let group_start = parser_tokens[group_start_idx].span.start;
        if let Some(previous_end) = previous_group_end {
            next_position += count_embedded_gap_positions(
                context.content,
                parser_tokens,
                previous_end,
                group_start,
            );
        }
        let mut expression_depth = 0usize;
        let mut previous_token_end = None;
        for raw in &parser_tokens[group_start_idx..=group_end_idx] {
            let before = tokens.len();
            // Prism keeps default whitespace string tokens inside nested JSX
            // script objects, and those tokens can decide minTokens windows.
            if expression_depth >= 2
                && let Some(gap_start) = previous_token_end
            {
                push_embedded_default_gap(
                    &mut tokens,
                    context,
                    gap_start,
                    raw.span.start,
                    line_index,
                );
            }
            push_oxc_token(&mut tokens, context, raw.kind, raw.span, line_index);
            for pushed in &mut tokens[before..] {
                pushed.start.position = next_position;
                pushed.end.position = next_position;
                next_position += 1;
            }
            match raw.kind {
                Kind::LCurly => expression_depth += 1,
                Kind::RCurly => expression_depth = expression_depth.saturating_sub(1),
                _ => {}
            }
            previous_token_end = Some(raw.span.end);
        }
        previous_group_end = Some(parser_tokens[group_end_idx].span.end);
    }

    tokens
}

fn push_embedded_default_gap(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    gap_start: usize,
    gap_end: usize,
    line_index: &LineIndex,
) {
    if gap_start >= gap_end {
        return;
    }
    if !context.content[gap_start..gap_end]
        .chars()
        .all(char::is_whitespace)
    {
        return;
    }
    push_token_part(
        tokens,
        context,
        TokenKind::Default,
        ByteSpan {
            start: gap_start,
            end: gap_end,
        },
        line_index,
    );
}

fn jsx_attribute_script_groups(parser_tokens: &[RawOxcToken]) -> Vec<(usize, usize)> {
    let mut groups = Vec::new();
    let mut in_jsx_tag = false;
    let mut idx = 0usize;

    while idx < parser_tokens.len() {
        let token = parser_tokens[idx];
        if !in_jsx_tag && token.kind == Kind::LAngle && looks_like_jsx_tag_start(parser_tokens, idx)
        {
            in_jsx_tag = true;
            idx += 1;
            continue;
        }
        if in_jsx_tag && token.kind == Kind::RAngle {
            in_jsx_tag = false;
            idx += 1;
            continue;
        }
        if in_jsx_tag
            && token.kind == Kind::Eq
            && parser_tokens
                .get(idx + 1)
                .is_some_and(|next| next.kind == Kind::LCurly)
            && let Some(group_end_idx) = jsx_attribute_expression_end(parser_tokens, idx + 1)
        {
            groups.push((idx, group_end_idx));
            idx = group_end_idx + 1;
            continue;
        }
        idx += 1;
    }

    groups
}

fn looks_like_jsx_tag_start(parser_tokens: &[RawOxcToken], idx: usize) -> bool {
    matches!(
        parser_tokens.get(idx + 1).map(|token| token.kind),
        Some(Kind::Ident) | Some(Kind::This) | Some(Kind::PrivateIdentifier)
    ) || matches!(
        (
            parser_tokens.get(idx + 1).map(|token| token.kind),
            parser_tokens.get(idx + 2).map(|token| token.kind),
        ),
        (Some(Kind::Slash), Some(Kind::Ident))
    )
}

fn jsx_attribute_expression_end(parser_tokens: &[RawOxcToken], lcurly_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, token) in parser_tokens.iter().enumerate().skip(lcurly_idx) {
        match token.kind {
            Kind::LCurly => depth += 1,
            Kind::RCurly => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn count_embedded_gap_positions(
    content: &str,
    parser_tokens: &[RawOxcToken],
    gap_start: usize,
    gap_end: usize,
) -> usize {
    count_prism_whitespace_tokens(content, gap_start, gap_end)
        + parser_tokens
            .iter()
            .filter(|token| token.span.start >= gap_start && token.span.end <= gap_end)
            .filter(|token| token.kind != Kind::Skip)
            .count()
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
    if matches!(
        kind,
        Kind::TemplateHead | Kind::TemplateMiddle | Kind::TemplateTail
    ) {
        push_template_token_parts(tokens, context, kind, span, line_index);
        return;
    }
    if kind == Kind::QuestionDot && context.slice(span) == "?." {
        push_token_part(
            tokens,
            context,
            TokenKind::Operator,
            ByteSpan {
                start: span.start,
                end: span.start + 1,
            },
            line_index,
        );
        push_token_part(
            tokens,
            context,
            TokenKind::Punctuation,
            ByteSpan {
                start: span.start + 1,
                end: span.end,
            },
            line_index,
        );
        return;
    }
    if context.overlaps_ignore_region(span) {
        return;
    }
    tokens.push(DetectionToken {
        hash: hash_token(
            oxc_token_kind(kind, context.slice(span)),
            context.slice(span),
            context.options.ignore_case,
        ),
        start: line_index.location(span.start),
        end: line_index.location(span.end),
        range: [span.start, span.end],
    });
}

fn push_template_token_parts(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    kind: Kind,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    match kind {
        Kind::TemplateHead => {
            let interpolation_start = span.end.saturating_sub(2);
            push_token_part(
                tokens,
                context,
                TokenKind::String,
                ByteSpan {
                    start: span.start,
                    end: interpolation_start,
                },
                line_index,
            );
            push_token_part(
                tokens,
                context,
                TokenKind::Punctuation,
                ByteSpan {
                    start: interpolation_start,
                    end: span.end,
                },
                line_index,
            );
        }
        Kind::TemplateMiddle => {
            push_token_part(
                tokens,
                context,
                TokenKind::Punctuation,
                ByteSpan {
                    start: span.start,
                    end: span.start.saturating_add(1),
                },
                line_index,
            );
            let interpolation_start = span.end.saturating_sub(2);
            push_token_part(
                tokens,
                context,
                TokenKind::String,
                ByteSpan {
                    start: span.start.saturating_add(1),
                    end: interpolation_start,
                },
                line_index,
            );
            push_token_part(
                tokens,
                context,
                TokenKind::Punctuation,
                ByteSpan {
                    start: interpolation_start,
                    end: span.end,
                },
                line_index,
            );
        }
        Kind::TemplateTail => {
            push_token_part(
                tokens,
                context,
                TokenKind::Punctuation,
                ByteSpan {
                    start: span.start,
                    end: span.start.saturating_add(1),
                },
                line_index,
            );
            push_token_part(
                tokens,
                context,
                TokenKind::String,
                ByteSpan {
                    start: span.start.saturating_add(1),
                    end: span.end,
                },
                line_index,
            );
        }
        _ => {}
    }
}

fn push_token_part(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    kind: TokenKind,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    if span.start >= span.end || context.overlaps_ignore_region(span) {
        return;
    }
    push_token(
        tokens,
        context,
        kind,
        span,
        line_index.location(span.start),
        line_index.location(span.end),
    );
}

fn tokenize_js_like_range(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    range_start: usize,
    range_end: usize,
    line_index: &LineIndex,
) {
    let bytes = context.content.as_bytes();
    let mut idx = range_start;

    while idx < range_end {
        let ch = context.content[idx..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            idx += ch.len_utf8();
            continue;
        }

        let (end, kind) = if idx + 1 < range_end && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            (scan_line_comment(bytes, idx, range_end), TokenKind::Comment)
        } else if idx + 1 < range_end && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            (
                scan_block_comment(bytes, idx, range_end),
                TokenKind::Comment,
            )
        } else if matches!(bytes[idx], b'\'' | b'"' | b'`') {
            (
                scan_string(bytes, idx, bytes[idx], range_end),
                TokenKind::String,
            )
        } else if is_identifier_start(ch) {
            let end = scan_identifier(context.content, idx, range_end);
            let value = &context.content[idx..end];
            let kind = if is_js_constant(value) {
                TokenKind::Constant
            } else if is_js_keyword(value) {
                TokenKind::Keyword
            } else {
                TokenKind::Default
            };
            (end, kind)
        } else if bytes[idx].is_ascii_digit() {
            (scan_number(bytes, idx, range_end), TokenKind::Number)
        } else {
            scan_operator_or_punctuation(bytes, idx, range_end)
        };

        push_token(
            tokens,
            context,
            kind,
            ByteSpan { start: idx, end },
            line_index.location(idx),
            line_index.location(end),
        );
        idx = end.max(idx + 1);
    }
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

fn scan_generic_token(content: &str, start: usize) -> usize {
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

fn generic_comment_span_end(
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

fn hash_token(kind: TokenKind, value: &str, ignore_case: bool) -> u64 {
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

fn token_kind_for_oxc(kind: Kind) -> TokenKind {
    if kind.is_number() {
        return TokenKind::Number;
    }
    if matches!(
        kind,
        Kind::Str
            | Kind::NoSubstitutionTemplate
            | Kind::TemplateHead
            | Kind::TemplateMiddle
            | Kind::TemplateTail
            | Kind::RegExp
    ) {
        return TokenKind::String;
    }
    if is_oxc_keyword(kind) {
        return TokenKind::Keyword;
    }
    if is_oxc_punctuation(kind) {
        return TokenKind::Punctuation;
    }
    if is_oxc_operator(kind) {
        return TokenKind::Operator;
    }
    TokenKind::Default
}

fn oxc_token_kind(kind: Kind, value: &str) -> TokenKind {
    if kind == Kind::Ident && is_js_constant(value) {
        TokenKind::Constant
    } else {
        token_kind_for_oxc(kind)
    }
}

fn is_oxc_keyword(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Await
            | Kind::Break
            | Kind::Case
            | Kind::Catch
            | Kind::Class
            | Kind::Const
            | Kind::Continue
            | Kind::Debugger
            | Kind::Default
            | Kind::Delete
            | Kind::Do
            | Kind::Else
            | Kind::Enum
            | Kind::Export
            | Kind::Extends
            | Kind::Finally
            | Kind::For
            | Kind::Function
            | Kind::If
            | Kind::Import
            | Kind::In
            | Kind::Instanceof
            | Kind::New
            | Kind::Return
            | Kind::Super
            | Kind::Switch
            | Kind::This
            | Kind::Throw
            | Kind::Try
            | Kind::Typeof
            | Kind::Var
            | Kind::Void
            | Kind::While
            | Kind::With
            | Kind::Async
            | Kind::From
            | Kind::Get
            | Kind::Of
            | Kind::Set
            | Kind::As
            | Kind::Type
            | Kind::Undefined
            | Kind::Implements
            | Kind::Interface
            | Kind::Let
            | Kind::Package
            | Kind::Private
            | Kind::Protected
            | Kind::Public
            | Kind::Static
            | Kind::Yield
            | Kind::True
            | Kind::False
            | Kind::Null
    )
}

fn is_oxc_punctuation(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Colon
            | Kind::Comma
            | Kind::Dot
            | Kind::Dot3
            | Kind::LBrack
            | Kind::LCurly
            | Kind::LParen
            | Kind::RBrack
            | Kind::RCurly
            | Kind::RParen
            | Kind::Semicolon
    )
}

fn is_oxc_operator(kind: Kind) -> bool {
    !matches!(kind, Kind::Ident | Kind::PrivateIdentifier | Kind::JSXText)
        && !matches!(token_kind_for_operator_check(kind), TokenKind::Default)
}

fn token_kind_for_operator_check(kind: Kind) -> TokenKind {
    if matches!(
        kind,
        Kind::Amp
            | Kind::Amp2
            | Kind::Amp2Eq
            | Kind::AmpEq
            | Kind::Bang
            | Kind::Caret
            | Kind::CaretEq
            | Kind::Eq
            | Kind::Eq2
            | Kind::Eq3
            | Kind::GtEq
            | Kind::LAngle
            | Kind::LtEq
            | Kind::Minus
            | Kind::Minus2
            | Kind::MinusEq
            | Kind::Neq
            | Kind::Neq2
            | Kind::Percent
            | Kind::PercentEq
            | Kind::Pipe
            | Kind::Pipe2
            | Kind::Pipe2Eq
            | Kind::PipeEq
            | Kind::Plus
            | Kind::Plus2
            | Kind::PlusEq
            | Kind::Question
            | Kind::Question2
            | Kind::Question2Eq
            | Kind::QuestionDot
            | Kind::RAngle
            | Kind::ShiftLeft
            | Kind::ShiftLeftEq
            | Kind::ShiftRight
            | Kind::ShiftRight3
            | Kind::ShiftRight3Eq
            | Kind::ShiftRightEq
            | Kind::Slash
            | Kind::SlashEq
            | Kind::Star
            | Kind::Star2
            | Kind::Star2Eq
            | Kind::StarEq
            | Kind::Tilde
            | Kind::Arrow
    ) {
        TokenKind::Operator
    } else {
        TokenKind::Default
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

fn has_code_in_gap(content: &str, start: usize, end: usize) -> bool {
    let bytes = content.as_bytes();
    let mut idx = start;
    while idx < end {
        let ch = content[idx..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            idx += ch.len_utf8();
        } else if idx + 1 < end && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            idx = scan_line_comment(bytes, idx, end);
        } else if idx + 1 < end && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            idx = scan_block_comment(bytes, idx, end);
        } else {
            return true;
        }
    }
    false
}

fn count_prism_whitespace_tokens(content: &str, start: usize, end: usize) -> usize {
    let bytes = content.as_bytes();
    let mut idx = start;
    let mut count = 0usize;

    while idx < end {
        match bytes[idx] {
            b'\n' => {
                count += 1;
                idx += 1;
            }
            b' ' | b'\t' | b'\r' | b'\x0c' | b'\x0b' => {
                count += 1;
                idx += 1;
                while idx < end && matches!(bytes[idx], b' ' | b'\t' | b'\r' | b'\x0c' | b'\x0b') {
                    idx += 1;
                }
            }
            _ => idx += 1,
        }
    }

    count
}

fn scan_string(bytes: &[u8], start: usize, quote: u8, limit: usize) -> usize {
    let mut idx = start + 1;
    while idx < limit {
        if bytes[idx] == b'\\' {
            idx = (idx + 2).min(limit);
            continue;
        }
        if bytes[idx] == quote {
            return idx + 1;
        }
        idx += 1;
    }
    limit
}

fn scan_identifier(content: &str, start: usize, limit: usize) -> usize {
    let mut idx = start;
    while idx < limit {
        let ch = content[idx..].chars().next().unwrap_or('\0');
        if !is_identifier_continue(ch) {
            break;
        }
        idx += ch.len_utf8();
    }
    idx
}

fn scan_number(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start;
    while idx < limit
        && (bytes[idx].is_ascii_alphanumeric() || matches!(bytes[idx], b'.' | b'_' | b'+' | b'-'))
    {
        idx += 1;
    }
    idx
}

fn scan_operator_or_punctuation(bytes: &[u8], start: usize, limit: usize) -> (usize, TokenKind) {
    const OPERATORS: &[&[u8]] = &[
        b">>>=", b"===", b"!==", b">>>", b"<<=", b">>=", b"**=", b"=>", b"==", b"!=", b"<=", b">=",
        b"++", b"--", b"&&", b"||", b"??", b"?.", b"...", b"+=", b"-=", b"*=", b"/=", b"%=", b"&=",
        b"|=", b"^=", b"<<", b">>", b"**",
    ];
    for operator in OPERATORS {
        if bytes[start..limit].starts_with(operator) {
            return (start + operator.len(), TokenKind::Operator);
        }
    }
    let kind = if matches!(
        bytes[start],
        b'{' | b'}' | b'[' | b']' | b'(' | b')' | b';' | b',' | b':' | b'.'
    ) {
        TokenKind::Punctuation
    } else {
        TokenKind::Operator
    };
    (start + 1, kind)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic() || (ch as u32) > 0x7f
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

fn is_js_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "async"
            | "await"
            | "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "from"
            | "function"
            | "get"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "interface"
            | "let"
            | "new"
            | "null"
            | "of"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "set"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "type"
            | "typeof"
            | "undefined"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
    )
}

fn is_js_constant(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase()
        && chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
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
    fn js_like_json_report_positions_count_prism_whitespace_tokens() {
        let options = Options {
            reporters: vec!["json".to_string()],
            ..Options::default()
        };
        for format in ["javascript", "typescript", "jsx", "tsx"] {
            let tokens = super::tokenize_for_detection("let a = 1;\nlet b = 2;", format, &options);
            assert_eq!(tokens[0].start.position, 0);
            assert_eq!(tokens[1].start.position, 2);
            assert_eq!(tokens[5].start.position, 9);
        }
    }

    #[test]
    fn jsx_attribute_expression_emits_embedded_javascript_map() {
        let maps = super::tokenize_maps_for_detection(
            "const x = <div className={classNames(className, classes)} />;",
            "jsx",
            &Options::default(),
        );
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].format, "jsx");
        assert_eq!(maps[1].format, "javascript");

        let embedded = &maps[1].tokens;
        assert_eq!(embedded.len(), 9);
        assert_eq!(
            embedded.last().unwrap().end.position - embedded.first().unwrap().start.position,
            8
        );
    }

    #[test]
    fn jsx_embedded_javascript_keeps_nested_object_whitespace() {
        let content = "const x = <A p={{\n  color: PRIMARY_COLOR\n}} />;";
        let maps = super::tokenize_maps_for_detection(content, "tsx", &Options::default());
        let embedded = maps
            .iter()
            .find(|map| map.format == "javascript")
            .expect("embedded javascript map");

        assert!(
            embedded
                .tokens
                .iter()
                .any(|token| &content[token.range[0]..token.range[1]] == "\n  ")
        );
    }

    #[test]
    fn generic_tokenizer_handles_common_non_native_formats() {
        for format in ["css", "markup", "yaml", "toml", "vue"] {
            let maps = super::tokenize_maps_for_detection(
                "alpha beta\n  gamma",
                format,
                &Options::default(),
            );

            assert_eq!(maps.len(), 1);
            assert_eq!(maps[0].format, format);
            assert_eq!(maps[0].tokens.len(), 3);
        }
    }

    #[test]
    fn weak_mode_skips_generic_comments() {
        let content = "# first comment\nalpha beta\n// second comment\ngamma\n";
        let weak_options = Options {
            mode: crate::cli::Mode::Weak,
            ..Options::default()
        };

        let strong = super::tokenize_for_detection(content, "yaml", &Options::default());
        let weak = super::tokenize_for_detection(content, "yaml", &weak_options);

        assert_eq!(strong.len(), 5);
        assert_eq!(weak.len(), 3);
    }

    #[test]
    fn generic_css_ids_are_not_treated_as_hash_comments() {
        let options = Options {
            mode: crate::cli::Mode::Weak,
            ..Options::default()
        };
        let tokens = super::tokenize_for_detection("#app .title\n", "css", &options);

        assert_eq!(tokens.len(), 2);
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

    #[test]
    fn splits_template_interpolation_like_prism() {
        let tokens = super::tokenize_for_detection(
            "const x = `a${b}c${d}e`;",
            "typescript",
            &Options::default(),
        );
        assert_eq!(tokens.len(), 13);
        assert_eq!(tokens[3].start.column, 11);
        assert_eq!(tokens[4].start.column, 13);
        assert_eq!(tokens[6].start.column, 16);
        assert_eq!(tokens[8].start.column, 18);
        assert_eq!(tokens[10].start.column, 21);
        assert_eq!(tokens[11].start.column, 22);
    }

    #[test]
    fn splits_optional_chaining_like_prism() {
        let tokens = super::tokenize_for_detection("a?.b", "typescript", &Options::default());
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[1].start.column, 2);
        assert_eq!(tokens[2].start.column, 3);
        assert_eq!(tokens[3].start.column, 4);
    }

    #[test]
    fn merges_adjacent_generic_closing_angles_like_prism() {
        let tokens =
            super::tokenize_for_detection("type A = X<Y<Z>>;", "typescript", &Options::default());
        assert_eq!(tokens.len(), 10);
        assert_eq!(tokens[8].start.column, 15);
        assert_eq!(tokens[8].end.column, 17);
        assert_eq!(tokens[9].start.column, 17);
    }

    #[test]
    fn weak_mode_skips_generic_markup_comments() {
        let content = "<!-- comment -->\nalpha beta\n<!-- another -->\ngamma\n";
        let weak_options = Options {
            mode: crate::cli::Mode::Weak,
            ..Options::default()
        };

        let strong = super::tokenize_for_detection(content, "markup", &Options::default());
        let weak = super::tokenize_for_detection(content, "markup", &weak_options);

        assert_eq!(strong.len(), 5);
        assert_eq!(weak.len(), 3);
        let token_values: Vec<&str> = weak
            .iter()
            .map(|t| &content[t.range[0]..t.range[1]])
            .collect();
        assert_eq!(token_values, vec!["alpha", "beta", "gamma"]);
    }
}
