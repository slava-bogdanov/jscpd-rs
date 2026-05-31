mod apex;
mod blocks;
mod embedded;
mod generic;
mod hash;
mod ignore;
mod line_index;
mod markdown;
mod markup_attrs;
mod oxc;
mod scan;
mod tap;

use serde::Serialize;

use crate::cli::{Mode, Options};

use generic::tokenize_generic;
use hash::hash_token;
use ignore::find_ignore_regions;
use line_index::LineIndex;
use oxc::{is_oxc_format, tokenize_oxc_maps};
use scan::count_prism_whitespace_tokens;

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

#[derive(Clone, Debug)]
pub struct SourceTokenMap {
    pub source_id: String,
    pub format: String,
    pub tokens: Vec<DetectionToken>,
    pub lines: usize,
}

#[derive(Clone, Debug)]
pub struct Tokenizer {
    options: Options,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            options: Options::default(),
        }
    }

    pub fn with_options(options: Options) -> Self {
        Self { options }
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    pub fn tokenize(&self, content: &str, format: &str) -> Vec<DetectionToken> {
        self.tokenize_maps(content, format)
            .into_iter()
            .next()
            .map(|map| map.tokens)
            .unwrap_or_default()
    }

    pub fn tokenize_maps(&self, content: &str, format: &str) -> Vec<TokenMap> {
        tokenize_maps_for_detection(content, format, &self.options)
    }

    pub fn generate_maps(
        &self,
        source_id: impl Into<String>,
        content: &str,
        format: &str,
    ) -> Vec<SourceTokenMap> {
        let source_id = source_id.into();
        self.tokenize_maps(content, format)
            .into_iter()
            .map(|map| SourceTokenMap {
                source_id: source_id.clone(),
                lines: token_map_line_count(&map.tokens),
                format: map.format,
                tokens: map.tokens,
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    Comment,
    Constant,
    Empty,
    Keyword,
    NewLine,
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
    let mut maps = if format == "markdown" {
        markdown::tokenize_maps(content, options, &ignore_regions)
    } else if format == "apex" {
        apex::tokenize_maps(content, options, &ignore_regions)
    } else if format == "tap" {
        tap::tokenize_maps(content, options, &ignore_regions)
    } else if matches!(format, "markup" | "vue" | "svelte" | "astro") {
        blocks::tokenize_maps(content, format, options, &ignore_regions)
    } else if is_oxc_format(format) {
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

fn token_map_line_count(tokens: &[DetectionToken]) -> usize {
    match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => last.end.line.saturating_sub(first.start.line),
        _ => 0,
    }
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

fn push_strict_whitespace_tokens(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    if context.options.mode != Mode::Strict {
        return;
    }
    let mut start = span.start;
    while start < span.end {
        let (end, kind) = scan_whitespace_token(context.content, start, span.end);
        push_token(
            tokens,
            context,
            kind,
            ByteSpan { start, end },
            line_index.location(start),
            line_index.location(end),
        );
        start = end.max(start + 1);
    }
}

fn scan_whitespace_token(content: &str, start: usize, limit: usize) -> (usize, TokenKind) {
    let bytes = content.as_bytes();
    if bytes[start] == b'\n' {
        return (start + 1, TokenKind::NewLine);
    }

    let mut end = start;
    while end < limit {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if ch == '\n' || !ch.is_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    (end, TokenKind::Empty)
}

#[cfg(test)]
mod tests;
