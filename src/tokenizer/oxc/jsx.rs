use oxc_parser::Kind;

use super::super::scan::count_prism_whitespace_tokens;
use super::super::{ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind};
use super::{RawOxcToken, push_oxc_token, push_token_part};

pub(super) fn tokenize_jsx_attribute_scripts(
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

pub(super) fn jsx_attribute_script_groups(parser_tokens: &[RawOxcToken]) -> Vec<(usize, usize)> {
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
