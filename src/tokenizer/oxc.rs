use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::{Kind, Parser, config::TokensParserConfig};
use oxc_span::SourceType;

use crate::cli::{Mode, Options};

use super::scan::{has_code_in_gap, scan_block_comment, scan_line_comment};
use super::{
    ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, TokenMap, hash_token, push_token,
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
