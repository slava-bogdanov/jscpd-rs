use super::super::scan::{scan_block_comment, scan_line_comment};
use super::super::{
    ByteSpan, DetectionToken, LineIndex, TokenContext, TokenKind, push_strict_whitespace_tokens,
    push_token,
};
use super::lexical::{is_js_constant, is_js_keyword};
use super::push_line_comment_tokens;

pub(super) fn tokenize_js_like_range(
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
            let whitespace_end = scan_whitespace(context.content, idx, range_end);
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

        if idx + 1 < range_end && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            let end = scan_line_comment(bytes, idx, range_end);
            if context.options.mode != crate::cli::Mode::Weak {
                push_line_comment_tokens(tokens, context, ByteSpan { start: idx, end }, line_index);
            }
            idx = end.max(idx + 1);
            continue;
        }

        let (end, kind) = if idx + 1 < range_end && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            (
                scan_block_comment(bytes, idx, range_end),
                TokenKind::Comment,
            )
        } else if bytes[idx] == b'`' {
            (
                scan_template_literal(bytes, idx, range_end).unwrap_or(range_end),
                TokenKind::String,
            )
        } else if matches!(bytes[idx], b'\'' | b'"') {
            if let Some(end) = scan_closed_string(bytes, idx, bytes[idx], range_end) {
                (end, TokenKind::String)
            } else {
                (
                    scan_unclosed_quote_fragment(context.content, idx, range_end),
                    TokenKind::Default,
                )
            }
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

fn scan_closed_string(bytes: &[u8], start: usize, quote: u8, limit: usize) -> Option<usize> {
    let mut idx = start + 1;
    while idx < limit {
        if bytes[idx] == b'\\' {
            idx = (idx + 2).min(limit);
            continue;
        }
        if matches!(bytes[idx], b'\n' | b'\r') {
            return None;
        }
        if bytes[idx] == quote {
            return Some(idx + 1);
        }
        idx += 1;
    }
    None
}

fn scan_template_literal(bytes: &[u8], start: usize, limit: usize) -> Option<usize> {
    let mut idx = start + 1;
    while idx < limit {
        if bytes[idx] == b'\\' {
            idx = (idx + 2).min(limit);
            continue;
        }
        if bytes[idx] == b'`' {
            return Some(idx + 1);
        }
        idx += 1;
    }
    None
}

fn scan_unclosed_quote_fragment(content: &str, start: usize, limit: usize) -> usize {
    let bytes = content.as_bytes();
    let mut idx = start + 1;
    while idx < limit {
        let ch = content[idx..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() || is_js_text_delimiter(bytes[idx]) {
            break;
        }
        idx += ch.len_utf8();
    }
    idx
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

fn is_js_text_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b'{' | b'}'
            | b'['
            | b']'
            | b'('
            | b')'
            | b';'
            | b','
            | b':'
            | b'.'
            | b'<'
            | b'>'
            | b'='
            | b'+'
            | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'&'
            | b'|'
            | b'^'
            | b'!'
            | b'?'
            | b'~'
    )
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphabetic() || (ch as u32) > 0x7f
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use crate::cli::Options;
    use crate::tokenizer::hash_token;

    use super::*;

    #[test]
    fn multiline_template_literals_are_single_string_tokens() {
        let content = "expect(store).toMatchInlineSnapshot(`\n  [root]\n`);\n";
        let options = Options::default();
        let line_index = LineIndex::new(content);
        let context = TokenContext {
            content,
            options: &options,
            ignore_regions: &[],
        };
        let mut tokens = Vec::new();

        tokenize_js_like_range(&mut tokens, &context, 0, content.len(), &line_index);

        let template = tokens
            .iter()
            .find(|token| content[token.range[0]..token.range[1]].starts_with('`'))
            .expect("template token");
        assert_eq!(
            &content[template.range[0]..template.range[1]],
            "`\n  [root]\n`"
        );
        assert_eq!(template.start.line, 1);
        assert_eq!(template.end.line, 3);
        assert_eq!(
            template.hash,
            hash_token(TokenKind::String, "`\n  [root]\n`", false)
        );
    }
}
