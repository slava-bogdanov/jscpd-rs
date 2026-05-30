use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::detector::{CloneMatch, DetectionResult, Fragment, SkippedClone};
use crate::tokenizer::Location;

const GREY: &str = "\x1b[90m";
const YELLOW: &str = "\x1b[33m";
const RESET_COLOR: &str = "\x1b[39m";

pub fn write_detection_events(result: &DetectionResult) {
    print!("{}", detection_events_output(result, current_time_millis()));
}

fn detection_events_output(result: &DetectionResult, found_date: u128) -> String {
    let mut output = String::new();
    let mut emitted = vec![false; result.clones.len()];
    let mut emitted_skipped = vec![false; result.skipped_clones.len()];
    for source in result.sources.iter().rev() {
        output.push_str(&format!("{YELLOW}START_DETECTION{RESET_COLOR}\n"));
        output.push_str(&format!(
            "{GREY}Start detection for source id={} format={}{RESET_COLOR}\n",
            source.path, source.format
        ));
        for (idx, clone) in result.clones.iter().enumerate() {
            if emitted[idx]
                || clone.format != source.format
                || clone.duplication_a.source_id != source.path
            {
                continue;
            }
            push_clone_found(&mut output, clone, found_date + idx as u128);
            emitted[idx] = true;
        }
        for (idx, skipped) in result.skipped_clones.iter().enumerate() {
            if emitted_skipped[idx]
                || skipped.clone.format != source.format
                || skipped.clone.duplication_a.source_id != source.path
            {
                continue;
            }
            push_clone_skipped(&mut output, skipped);
            emitted_skipped[idx] = true;
        }
    }
    for (idx, clone) in result.clones.iter().enumerate() {
        if !emitted[idx] {
            push_clone_found(&mut output, clone, found_date + idx as u128);
        }
    }
    for (idx, skipped) in result.skipped_clones.iter().enumerate() {
        if !emitted_skipped[idx] {
            push_clone_skipped(&mut output, skipped);
        }
    }
    output
}

fn push_clone_found(output: &mut String, clone: &CloneMatch, found_date: u128) {
    output.push_str(&format!("{YELLOW}CLONE_FOUND{RESET_COLOR}\n"));
    if let Ok(json) = serde_json::to_string_pretty(&VerboseClone::new(clone, found_date)) {
        for line in json.lines() {
            output.push_str(GREY);
            output.push_str(line);
            output.push_str(RESET_COLOR);
            output.push('\n');
        }
    }
}

fn push_clone_skipped(output: &mut String, skipped: &SkippedClone) {
    output.push_str(&format!("{YELLOW}CLONE_SKIPPED{RESET_COLOR}\n"));
    output.push_str(&format!(
        "{GREY}Clone skipped: {}{RESET_COLOR}\n",
        skipped.message.join(" ")
    ));
}

fn current_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseClone<'a> {
    format: &'a str,
    found_date: u128,
    duplication_a: VerboseFragment<'a>,
    duplication_b: VerboseFragment<'a>,
}

impl<'a> VerboseClone<'a> {
    fn new(clone: &'a CloneMatch, found_date: u128) -> Self {
        Self {
            format: &clone.format,
            found_date,
            duplication_a: VerboseFragment::new(&clone.duplication_a),
            duplication_b: VerboseFragment::new(&clone.duplication_b),
        }
    }
}

#[derive(Serialize)]
struct VerboseFragment<'a> {
    #[serde(rename = "sourceId")]
    source_id: &'a str,
    start: &'a Location,
    end: &'a Location,
    range: [usize; 2],
}

impl<'a> VerboseFragment<'a> {
    fn new(fragment: &'a Fragment) -> Self {
        Self {
            source_id: &fragment.source_id,
            start: &fragment.start,
            end: &fragment.end,
            range: fragment.range,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::detector::{
        CloneMatch, DetectionResult, Fragment, SkippedClone, SourceSummary, Statistics,
    };
    use crate::tokenizer::Location;

    use super::detection_events_output;

    #[test]
    fn verbose_events_match_upstream_event_shape() {
        let result = DetectionResult {
            clones: vec![CloneMatch {
                format: "javascript".to_string(),
                duplication_a: fragment("src/a.js", 2),
                duplication_b: fragment("src/b.js", 8),
                tokens: 6,
            }],
            skipped_clones: vec![SkippedClone {
                clone: CloneMatch {
                    format: "javascript".to_string(),
                    duplication_a: fragment("src/a.js", 20),
                    duplication_b: fragment("src/b.js", 30),
                    tokens: 3,
                },
                message: vec!["Lines of code less than limit (2 < 5)".to_string()],
            }],
            statistics: Statistics::default(),
            sources: vec![SourceSummary {
                path: "src/a.js".to_string(),
                format: "javascript".to_string(),
                lines: 10,
                tokens: 20,
            }],
            source_contents: HashMap::new(),
        };

        let output = detection_events_output(&result, 123);

        assert!(output.contains("START_DETECTION"));
        assert!(output.contains("Start detection for source id=src/a.js format=javascript"));
        assert!(output.contains("CLONE_FOUND"));
        assert!(output.contains("CLONE_SKIPPED"));
        assert!(output.contains("Clone skipped: Lines of code less than limit (2 < 5)"));
        assert!(output.contains(r#""foundDate": 123"#));
        assert!(output.contains(r#""sourceId": "src/a.js""#));
        assert!(!output.contains(r#""tokens""#));
    }

    fn fragment(source_id: &str, line: usize) -> Fragment {
        Fragment {
            source_id: source_id.to_string(),
            start: location(line, 1, 0),
            end: location(line + 3, 1, 6),
            range: [0, 6],
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
