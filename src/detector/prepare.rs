use rustc_hash::FxHashMap;

use crate::cli::Options;
use crate::files::SourceFile;
use crate::tokenizer::{DetectionToken, tokenize_maps_for_detection};

use super::model::{FormatId, PreparedSourceDraft, SourceMeta, TokenSpan};

pub(super) fn assign_formats(files: &[PreparedSourceDraft]) -> (Vec<FormatId>, Vec<String>) {
    let mut by_name = FxHashMap::default();
    let mut names = Vec::new();
    let ids = files
        .iter()
        .map(|file| {
            if let Some(id) = by_name.get(&file.meta.format) {
                *id
            } else {
                let id = FormatId(names.len());
                by_name.insert(file.meta.format.clone(), id);
                names.push(file.meta.format.clone());
                id
            }
        })
        .collect();
    (ids, names)
}

pub(super) fn prepare_file_maps(file: SourceFile, options: &Options) -> Vec<PreparedSourceDraft> {
    tokenize_maps_for_detection(&file.content, &file.format, options)
        .into_iter()
        .map(|map| {
            let (hashes, spans) = split_tokens(map.tokens);
            let (stat_lines, stat_tokens) = token_stream_statistics(&spans);
            PreparedSourceDraft {
                meta: SourceMeta {
                    source_id: file.source_id.clone(),
                    format: map.format,
                    content: file.content.clone(),
                    lines: stat_lines,
                    tokens: stat_tokens,
                },
                hashes,
                spans,
            }
        })
        .collect()
}

fn split_tokens(tokens: Vec<DetectionToken>) -> (Vec<u64>, Vec<TokenSpan>) {
    let mut hashes = Vec::with_capacity(tokens.len());
    let mut spans = Vec::with_capacity(tokens.len());
    for token in tokens {
        hashes.push(token.hash);
        spans.push(TokenSpan {
            start: token.start,
            end: token.end,
            range: token.range,
        });
    }
    (hashes, spans)
}

fn token_stream_statistics(spans: &[TokenSpan]) -> (usize, usize) {
    match (spans.first(), spans.last()) {
        (Some(first), Some(last)) => (
            last.end.line.saturating_sub(first.start.line),
            last.end.position.saturating_sub(first.start.position),
        ),
        _ => (0, 0),
    }
}
