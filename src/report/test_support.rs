use std::collections::HashMap;
use std::path::PathBuf;

use crate::detector::{CloneMatch, DetectionResult, FormatStatistic, Fragment, StatisticRow};
use crate::tokenizer::Location;

pub(super) fn make_test_statistics() -> crate::detector::Statistics {
    let mut formats = HashMap::new();
    formats.insert(
        "javascript".to_string(),
        FormatStatistic {
            sources: HashMap::new(),
            total: StatisticRow {
                sources: 2,
                lines: 20,
                tokens: 100,
                clones: 1,
                duplicated_lines: 5,
                duplicated_tokens: 30,
                percentage: 25.0,
                percentage_tokens: 30.0,
                new_duplicated_lines: 0,
                new_clones: 0,
            },
        },
    );
    crate::detector::Statistics {
        total: StatisticRow {
            sources: 2,
            lines: 20,
            tokens: 100,
            clones: 1,
            duplicated_lines: 5,
            duplicated_tokens: 30,
            percentage: 25.0,
            percentage_tokens: 30.0,
            new_duplicated_lines: 0,
            new_clones: 0,
        },
        formats,
    }
}

pub(super) fn make_test_clone(source_a: &str, source_b: &str) -> CloneMatch {
    CloneMatch {
        format: "javascript".to_string(),
        duplication_a: Fragment {
            source_id: source_a.to_string(),
            start: location(2, 3, 0),
            end: location(5, 1, 18),
            range: [0, 18],
            blame: None,
        },
        duplication_b: Fragment {
            source_id: source_b.to_string(),
            start: location(8, 1, 0),
            end: location(11, 1, 18),
            range: [0, 18],
            blame: None,
        },
        tokens: 6,
    }
}

pub(super) fn make_test_result_with_clone(source_a: &str, source_b: &str) -> DetectionResult {
    let mut source_contents = HashMap::new();
    source_contents.insert(source_a.to_string(), "alpha <beta> ]]>\n".to_string());
    source_contents.insert(source_b.to_string(), "alpha & beta\nxxxx\n".to_string());

    DetectionResult {
        clones: vec![make_test_clone(source_a, source_b)],
        statistics: make_test_statistics(),
        sources: Vec::new(),
        source_contents,
    }
}

pub(super) fn temp_output(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("jscpd-rs-{label}-{}-{nonce}", std::process::id()))
}

fn location(line: usize, column: usize, position: usize) -> Location {
    Location {
        line,
        column,
        position,
    }
}
