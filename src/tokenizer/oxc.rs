use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::{Kind, Parser, config::TokensParserConfig};
use oxc_span::SourceType;

use crate::cli::{Mode, Options};

use super::scan::{has_code_in_gap, scan_block_comment, scan_line_comment};
use super::{
    ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, TokenMap, hash_token,
    push_strict_whitespace_tokens, push_token,
};

mod fallback;
mod jsx;
mod kind;
mod lexical;

use fallback::tokenize_js_like_range;
use jsx::{jsx_attribute_script_groups, tokenize_jsx_attribute_scripts};
use kind::oxc_token_kind;

#[derive(Clone, Copy)]
struct RawOxcToken {
    kind: Kind,
    span: ByteSpan,
}

pub(super) fn is_oxc_format(format: &str) -> bool {
    matches!(format, "javascript" | "typescript" | "jsx" | "tsx" | "json")
}

pub(super) fn tokenize_oxc_maps(
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
        let span = ByteSpan {
            start: start_byte,
            end: end_byte,
        };
        if token.kind == Kind::Slash
            && context.slice(span) == "/"
            && let Some(regex_end) = scan_regex_literal_end(content, start_byte, content.len())
        {
            push_token_part(
                &mut tokens,
                &context,
                TokenKind::String,
                ByteSpan {
                    start: start_byte,
                    end: regex_end,
                },
                &line_index,
            );
            previous_end = previous_end.max(regex_end);
            idx += 1;
            while idx < parser_tokens.len() && parser_tokens[idx].span.start < regex_end {
                previous_end = previous_end.max(parser_tokens[idx].span.end);
                idx += 1;
            }
            continue;
        }
        push_oxc_token(&mut tokens, &context, token.kind, span, &line_index);
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

fn source_type_for_format(format: &str) -> SourceType {
    let filename = match format {
        "javascript" => "input.jsx",
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
    if span.start >= span.end {
        return;
    }
    let value = context.slice(span);
    if value.starts_with("//") {
        if context.options.mode != Mode::Weak {
            push_line_comment_tokens(tokens, context, span, line_index);
        }
        return;
    }
    if value.starts_with("#!") {
        push_hashbang_tokens(tokens, context, span, line_index);
        return;
    }
    if value.starts_with("/*") || value.starts_with("<!--") {
        if context.options.mode != Mode::Weak {
            push_comment_token(tokens, context, span, line_index);
        }
        return;
    }
    if kind == Kind::Skip {
        return;
    }
    if kind == Kind::JSXText {
        tokenize_js_like_range(tokens, context, span.start, span.end, line_index);
        return;
    }
    if kind == Kind::Ident && value.contains('-') {
        tokenize_js_like_range(tokens, context, span.start, span.end, line_index);
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

pub(super) fn scan_regex_literal_end(
    content: &str,
    slash_start: usize,
    limit: usize,
) -> Option<usize> {
    if !regex_literal_allowed_at(content, slash_start) {
        return None;
    }
    let bytes = content.as_bytes();
    if bytes.get(slash_start) != Some(&b'/')
        || matches!(bytes.get(slash_start + 1), Some(b'/' | b'*'))
    {
        return None;
    }

    let mut idx = slash_start + 1;
    let mut escaped = false;
    let mut in_class = false;
    let mut saw_body = false;
    while idx < bytes.len().min(limit) {
        let byte = bytes[idx];
        if byte == b'\n' || byte == b'\r' {
            return None;
        }
        if escaped {
            escaped = false;
            saw_body = true;
            idx += 1;
            continue;
        }
        match byte {
            b'\\' => {
                escaped = true;
                saw_body = true;
            }
            b'[' => {
                in_class = true;
                saw_body = true;
            }
            b']' => {
                in_class = false;
                saw_body = true;
            }
            b'/' if !in_class => {
                if !saw_body {
                    return None;
                }
                idx += 1;
                while idx < bytes.len().min(limit) && bytes[idx].is_ascii_alphabetic() {
                    idx += 1;
                }
                return Some(idx);
            }
            _ => {
                saw_body = true;
            }
        }
        idx += 1;
    }
    None
}

fn regex_literal_allowed_at(content: &str, slash_start: usize) -> bool {
    let Some((idx, previous)) = content[..slash_start]
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_whitespace())
    else {
        return true;
    };
    if previous == '!' && content[..idx].chars().rev().find(|ch| !ch.is_whitespace()) == Some('#') {
        return false;
    }

    if matches!(
        previous,
        '(' | '['
            | '{'
            | '='
            | ':'
            | ','
            | ';'
            | '!'
            | '?'
            | '&'
            | '|'
            | '+'
            | '-'
            | '*'
            | '~'
            | '^'
            | '<'
            | '>'
    ) {
        return true;
    }

    let word_end = idx + previous.len_utf8();
    let mut word_start = idx;
    while word_start > 0 {
        let Some((prev_idx, ch)) = content[..word_start].char_indices().next_back() else {
            break;
        };
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            word_start = prev_idx;
        } else {
            break;
        }
    }
    matches!(
        &content[word_start..word_end],
        "return" | "throw" | "case" | "delete" | "typeof" | "void" | "new" | "yield" | "await"
    )
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

fn push_comments_in_gap(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    gap_start: usize,
    gap_end: usize,
    line_index: &LineIndex,
) {
    if gap_start >= gap_end {
        return;
    }

    let bytes = context.content.as_bytes();
    let mut idx = gap_start;
    while idx < gap_end {
        let ch = context.content[idx..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            let whitespace_end = scan_whitespace(context.content, idx, gap_end);
            push_strict_whitespace_tokens(
                tokens,
                context,
                ByteSpan {
                    start: idx,
                    end: whitespace_end,
                },
                line_index,
            );
            idx = whitespace_end.max(idx + ch.len_utf8());
            continue;
        }
        if idx + 1 >= gap_end {
            break;
        }
        let is_hashbang = idx == 0 && bytes[idx] == b'#' && bytes[idx + 1] == b'!';
        let is_line_comment = (bytes[idx] == b'/' && bytes[idx + 1] == b'/')
            || bytes[idx..gap_end].starts_with(b"<!--");
        let comment_end = if is_line_comment {
            Some(scan_line_comment(bytes, idx, gap_end))
        } else if is_hashbang {
            Some(scan_line_comment(bytes, idx, gap_end))
        } else if bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            Some(scan_block_comment(bytes, idx, gap_end))
        } else {
            None
        };

        if let Some(comment_end) = comment_end {
            if is_hashbang {
                let span = ByteSpan {
                    start: idx,
                    end: comment_end,
                };
                push_hashbang_tokens(tokens, context, span, line_index);
            } else if context.options.mode != Mode::Weak {
                let span = ByteSpan {
                    start: idx,
                    end: comment_end,
                };
                if bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
                    push_line_comment_tokens(tokens, context, span, line_index);
                } else {
                    push_comment_token(tokens, context, span, line_index);
                }
            }
            idx = comment_end.max(idx + 1);
        } else {
            idx += ch.len_utf8();
        }
    }
}

fn push_hashbang_tokens(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    let hash_span = ByteSpan {
        start: span.start,
        end: span.start + 1,
    };
    push_token_part(tokens, context, TokenKind::Default, hash_span, line_index);
    tokenize_js_like_range(tokens, context, span.start + 1, span.end, line_index);
}

pub(super) fn push_line_comment_tokens(
    tokens: &mut Vec<DetectionToken>,
    context: &TokenContext<'_>,
    span: ByteSpan,
    line_index: &LineIndex,
) {
    let mut part_start = None;
    for (offset, ch) in context.slice(span).char_indices() {
        let idx = span.start + offset;
        if ch.is_whitespace() {
            if let Some(start) = part_start.take() {
                push_comment_token(tokens, context, ByteSpan { start, end: idx }, line_index);
            }
        } else if part_start.is_none() {
            part_start = Some(idx);
        }
    }
    if let Some(start) = part_start {
        push_comment_token(
            tokens,
            context,
            ByteSpan {
                start,
                end: span.end,
            },
            line_index,
        );
    }
}

fn scan_whitespace(content: &str, start: usize, limit: usize) -> usize {
    let mut end = start;
    while end < limit {
        let ch = content[end..].chars().next().unwrap_or('\0');
        if !ch.is_whitespace() {
            break;
        }
        end += ch.len_utf8();
    }
    end
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
