use rustc_hash::FxHashMap;

use crate::cli::Options;

use super::model::{CloneMatch, FormatId, Fragment, Occurrence, PreparedSource, TokenStream};
use super::skip_local::same_configured_root;
use super::statistics::clone_stat_lines;

const WINDOW_HASH_BASE: u64 = 0x9e37_79b9_7f4a_7c15;

pub(super) fn detect_format(
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
        && same_configured_root(
            &clone.duplication_a.source_id,
            &clone.duplication_b.source_id,
            options,
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
