use std::collections::BTreeMap;

use crate::cli::Options;
use crate::formats;

use super::embedded::{
    assign_sequential_positions, blank_ranges_preserve_newlines, offset_tokens,
    tokenize_generic_with_whitespace,
};
use super::markup_attrs::{
    append_inline_style_attr_tokens, find_inline_style_attrs, inline_style_attr_ranges,
};
use super::{
    DetectionToken, LineIndex, TokenMap, find_ignore_regions, is_oxc_format, tokenize_generic,
    tokenize_oxc_maps,
};

const MAX_BLOCK_SOURCE_LENGTH: usize = 5_000_000;

pub(super) fn tokenize_maps(
    content: &str,
    format: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    if matches!(format, "svelte" | "astro") && content.len() > MAX_BLOCK_SOURCE_LENGTH {
        return Vec::new();
    }

    match format {
        "markup" => tokenize_markup_maps(content, options, ignore_regions),
        "vue" => tokenize_vue_maps(content, options),
        "svelte" => tokenize_svelte_maps(content, options, ignore_regions),
        "astro" => tokenize_astro_maps(content, options, ignore_regions),
        _ => Vec::new(),
    }
}

fn tokenize_markup_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let blocks = find_tag_blocks(content, &["script", "style"]);
    let inner_ranges = blocks
        .iter()
        .map(|block| [block.inner_start, block.inner_end])
        .collect::<Vec<_>>();
    let sanitized = blank_ranges_preserve_newlines(content, &inner_ranges);
    let mut grouped = BTreeMap::<String, Vec<DetectionToken>>::new();
    let line_index = LineIndex::new(content);
    append_markup_fragment_tokens(
        &mut grouped,
        &sanitized,
        options,
        ignore_regions,
        &line_index,
        false,
    );
    for block in blocks {
        let format = resolve_markup_block_format(&block);
        append_offset_block_tokens(&mut grouped, content, &block, &format, options, &line_index);
    }

    grouped_maps(grouped)
}

fn tokenize_vue_maps(content: &str, options: &Options) -> Vec<TokenMap> {
    let blocks = find_tag_blocks(content, &["template", "script", "style"]);
    let mut grouped = BTreeMap::<String, Vec<DetectionToken>>::new();
    let line_index = LineIndex::new(content);

    for block in blocks {
        let format = resolve_vue_block_format(&block);
        append_offset_block_tokens(&mut grouped, content, &block, &format, options, &line_index);
    }

    grouped_maps(grouped)
}

fn tokenize_svelte_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let blocks = find_tag_blocks(content, &["script", "style"]);
    let inner_ranges = blocks
        .iter()
        .map(|block| [block.inner_start, block.inner_end])
        .collect::<Vec<_>>();
    let sanitized = blank_ranges_preserve_newlines(content, &inner_ranges);
    let mut grouped = BTreeMap::<String, Vec<DetectionToken>>::new();
    let mut markup_tokens =
        tokenize_generic_with_whitespace(&sanitized, "markup", options, ignore_regions);
    grouped
        .entry("markup".to_string())
        .or_default()
        .append(&mut markup_tokens);

    let line_index = LineIndex::new(content);
    for block in blocks {
        let format = resolve_svelte_block_format(&block);
        append_offset_block_tokens(&mut grouped, content, &block, &format, options, &line_index);
    }

    grouped_maps(grouped)
}

fn tokenize_astro_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let frontmatter = astro_frontmatter_block(content);
    let blocks = find_tag_blocks(content, &["script", "style"]);
    let mut grouped = BTreeMap::<String, Vec<DetectionToken>>::new();
    let line_index = LineIndex::new(content);

    if let Some(block) = &frontmatter {
        append_offset_block_tokens(
            &mut grouped,
            content,
            block,
            "typescript",
            options,
            &line_index,
        );
    }

    for block in &blocks {
        let format = resolve_astro_block_format(block);
        append_offset_block_tokens(&mut grouped, content, block, &format, options, &line_index);
    }

    let mut blank_ranges = blocks
        .iter()
        .map(|block| [block.inner_start, block.inner_end])
        .collect::<Vec<_>>();
    if let Some(block) = &frontmatter {
        blank_ranges.push([block.block_start, block.block_end]);
    }
    let sanitized = blank_ranges_preserve_newlines(content, &blank_ranges);
    let mut markup_tokens =
        tokenize_generic_with_whitespace(&sanitized, "markup", options, ignore_regions);
    trim_edge_whitespace_tokens(&mut markup_tokens, &sanitized);
    grouped
        .entry("markup".to_string())
        .or_default()
        .append(&mut markup_tokens);

    grouped_maps(grouped)
}

fn append_offset_block_tokens(
    grouped: &mut BTreeMap<String, Vec<DetectionToken>>,
    content: &str,
    block: &TagBlock,
    format: &str,
    options: &Options,
    line_index: &LineIndex,
) {
    if block.inner_start >= block.inner_end {
        return;
    }
    let inner = &content[block.inner_start..block.inner_end];
    let inner_ignore_regions = find_ignore_regions(inner, options);
    let inner_maps = if is_oxc_format(format) {
        tokenize_oxc_maps(inner, format, options, &inner_ignore_regions)
    } else if format == "markup" {
        tokenize_markup_fragment_maps(inner, options, &inner_ignore_regions, true)
    } else {
        vec![TokenMap {
            format: format.to_string(),
            tokens: tokenize_generic_with_whitespace(inner, format, options, &inner_ignore_regions),
            positions_assigned: false,
        }]
    };
    let inner_start = line_index.location(block.inner_start);
    for mut map in inner_maps {
        if map.format == format {
            trim_edge_whitespace_tokens(&mut map.tokens, inner);
        }
        offset_tokens(&mut map.tokens, block.inner_start, &inner_start);
        grouped.entry(map.format).or_default().extend(map.tokens);
    }
}

fn trim_edge_whitespace_tokens(tokens: &mut Vec<DetectionToken>, content: &str) {
    let Some(first_content) = tokens
        .iter()
        .position(|token| !token_slice(content, token).chars().all(char::is_whitespace))
    else {
        tokens.clear();
        return;
    };
    let last_content = tokens
        .iter()
        .rposition(|token| !token_slice(content, token).chars().all(char::is_whitespace))
        .unwrap_or(first_content);

    if last_content + 1 < tokens.len() {
        tokens.drain(last_content + 1..);
    }
    if first_content > 0 {
        tokens.drain(..first_content);
    }
}

fn token_slice<'a>(content: &'a str, token: &DetectionToken) -> &'a str {
    &content[token.range[0]..token.range[1]]
}

fn tokenize_markup_fragment_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
    keep_whitespace: bool,
) -> Vec<TokenMap> {
    let mut grouped = BTreeMap::<String, Vec<DetectionToken>>::new();
    let line_index = LineIndex::new(content);
    append_markup_fragment_tokens(
        &mut grouped,
        content,
        options,
        ignore_regions,
        &line_index,
        keep_whitespace,
    );
    grouped
        .into_iter()
        .filter_map(|(format, tokens)| {
            (!tokens.is_empty()).then_some(TokenMap {
                format,
                tokens,
                positions_assigned: false,
            })
        })
        .collect()
}

fn append_markup_fragment_tokens(
    grouped: &mut BTreeMap<String, Vec<DetectionToken>>,
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
    line_index: &LineIndex,
    keep_whitespace: bool,
) {
    let style_attrs = find_inline_style_attrs(content);
    let style_attr_ranges = inline_style_attr_ranges(&style_attrs);
    let markup_sanitized = blank_ranges_preserve_newlines(content, &style_attr_ranges);
    let mut markup_tokens = if keep_whitespace {
        tokenize_generic_with_whitespace(&markup_sanitized, "markup", options, ignore_regions)
    } else {
        tokenize_generic(&markup_sanitized, "markup", options, ignore_regions)
    };
    grouped
        .entry("markup".to_string())
        .or_default()
        .append(&mut markup_tokens);
    append_inline_style_attr_tokens(
        grouped,
        content,
        &style_attrs,
        options,
        ignore_regions,
        line_index,
    );
}

fn grouped_maps(grouped: BTreeMap<String, Vec<DetectionToken>>) -> Vec<TokenMap> {
    grouped
        .into_iter()
        .filter_map(|(format, mut tokens)| {
            if tokens.is_empty() {
                return None;
            }
            tokens.sort_by_key(|token| (token.range[0], token.range[1]));
            assign_sequential_positions(&mut tokens);
            Some(TokenMap {
                format,
                tokens,
                positions_assigned: true,
            })
        })
        .collect()
}

fn resolve_vue_block_format(block: &TagBlock) -> String {
    let lang = attr_value(&block.attrs, "lang").unwrap_or_default();
    match block.tag.as_str() {
        "template" => {
            if !lang.is_empty() && formats::supported_formats().contains(&lang.as_str()) {
                lang
            } else {
                "markup".to_string()
            }
        }
        "script" => {
            if matches!(lang.as_str(), "ts" | "typescript") {
                "typescript".to_string()
            } else {
                "javascript".to_string()
            }
        }
        "style" => match lang.as_str() {
            "scss" => "scss".to_string(),
            "less" => "less".to_string(),
            _ => "css".to_string(),
        },
        _ => "markup".to_string(),
    }
}

fn resolve_svelte_block_format(block: &TagBlock) -> String {
    let lang = attr_value(&block.attrs, "lang").unwrap_or_default();
    match block.tag.as_str() {
        "script" => match lang.as_str() {
            "ts" | "typescript" => "typescript".to_string(),
            "" | "js" | "javascript" => "javascript".to_string(),
            _ => "markup".to_string(),
        },
        "style" => match lang.as_str() {
            "scss" | "sass" => "scss".to_string(),
            "less" => "less".to_string(),
            "" | "css" | "postcss" | "stylus" => "css".to_string(),
            _ => "markup".to_string(),
        },
        _ => "markup".to_string(),
    }
}

fn resolve_astro_block_format(block: &TagBlock) -> String {
    let lang = attr_value(&block.attrs, "lang").unwrap_or_default();
    match block.tag.as_str() {
        "script" => {
            if matches!(lang.as_str(), "ts" | "typescript") {
                "typescript".to_string()
            } else {
                "javascript".to_string()
            }
        }
        "style" => match lang.as_str() {
            "scss" => "scss".to_string(),
            "less" => "less".to_string(),
            _ => "css".to_string(),
        },
        _ => "markup".to_string(),
    }
}

fn resolve_markup_block_format(block: &TagBlock) -> String {
    let lang = attr_value(&block.attrs, "lang")
        .or_else(|| attr_value(&block.attrs, "language"))
        .or_else(|| attr_value(&block.attrs, "type"))
        .unwrap_or_default();
    match block.tag.as_str() {
        "script" => match lang.as_str() {
            "ts" | "typescript" | "text/typescript" | "application/typescript" => {
                "typescript".to_string()
            }
            _ => "javascript".to_string(),
        },
        "style" => match lang.as_str() {
            "scss" | "text/scss" => "scss".to_string(),
            "sass" | "text/sass" => "sass".to_string(),
            "less" | "text/less" => "less".to_string(),
            _ => "css".to_string(),
        },
        _ => "markup".to_string(),
    }
}

#[derive(Clone, Debug)]
struct TagBlock {
    tag: String,
    attrs: String,
    block_start: usize,
    inner_start: usize,
    inner_end: usize,
    block_end: usize,
}

fn find_tag_blocks(content: &str, tags: &[&'static str]) -> Vec<TagBlock> {
    let lower = content.to_ascii_lowercase();
    let mut blocks = Vec::new();
    let mut cursor = 0usize;

    while let Some(open_offset) = lower[cursor..].find('<') {
        let block_start = cursor + open_offset;
        if lower.as_bytes().get(block_start + 1) == Some(&b'/') {
            cursor = block_start + 1;
            continue;
        }
        let Some(tag) = opening_tag_at(&lower, block_start, tags) else {
            cursor = block_start + 1;
            continue;
        };
        let Some(open_tag_end) = lower[block_start..].find('>').map(|idx| block_start + idx) else {
            break;
        };
        let inner_start = open_tag_end + 1;
        let close_needle = format!("</{tag}");
        let Some(close_offset) = lower[inner_start..].find(&close_needle) else {
            cursor = inner_start;
            continue;
        };
        let inner_end = inner_start + close_offset;
        let close_start = inner_end;
        let block_end = lower[close_start..]
            .find('>')
            .map(|idx| close_start + idx + 1)
            .unwrap_or(close_start + close_needle.len());
        let attrs_start = block_start + 1 + tag.len();
        blocks.push(TagBlock {
            tag: tag.to_string(),
            attrs: content[attrs_start..open_tag_end].to_string(),
            block_start,
            inner_start,
            inner_end,
            block_end: block_end.min(content.len()),
        });
        cursor = block_end;
    }

    blocks
}

fn opening_tag_at(lower: &str, block_start: usize, tags: &[&'static str]) -> Option<&'static str> {
    tags.iter().copied().find(|tag| {
        let name_start = block_start + 1;
        let name_end = name_start + tag.len();
        lower[name_start..].starts_with(*tag)
            && lower
                .as_bytes()
                .get(name_end)
                .is_some_and(|byte| matches!(*byte, b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r'))
    })
}

fn astro_frontmatter_block(content: &str) -> Option<TagBlock> {
    if !(content.starts_with("---\n") || content.starts_with("---\r\n")) {
        return None;
    }
    let lines = line_spans(content);
    let close_idx = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, span)| content[span.start..span.end].trim() == "---")
        .map(|(idx, _)| idx)?;
    let inner_start = lines.get(1)?.start;
    let inner_end = content[..lines[close_idx].start]
        .strip_suffix('\n')
        .map(|prefix| prefix.len())
        .unwrap_or(lines[close_idx].start);
    Some(TagBlock {
        tag: "script".to_string(),
        attrs: "lang=\"ts\"".to_string(),
        block_start: 0,
        inner_start,
        inner_end: inner_end.max(inner_start),
        block_end: lines[close_idx].next_start.min(content.len()),
    })
}

#[derive(Clone, Copy)]
struct LineSpan {
    start: usize,
    end: usize,
    next_start: usize,
}

fn line_spans(content: &str) -> Vec<LineSpan> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    while start < content.len() {
        let rest = &content[start..];
        let newline = rest.find('\n');
        let end = newline
            .map(|offset| start + offset)
            .unwrap_or(content.len());
        let next_start = newline.map(|offset| start + offset + 1).unwrap_or(end);
        spans.push(LineSpan {
            start,
            end,
            next_start,
        });
        start = next_start;
    }
    spans
}

fn attr_value(attrs: &str, name: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let name = name.to_ascii_lowercase();
    let mut cursor = 0usize;
    while let Some(offset) = lower[cursor..].find(&name) {
        let start = cursor + offset;
        let end = start + name.len();
        if !attr_name_boundary(lower.as_bytes(), start, end) {
            cursor = end;
            continue;
        }
        let mut idx = skip_ascii_whitespace(lower.as_bytes(), end);
        if lower.as_bytes().get(idx) != Some(&b'=') {
            cursor = end;
            continue;
        }
        idx = skip_ascii_whitespace(lower.as_bytes(), idx + 1);
        let quote = *attrs.as_bytes().get(idx)?;
        if !matches!(quote, b'\'' | b'"') {
            cursor = idx + 1;
            continue;
        }
        let value_start = idx + 1;
        let value_end = attrs[value_start..]
            .bytes()
            .position(|byte| byte == quote)
            .map(|value_offset| value_start + value_offset)?;
        return Some(attrs[value_start..value_end].to_ascii_lowercase());
    }
    None
}

fn attr_name_boundary(bytes: &[u8], start: usize, end: usize) -> bool {
    let before_ok = start == 0
        || !matches!(
            bytes[start - 1],
            b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b':'
        );
    let after_ok = end >= bytes.len()
        || !matches!(
            bytes[end],
            b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' | b':'
        );
    before_ok && after_ok
}

fn skip_ascii_whitespace(bytes: &[u8], mut idx: usize) -> usize {
    while bytes
        .get(idx)
        .is_some_and(|byte| matches!(*byte, b' ' | b'\t' | b'\n' | b'\r'))
    {
        idx += 1;
    }
    idx
}
