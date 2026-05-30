use std::collections::HashMap;

use rayon::prelude::*;
use rustc_hash::FxHashSet;

use crate::cli::Options;
use crate::files::SourceFile;

mod matching;
mod model;
mod prepare;
mod skip_local;
mod statistics;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub use model::FormatStatistic;
pub(crate) use model::PreparedSourceDraft;
pub use model::{
    BlamedLine, BlamedLines, CloneMatch, DetectionResult, Fragment, SkippedClone, SourceSummary,
    StatisticRow, Statistics,
};
pub use statistics::clone_lines;

use matching::detect_format;
use model::{FormatId, PreparedSource, SourceId, TokenStream};
use prepare::{assign_formats, prepare_file_maps};
use statistics::{finalize_percentages, update_clone_statistics, update_source_statistics};

pub fn detect(files: Vec<SourceFile>, options: &Options) -> DetectionResult {
    detect_prepared_drafts(prepare_source_drafts(files, options), options)
}

pub(crate) fn prepare_source_drafts(
    files: Vec<SourceFile>,
    options: &Options,
) -> Vec<PreparedSourceDraft> {
    files
        .into_par_iter()
        .map(|file| prepare_file_maps(file, options))
        .collect::<Vec<_>>()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
}

pub(crate) fn detect_prepared_drafts(
    prepared_drafts: Vec<PreparedSourceDraft>,
    options: &Options,
) -> DetectionResult {
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

    let format_results = source_indices_by_format
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

    let mut clones = Vec::new();
    let mut skipped_clones = Vec::new();
    for format_result in format_results {
        clones.extend(format_result.clones);
        skipped_clones.extend(format_result.skipped_clones);
    }
    dedup_exact_clones(&mut clones);
    for clone in &clones {
        update_clone_statistics(&mut statistics, clone);
    }

    finalize_percentages(&mut statistics);

    DetectionResult {
        clones,
        skipped_clones,
        statistics,
        sources,
        source_contents,
    }
}

fn dedup_exact_clones(clones: &mut Vec<CloneMatch>) {
    let mut seen = FxHashSet::default();
    clones.retain(|clone| seen.insert(CloneDedupKey::from(clone)));
}

#[derive(Hash, Eq, PartialEq)]
struct CloneDedupKey {
    format: String,
    duplication_a: FragmentDedupKey,
    duplication_b: FragmentDedupKey,
    tokens: usize,
}

impl From<&CloneMatch> for CloneDedupKey {
    fn from(clone: &CloneMatch) -> Self {
        Self {
            format: clone.format.clone(),
            duplication_a: FragmentDedupKey::from(&clone.duplication_a),
            duplication_b: FragmentDedupKey::from(&clone.duplication_b),
            tokens: clone.tokens,
        }
    }
}

#[derive(Hash, Eq, PartialEq)]
struct FragmentDedupKey {
    source_id: String,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    range: [usize; 2],
}

impl From<&Fragment> for FragmentDedupKey {
    fn from(fragment: &Fragment) -> Self {
        Self {
            source_id: fragment.source_id.clone(),
            start_line: fragment.start.line,
            start_column: fragment.start.column,
            end_line: fragment.end.line,
            end_column: fragment.end.column,
            range: fragment.range,
        }
    }
}
