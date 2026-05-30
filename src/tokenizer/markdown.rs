use std::path::Path;

use crate::cli::Options;
use crate::formats;

use super::embedded::{
    assign_sequential_positions, blank_ranges_preserve_newlines, offset_tokens,
    tokenize_generic_with_whitespace,
};
use super::{
    DetectionToken, LineIndex, TokenMap, find_ignore_regions, is_oxc_format, tokenize_generic,
    tokenize_oxc_maps,
};

pub(super) fn tokenize_maps(
    content: &str,
    options: &Options,
    ignore_regions: &[[usize; 2]],
) -> Vec<TokenMap> {
    let mut fences = markdown_fenced_code_blocks(content, options);
    if let Some(front_matter) = markdown_front_matter_block(content) {
        fences.push(front_matter);
        fences.sort_by_key(|fence| fence.block_start);
    }
    let sanitized = blank_ranges_preserve_newlines(
        content,
        fences
            .iter()
            .map(|fence| [fence.block_start, fence.block_end])
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let mut maps = Vec::new();
    let line_index = LineIndex::new(content);
    let mut markdown_tokens = tokenize_generic(&sanitized, "markdown", options, ignore_regions);
    if !markdown_tokens.is_empty() {
        markdown_tokens.sort_by_key(|token| (token.range[0], token.range[1]));
        maps.push(TokenMap {
            format: "markdown".to_string(),
            tokens: markdown_tokens,
            positions_assigned: false,
        });
    }

    let mut embedded_maps = std::collections::BTreeMap::<String, Vec<DetectionToken>>::new();
    for fence in fences {
        let inner = &content[fence.inner_start..fence.inner_end];
        let inner_ignore_regions = find_ignore_regions(inner, options);
        let inner_maps = if is_oxc_format(&fence.format) {
            tokenize_oxc_maps(inner, &fence.format, options, &inner_ignore_regions)
        } else if fence.format == "yaml" {
            vec![TokenMap {
                format: fence.format.clone(),
                tokens: tokenize_generic(inner, &fence.format, options, &inner_ignore_regions),
                positions_assigned: false,
            }]
        } else {
            vec![TokenMap {
                format: fence.format.clone(),
                tokens: tokenize_generic_with_whitespace(
                    inner,
                    &fence.format,
                    options,
                    &inner_ignore_regions,
                ),
                positions_assigned: false,
            }]
        };
        let inner_start = line_index.location(fence.inner_start);
        for mut map in inner_maps {
            offset_tokens(&mut map.tokens, fence.inner_start, &inner_start);
            embedded_maps
                .entry(map.format)
                .or_default()
                .extend(map.tokens);
        }
    }

    for (format, mut tokens) in embedded_maps {
        assign_sequential_positions(&mut tokens);
        maps.push(TokenMap {
            format,
            tokens,
            positions_assigned: true,
        });
    }

    maps
}

#[derive(Debug)]
struct MarkdownFence {
    format: String,
    block_start: usize,
    inner_start: usize,
    inner_end: usize,
    block_end: usize,
}

fn markdown_fenced_code_blocks(content: &str, options: &Options) -> Vec<MarkdownFence> {
    let lines = line_spans(content);
    let mut fences = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let line = &content[lines[idx].start..lines[idx].end];
        let Some(open) = markdown_opening_fence(line) else {
            idx += 1;
            continue;
        };
        let Some(format) = resolve_markdown_fence_format(open.info, options) else {
            idx += 1;
            continue;
        };
        let Some(close_idx) = lines[idx + 1..]
            .iter()
            .position(|span| markdown_closing_fence(&content[span.start..span.end], &open))
            .map(|position| idx + 1 + position)
        else {
            idx += 1;
            continue;
        };
        let inner_start = lines
            .get(idx + 1)
            .map(|span| span.start)
            .unwrap_or(lines[idx].next_start);
        let inner_end = content[..lines[close_idx].start]
            .strip_suffix('\n')
            .map(|prefix| prefix.len())
            .unwrap_or(lines[close_idx].start);
        fences.push(MarkdownFence {
            format,
            block_start: lines[idx].start,
            inner_start,
            inner_end: inner_end.max(inner_start),
            block_end: lines[close_idx].next_start.min(content.len()),
        });
        idx = close_idx + 1;
    }
    fences
}

fn markdown_front_matter_block(content: &str) -> Option<MarkdownFence> {
    if !(content.starts_with("---\n") || content.starts_with("---\r\n")) {
        return None;
    }
    let lines = line_spans(content);
    let close_idx = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, span)| {
            let line = content[span.start..span.end].trim();
            line == "---" || line == "..."
        })
        .map(|(idx, _)| idx)?;
    let inner_start = lines.get(1)?.start;
    let inner_end = content[..lines[close_idx].start]
        .strip_suffix('\n')
        .map(|prefix| prefix.len())
        .unwrap_or(lines[close_idx].start);
    Some(MarkdownFence {
        format: "yaml".to_string(),
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

struct MarkdownFenceOpen<'a> {
    marker: u8,
    len: usize,
    info: &'a str,
}

fn markdown_opening_fence(line: &str) -> Option<MarkdownFenceOpen<'_>> {
    let bytes = line.as_bytes();
    let marker = *bytes.first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let len = bytes.iter().take_while(|byte| **byte == marker).count();
    if len < 3 {
        return None;
    }
    Some(MarkdownFenceOpen {
        marker,
        len,
        info: line[len..].trim(),
    })
}

fn markdown_closing_fence(line: &str, open: &MarkdownFenceOpen<'_>) -> bool {
    let bytes = line.as_bytes();
    let len = bytes
        .iter()
        .take_while(|byte| **byte == open.marker)
        .count();
    len >= open.len && bytes[len..].iter().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn resolve_markdown_fence_format(info: &str, options: &Options) -> Option<String> {
    let tag = info.split_whitespace().next()?.to_ascii_lowercase();
    let mapped = match tag.as_str() {
        "node" => Some("javascript"),
        "shell" | "zsh" => Some("bash"),
        "golang" => Some("go"),
        _ => formats::format_for_path(
            Path::new(&format!("code.{tag}")),
            &options.formats_exts,
            &options.formats_names,
        )
        .or_else(|| {
            formats::supported_formats()
                .contains(&tag.as_str())
                .then_some(tag.as_str())
        }),
    }?;
    Some(mapped.to_string())
}
