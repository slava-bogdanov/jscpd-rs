use rustc_hash::FxHashMap;

use crate::cli::Options;

use super::super::model::{
    CloneMatch, Fragment, Occurrence, PreparedSource, SkippedClone, SourceId,
};
use super::{create_clone, enlarge_clone, flush_clone, windows_match};

const SECONDARY_OCCURRENCE_CAP: usize = 2;

pub(super) fn remember_repeated_window(
    repeated_windows: &mut FxHashMap<u64, Vec<Occurrence>>,
    hash: u64,
    occurrence: Occurrence,
) {
    let bucket = repeated_windows.entry(hash).or_default();
    if bucket.iter().any(|stored| {
        stored.source_id == occurrence.source_id && stored.token_start == occurrence.token_start
    }) {
        return;
    }
    if bucket.len() < SECONDARY_OCCURRENCE_CAP {
        bucket.push(occurrence);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct CandidateWindow {
    source_a: usize,
    source_b: usize,
    token_start_a: usize,
    token_start_b: usize,
}

struct SecondaryOpen {
    clone: CloneMatch,
    source_a: usize,
    source_b: usize,
    last_token_start_a: usize,
    last_token_start_b: usize,
}

pub(super) fn add_secondary_clones(
    format: &str,
    repeated_windows: FxHashMap<u64, Vec<Occurrence>>,
    prepared_files: &[PreparedSource],
    options: &Options,
    clones: &mut Vec<CloneMatch>,
    skipped_clones: &mut Vec<SkippedClone>,
) {
    if repeated_windows.is_empty() {
        return;
    }

    let mut candidates = Vec::new();
    for occurrences in repeated_windows.values() {
        if occurrences.len() < 2 {
            continue;
        }
        for left_idx in 0..occurrences.len() {
            for right_idx in left_idx + 1..occurrences.len() {
                let left = occurrences[left_idx];
                let right = occurrences[right_idx];
                if left.source_id == right.source_id && left.token_start == right.token_start {
                    continue;
                }
                if !windows_match(
                    &prepared_files[left.source_id.0].stream,
                    left.token_start,
                    &prepared_files[right.source_id.0].stream,
                    right.token_start,
                    options.min_tokens,
                ) {
                    continue;
                }
                let (source_a, token_start_a, source_b, token_start_b) =
                    canonical_candidate_pair(left, right);
                candidates.push(CandidateWindow {
                    source_a,
                    source_b,
                    token_start_a,
                    token_start_b,
                });
            }
        }
    }
    if candidates.is_empty() {
        return;
    }
    candidates.sort_unstable();
    candidates.dedup();

    let mut coverage = LineCoverage::from_clones(prepared_files, clones);
    let mut open: Option<SecondaryOpen> = None;

    for candidate in candidates {
        if let Some(current) = open.as_mut()
            && current.source_a == candidate.source_a
            && current.source_b == candidate.source_b
            && current.last_token_start_a + 1 == candidate.token_start_a
            && current.last_token_start_b + 1 == candidate.token_start_b
        {
            enlarge_clone(
                &mut current.clone,
                Occurrence {
                    source_id: SourceId(candidate.source_a),
                    token_start: candidate.token_start_a,
                },
                Occurrence {
                    source_id: SourceId(candidate.source_b),
                    token_start: candidate.token_start_b,
                },
                prepared_files,
                options,
            );
            current.last_token_start_a = candidate.token_start_a;
            current.last_token_start_b = candidate.token_start_b;
            continue;
        }

        flush_secondary_clone(open.take(), clones, skipped_clones, options, &mut coverage);
        let occurrence_a = Occurrence {
            source_id: SourceId(candidate.source_a),
            token_start: candidate.token_start_a,
        };
        let occurrence_b = Occurrence {
            source_id: SourceId(candidate.source_b),
            token_start: candidate.token_start_b,
        };
        open = Some(SecondaryOpen {
            clone: create_clone(format, occurrence_a, occurrence_b, prepared_files, options),
            source_a: candidate.source_a,
            source_b: candidate.source_b,
            last_token_start_a: candidate.token_start_a,
            last_token_start_b: candidate.token_start_b,
        });
    }

    flush_secondary_clone(open.take(), clones, skipped_clones, options, &mut coverage);
}

fn canonical_candidate_pair(left: Occurrence, right: Occurrence) -> (usize, usize, usize, usize) {
    let left_key = (left.source_id.0, left.token_start);
    let right_key = (right.source_id.0, right.token_start);
    if left_key <= right_key {
        (
            left.source_id.0,
            left.token_start,
            right.source_id.0,
            right.token_start,
        )
    } else {
        (
            right.source_id.0,
            right.token_start,
            left.source_id.0,
            left.token_start,
        )
    }
}

fn flush_secondary_clone(
    open: Option<SecondaryOpen>,
    clones: &mut Vec<CloneMatch>,
    skipped_clones: &mut Vec<SkippedClone>,
    options: &Options,
    coverage: &mut LineCoverage,
) {
    let Some(open) = open else {
        return;
    };
    let range_a = fragment_line_range(&open.clone.duplication_a);
    let range_b = fragment_line_range(&open.clone.duplication_b);
    if !coverage.extends(open.source_a, range_a) && !coverage.extends(open.source_b, range_b) {
        return;
    }

    let before = clones.len();
    flush_clone(Some(open.clone), clones, skipped_clones, options);
    if clones.len() > before {
        coverage.insert(open.source_a, range_a);
        coverage.insert(open.source_b, range_b);
    }
}

struct LineCoverage {
    ranges_by_source: Vec<Vec<(usize, usize)>>,
}

impl LineCoverage {
    fn from_clones(prepared_files: &[PreparedSource], clones: &[CloneMatch]) -> Self {
        let mut source_lookup = FxHashMap::default();
        for (idx, source) in prepared_files.iter().enumerate() {
            source_lookup.insert(source.meta.source_id.as_str(), idx);
        }
        let mut coverage = Self {
            ranges_by_source: vec![Vec::new(); prepared_files.len()],
        };
        for clone in clones {
            if let Some(source_idx) = source_lookup.get(clone.duplication_a.source_id.as_str()) {
                coverage.insert(*source_idx, fragment_line_range(&clone.duplication_a));
            }
            if let Some(source_idx) = source_lookup.get(clone.duplication_b.source_id.as_str()) {
                coverage.insert(*source_idx, fragment_line_range(&clone.duplication_b));
            }
        }
        coverage
    }

    fn extends(&self, source_idx: usize, range: (usize, usize)) -> bool {
        let Some(ranges) = self.ranges_by_source.get(source_idx) else {
            return true;
        };
        let mut next_line = range.0;
        for &(start, end) in ranges {
            if end < next_line {
                continue;
            }
            if start > next_line {
                return true;
            }
            next_line = next_line.max(end.saturating_add(1));
            if next_line > range.1 {
                return false;
            }
        }
        next_line <= range.1
    }

    fn insert(&mut self, source_idx: usize, range: (usize, usize)) {
        let Some(ranges) = self.ranges_by_source.get_mut(source_idx) else {
            return;
        };
        ranges.push(range);
        ranges.sort_unstable();

        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
        for &(start, end) in ranges.iter() {
            if let Some((_, previous_end)) = merged.last_mut()
                && start <= previous_end.saturating_add(1)
            {
                *previous_end = (*previous_end).max(end);
                continue;
            }
            merged.push((start, end));
        }
        *ranges = merged;
    }
}

fn fragment_line_range(fragment: &Fragment) -> (usize, usize) {
    let start = fragment.start.line.min(fragment.end.line);
    let end = fragment.start.line.max(fragment.end.line);
    (start, end)
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use crate::tokenizer::Location;

    use super::super::super::model::{FormatId, SourceMeta, TokenSpan, TokenStream};
    use super::*;

    #[test]
    fn secondary_clones_only_extend_uncovered_lines() {
        let options = Options {
            min_tokens: 3,
            min_lines: 0,
            ..Options::default()
        };
        let prepared_files = vec![
            prepared_source(0, "a.js", &[1, 2, 3, 4, 5, 6]),
            prepared_source(1, "b.js", &[8, 9, 3, 4, 5, 6]),
        ];
        let mut clones = vec![clone_with_lines("a.js", 1, 2, "b.js", 1, 2)];
        let mut skipped_clones = Vec::new();

        add_secondary_clones(
            "javascript",
            repeated_windows([
                Occurrence {
                    source_id: SourceId(0),
                    token_start: 2,
                },
                Occurrence {
                    source_id: SourceId(1),
                    token_start: 2,
                },
            ]),
            &prepared_files,
            &options,
            &mut clones,
            &mut skipped_clones,
        );

        assert_eq!(clones.len(), 2);
        assert_eq!(clones[1].duplication_a.source_id, "a.js");
        assert_eq!(clones[1].duplication_a.start.line, 3);
        assert_eq!(clones[1].duplication_a.end.line, 6);
        assert_eq!(clones[1].duplication_b.source_id, "b.js");

        add_secondary_clones(
            "javascript",
            repeated_windows([
                Occurrence {
                    source_id: SourceId(0),
                    token_start: 2,
                },
                Occurrence {
                    source_id: SourceId(1),
                    token_start: 2,
                },
            ]),
            &prepared_files,
            &options,
            &mut clones,
            &mut skipped_clones,
        );

        assert_eq!(clones.len(), 2);
    }

    fn repeated_windows<const N: usize>(
        occurrences: [Occurrence; N],
    ) -> FxHashMap<u64, Vec<Occurrence>> {
        let mut repeated_windows = FxHashMap::default();
        repeated_windows.insert(42, occurrences.to_vec());
        repeated_windows
    }

    fn prepared_source(source_idx: usize, source_id: &str, hashes: &[u64]) -> PreparedSource {
        PreparedSource {
            meta: SourceMeta {
                source_id: source_id.to_string(),
                format: "javascript".to_string(),
                content: String::new(),
                lines: hashes.len(),
                tokens: hashes.len(),
            },
            stream: TokenStream {
                source_id: SourceId(source_idx),
                format_id: FormatId(0),
                hashes: hashes.to_vec(),
                spans: (0..hashes.len()).map(token_span).collect(),
            },
        }
    }

    fn token_span(idx: usize) -> TokenSpan {
        let line = idx + 1;
        TokenSpan {
            start: location(line, 1, idx),
            end: location(line, 2, idx),
            range: [idx, idx + 1],
        }
    }

    fn clone_with_lines(
        source_a: &str,
        start_a: usize,
        end_a: usize,
        source_b: &str,
        start_b: usize,
        end_b: usize,
    ) -> CloneMatch {
        CloneMatch {
            format: "javascript".to_string(),
            duplication_a: fragment(source_a, start_a, end_a),
            duplication_b: fragment(source_b, start_b, end_b),
            tokens: 3,
        }
    }

    fn fragment(source_id: &str, start: usize, end: usize) -> Fragment {
        Fragment {
            source_id: source_id.to_string(),
            start: location(start, 1, start),
            end: location(end, 2, end),
            range: [start, end],
            blame: None,
        }
    }

    fn location(line: usize, column: usize, position: usize) -> Location {
        Location {
            line,
            column,
            position,
        }
    }
}
