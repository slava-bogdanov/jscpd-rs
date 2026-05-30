use std::collections::HashMap;

use rayon::prelude::*;
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::cli::Options;
use crate::files::SourceFile;
use crate::tokenizer::{DetectionToken, Location, tokenize_maps_for_detection};

const WINDOW_HASH_BASE: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SourceId(usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FormatId(usize);

#[derive(Clone, Debug, Serialize)]
pub struct Fragment {
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub start: Location,
    pub end: Location,
    pub range: [usize; 2],
}

#[derive(Clone, Debug, Serialize)]
pub struct CloneMatch {
    pub format: String,
    #[serde(rename = "duplicationA")]
    pub duplication_a: Fragment,
    #[serde(rename = "duplicationB")]
    pub duplication_b: Fragment,
    pub tokens: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StatisticRow {
    pub lines: usize,
    pub tokens: usize,
    pub sources: usize,
    pub clones: usize,
    #[serde(rename = "duplicatedLines")]
    pub duplicated_lines: usize,
    #[serde(rename = "duplicatedTokens")]
    pub duplicated_tokens: usize,
    pub percentage: f64,
    #[serde(rename = "percentageTokens")]
    pub percentage_tokens: f64,
    #[serde(rename = "newDuplicatedLines")]
    pub new_duplicated_lines: usize,
    #[serde(rename = "newClones")]
    pub new_clones: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FormatStatistic {
    pub sources: HashMap<String, StatisticRow>,
    pub total: StatisticRow,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Statistics {
    pub total: StatisticRow,
    pub formats: HashMap<String, FormatStatistic>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceSummary {
    pub path: String,
    pub format: String,
    pub lines: usize,
    pub tokens: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DetectionResult {
    pub clones: Vec<CloneMatch>,
    pub statistics: Statistics,
    pub sources: Vec<SourceSummary>,
    #[serde(skip)]
    pub source_contents: HashMap<String, String>,
}

#[derive(Clone, Debug)]
struct TokenSpan {
    start: Location,
    end: Location,
    range: [usize; 2],
}

#[derive(Debug)]
struct SourceMeta {
    source_id: String,
    format: String,
    content: String,
    lines: usize,
    tokens: usize,
}

#[derive(Debug)]
struct TokenStream {
    source_id: SourceId,
    format_id: FormatId,
    hashes: Vec<u64>,
    spans: Vec<TokenSpan>,
}

#[derive(Clone, Copy, Debug)]
struct Occurrence {
    source_id: SourceId,
    token_start: usize,
}

#[derive(Debug)]
struct PreparedSource {
    meta: SourceMeta,
    stream: TokenStream,
}

#[derive(Debug)]
struct PreparedSourceDraft {
    meta: SourceMeta,
    hashes: Vec<u64>,
    spans: Vec<TokenSpan>,
}

pub fn detect(files: Vec<SourceFile>, options: &Options) -> DetectionResult {
    let prepared_drafts = files
        .into_par_iter()
        .map(|file| prepare_file_maps(file, options))
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let (format_ids, format_names) = assign_formats(&prepared_drafts);
    let prepared_files = prepared_drafts
        .into_iter()
        .enumerate()
        .map(|(idx, draft)| PreparedSource {
            meta: draft.meta,
            stream: TokenStream {
                source_id: SourceId(idx),
                format_id: format_ids[idx],
                hashes: draft.hashes,
                spans: draft.spans,
            },
        })
        .collect::<Vec<_>>();

    let mut statistics = Statistics::default();
    let mut sources = Vec::new();
    let mut source_contents = HashMap::new();
    let mut source_indices_by_format = vec![Vec::new(); format_names.len()];
    let include_source_contents = options
        .reporters
        .iter()
        .any(|reporter| matches!(reporter.as_str(), "json" | "xml" | "html" | "consoleFull"));

    for (idx, prepared) in prepared_files.iter().enumerate() {
        if prepared.stream.spans.is_empty() {
            continue;
        }
        update_source_statistics(
            &mut statistics,
            &prepared.meta.source_id,
            &prepared.meta.format,
            prepared.meta.lines,
            prepared.meta.tokens,
        );
        sources.push(SourceSummary {
            path: prepared.meta.source_id.clone(),
            format: prepared.meta.format.clone(),
            lines: prepared.meta.lines,
            tokens: prepared.meta.tokens,
        });
        if include_source_contents {
            source_contents.insert(
                prepared.meta.source_id.clone(),
                prepared.meta.content.clone(),
            );
        }
        source_indices_by_format[prepared.stream.format_id.0].push(idx);
    }

    let clones_by_format = source_indices_by_format
        .par_iter()
        .enumerate()
        .map(|(format_id, source_indices)| {
            detect_format(
                FormatId(format_id),
                source_indices,
                &prepared_files,
                &format_names,
                options,
            )
        })
        .collect::<Vec<_>>();

    let clones = clones_by_format.into_iter().flatten().collect::<Vec<_>>();
    for clone in &clones {
        update_clone_statistics(&mut statistics, clone);
    }

    finalize_percentages(&mut statistics);

    DetectionResult {
        clones,
        statistics,
        sources,
        source_contents,
    }
}

fn assign_formats(files: &[PreparedSourceDraft]) -> (Vec<FormatId>, Vec<String>) {
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

fn prepare_file_maps(file: SourceFile, options: &Options) -> Vec<PreparedSourceDraft> {
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

fn detect_format(
    format_id: FormatId,
    source_indices: &[usize],
    prepared_files: &[PreparedSource],
    format_names: &[String],
    options: &Options,
) -> Vec<CloneMatch> {
    let mut store: FxHashMap<u64, Occurrence> = FxHashMap::default();
    store.reserve(total_windows(
        source_indices,
        prepared_files,
        options.min_tokens,
    ));
    let mut clones = Vec::new();

    for &source_idx in source_indices.iter().rev() {
        let stream = &prepared_files[source_idx].stream;
        debug_assert_eq!(stream.source_id.0, source_idx);
        debug_assert_eq!(stream.format_id, format_id);

        if stream.hashes.len() <= options.min_tokens {
            continue;
        }

        let mut open_clone: Option<CloneMatch> = None;
        let mut hash = initial_window_hash(&stream.hashes, options.min_tokens);
        let window_power = WINDOW_HASH_BASE.wrapping_pow((options.min_tokens - 1) as u32);
        let windows_len = stream.hashes.len() - options.min_tokens;

        for token_start in 0..windows_len {
            let current = Occurrence {
                source_id: stream.source_id,
                token_start,
            };
            match store.get(&hash).copied() {
                Some(stored)
                    if windows_match(
                        stream,
                        token_start,
                        &prepared_files[stored.source_id.0].stream,
                        stored.token_start,
                        options.min_tokens,
                    ) =>
                {
                    if open_clone.is_none() {
                        open_clone = Some(create_clone(
                            &format_names[format_id.0],
                            current,
                            stored,
                            prepared_files,
                            options,
                        ));
                    } else if let Some(clone) = open_clone.as_mut() {
                        enlarge_clone(clone, current, stored, prepared_files, options);
                    }
                }
                _ => {
                    flush_clone(open_clone.take(), &mut clones, options);
                    store.insert(hash, current);
                }
            }

            if token_start + options.min_tokens < stream.hashes.len() {
                hash = next_window_hash(
                    hash,
                    stream.hashes[token_start],
                    stream.hashes[token_start + options.min_tokens],
                    window_power,
                );
            }
        }
        flush_clone(open_clone.take(), &mut clones, options);
    }

    clones
}

fn total_windows(
    source_indices: &[usize],
    prepared_files: &[PreparedSource],
    min_tokens: usize,
) -> usize {
    source_indices
        .iter()
        .map(|&source_idx| {
            prepared_files[source_idx]
                .stream
                .hashes
                .len()
                .saturating_sub(min_tokens)
        })
        .sum()
}

fn initial_window_hash(hashes: &[u64], min_tokens: usize) -> u64 {
    hashes[..min_tokens].iter().fold(0u64, |hash, token_hash| {
        hash.wrapping_mul(WINDOW_HASH_BASE)
            .wrapping_add(*token_hash)
    })
}

fn next_window_hash(hash: u64, outgoing: u64, incoming: u64, window_power: u64) -> u64 {
    hash.wrapping_sub(outgoing.wrapping_mul(window_power))
        .wrapping_mul(WINDOW_HASH_BASE)
        .wrapping_add(incoming)
}

fn windows_match(
    stream_a: &TokenStream,
    start_a: usize,
    stream_b: &TokenStream,
    start_b: usize,
    min_tokens: usize,
) -> bool {
    stream_a.hashes[start_a..start_a + min_tokens] == stream_b.hashes[start_b..start_b + min_tokens]
}

fn create_clone(
    format: &str,
    occurrence_a: Occurrence,
    occurrence_b: Occurrence,
    prepared_files: &[PreparedSource],
    options: &Options,
) -> CloneMatch {
    CloneMatch {
        format: format.to_string(),
        duplication_a: fragment_from_occurrence(occurrence_a, prepared_files, options.min_tokens),
        duplication_b: fragment_from_occurrence(occurrence_b, prepared_files, options.min_tokens),
        tokens: options.min_tokens,
    }
}

fn enlarge_clone(
    clone: &mut CloneMatch,
    occurrence_a: Occurrence,
    occurrence_b: Occurrence,
    prepared_files: &[PreparedSource],
    options: &Options,
) {
    enlarge_fragment_end(
        &mut clone.duplication_a,
        occurrence_a,
        prepared_files,
        options.min_tokens,
    );
    enlarge_fragment_end(
        &mut clone.duplication_b,
        occurrence_b,
        prepared_files,
        options.min_tokens,
    );
    clone.tokens += 1;
}

fn fragment_from_occurrence(
    occurrence: Occurrence,
    prepared_files: &[PreparedSource],
    min_tokens: usize,
) -> Fragment {
    let source = &prepared_files[occurrence.source_id.0];
    let start_span = &source.stream.spans[occurrence.token_start];
    let end_span = &source.stream.spans[occurrence.token_start + min_tokens];
    Fragment {
        source_id: source.meta.source_id.clone(),
        start: start_span.start.clone(),
        end: end_span.end.clone(),
        range: [start_span.range[0], end_span.range[1]],
    }
}

fn enlarge_fragment_end(
    fragment: &mut Fragment,
    occurrence: Occurrence,
    prepared_files: &[PreparedSource],
    min_tokens: usize,
) {
    let source = &prepared_files[occurrence.source_id.0];
    let end_span = &source.stream.spans[occurrence.token_start + min_tokens];
    fragment.end = end_span.end.clone();
    fragment.range[1] = end_span.range[1];
}

fn flush_clone(clone: Option<CloneMatch>, clones: &mut Vec<CloneMatch>, options: &Options) {
    let Some(clone) = clone else {
        return;
    };
    if options.skip_local
        && same_parent(
            &clone.duplication_a.source_id,
            &clone.duplication_b.source_id,
        )
    {
        return;
    }
    let lines = clone_stat_lines(&clone);
    if lines < options.min_lines {
        return;
    }

    clones.push(clone);
}

fn same_parent(a: &str, b: &str) -> bool {
    let a = std::path::Path::new(a).parent();
    let b = std::path::Path::new(b).parent();
    a.is_some() && a == b
}

pub fn clone_lines(clone: &CloneMatch) -> usize {
    clone
        .duplication_a
        .end
        .line
        .saturating_sub(clone.duplication_a.start.line)
        + 1
}

fn clone_stat_lines(clone: &CloneMatch) -> usize {
    clone
        .duplication_a
        .end
        .line
        .saturating_sub(clone.duplication_a.start.line)
}

fn clone_stat_tokens(clone: &CloneMatch) -> usize {
    clone
        .duplication_a
        .end
        .position
        .saturating_sub(clone.duplication_a.start.position)
}

fn update_source_statistics(
    statistics: &mut Statistics,
    source_id: &str,
    format_name: &str,
    lines: usize,
    tokens: usize,
) {
    statistics.total.sources += 1;
    statistics.total.lines += lines;
    statistics.total.tokens += tokens;

    let format = statistics
        .formats
        .entry(format_name.to_string())
        .or_default();
    format.total.sources += 1;
    format.total.lines += lines;
    format.total.tokens += tokens;

    let source = format.sources.entry(source_id.to_string()).or_default();
    source.sources = 1;
    source.lines += lines;
    source.tokens += tokens;
}

fn update_clone_statistics(statistics: &mut Statistics, clone: &CloneMatch) {
    let lines = clone_stat_lines(clone);
    let tokens = clone_stat_tokens(clone);
    statistics.total.clones += 1;
    statistics.total.duplicated_lines += lines;
    statistics.total.duplicated_tokens += tokens;

    let format = statistics.formats.entry(clone.format.clone()).or_default();
    format.total.clones += 1;
    format.total.duplicated_lines += lines;
    format.total.duplicated_tokens += tokens;

    for source_id in [
        &clone.duplication_a.source_id,
        &clone.duplication_b.source_id,
    ] {
        let source = format.sources.entry(source_id.clone()).or_default();
        source.clones += 1;
        source.duplicated_lines += lines;
        source.duplicated_tokens += tokens;
    }
}

fn finalize_percentages(statistics: &mut Statistics) {
    update_row_percentages(&mut statistics.total);
    for format in statistics.formats.values_mut() {
        update_row_percentages(&mut format.total);
        for source in format.sources.values_mut() {
            update_row_percentages(source);
        }
    }
}

fn update_row_percentages(row: &mut StatisticRow) {
    row.percentage = percentage(row.lines, row.duplicated_lines);
    row.percentage_tokens = percentage(row.tokens, row.duplicated_tokens);
}

fn percentage(total: usize, duplicated: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((duplicated as f64 * 10000.0) / total as f64).round() / 100.0
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::Options;
    use crate::files::SourceFile;

    use super::detect;

    #[test]
    fn detects_cross_file_duplicates() {
        let options = Options {
            min_tokens: 3,
            min_lines: 0,
            ..Options::default()
        };
        let content = "alpha beta gamma delta epsilon\n";
        let files = vec![
            source("a.js", content),
            source("b.js", &format!("prefix\n{content}\nsuffix\n")),
        ];

        let result = detect(files, &options);

        assert!(!result.clones.is_empty());
    }

    #[test]
    fn detects_generic_format_duplicates() {
        let options = Options {
            min_tokens: 3,
            min_lines: 0,
            ..Options::default()
        };
        let content = "alpha beta gamma delta epsilon\n";
        let files = vec![
            source_with_format("a.css", "css", content),
            source_with_format("b.css", "css", &format!("prefix\n{content}\nsuffix\n")),
        ];

        let result = detect(files, &options);

        assert!(!result.clones.is_empty());
    }

    #[test]
    fn skips_empty_token_sources_in_statistics() {
        let content = "// jscpd:ignore-start\nignored\n// jscpd:ignore-end\n";

        let result = detect(vec![source("ignored.js", content)], &Options::default());

        assert_eq!(result.sources.len(), 0);
        assert_eq!(result.statistics.total.sources, 0);
    }

    fn source(path: &str, content: &str) -> SourceFile {
        source_with_format(path, "javascript", content)
    }

    fn source_with_format(path: &str, format: &str, content: &str) -> SourceFile {
        SourceFile {
            source_id: path.to_string(),
            format: format.to_string(),
            content: content.to_string(),
        }
    }
}
