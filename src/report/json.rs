use std::fs;

use anyhow::{Context, Result};
use serde::Serialize;

use super::source::slice_range;
use crate::cli::Options;
use crate::detector::{BlamedLines, CloneMatch, DetectionResult, Statistics, clone_lines};

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.json");
    let json = to_pretty_json(result)?;
    fs::write(&path, json).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("JSON report saved to {}", path.display());
    Ok(())
}

pub(super) fn to_pretty_json(result: &DetectionResult) -> Result<String> {
    Ok(serde_json::to_string_pretty(&JsonReport::from_detection(
        result,
    ))?)
}

#[derive(Serialize)]
struct JsonReport {
    duplicates: Vec<JsonDuplicate>,
    statistics: Statistics,
}

#[derive(Serialize)]
struct JsonDuplicate {
    format: String,
    lines: usize,
    tokens: usize,
    #[serde(rename = "firstFile")]
    first_file: JsonFile,
    #[serde(rename = "secondFile")]
    second_file: JsonFile,
    fragment: String,
}

#[derive(Serialize)]
struct JsonFile {
    name: String,
    start: usize,
    end: usize,
    #[serde(rename = "startLoc")]
    start_loc: crate::tokenizer::Location,
    #[serde(rename = "endLoc")]
    end_loc: crate::tokenizer::Location,
    #[serde(skip_serializing_if = "Option::is_none")]
    blame: Option<BlamedLines>,
}

impl JsonReport {
    fn from_detection(result: &DetectionResult) -> Self {
        Self {
            duplicates: result
                .clones
                .iter()
                .map(|clone| JsonDuplicate::from_clone(clone, result))
                .collect(),
            statistics: result.statistics.clone(),
        }
    }
}

impl JsonDuplicate {
    fn from_clone(clone: &CloneMatch, result: &DetectionResult) -> Self {
        let fragment = result
            .source_contents
            .get(&clone.duplication_a.source_id)
            .map(|content| slice_range(content, clone.duplication_a.range))
            .unwrap_or_default();

        Self {
            format: clone.format.clone(),
            lines: clone_lines(clone),
            tokens: 0,
            first_file: JsonFile {
                name: clone.duplication_a.source_id.clone(),
                start: clone.duplication_a.start.line,
                end: clone.duplication_a.end.line,
                start_loc: clone.duplication_a.start.clone(),
                end_loc: clone.duplication_a.end.clone(),
                blame: clone.duplication_a.blame.clone(),
            },
            second_file: JsonFile {
                name: clone.duplication_b.source_id.clone(),
                start: clone.duplication_b.start.line,
                end: clone.duplication_b.end.line,
                start_loc: clone.duplication_b.start.clone(),
                end_loc: clone.duplication_b.end.clone(),
                blame: clone.duplication_b.blame.clone(),
            },
            fragment,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::detector::BlamedLine;
    use crate::report::test_support::make_test_result_with_clone;

    use super::to_pretty_json;

    #[test]
    fn json_report_includes_blame_when_present() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.clones[0].duplication_a.blame = Some(
            [(
                "2".to_string(),
                BlamedLine {
                    rev: "abc123".to_string(),
                    author: "Alice".to_string(),
                    date: "2024-01-01 00:00:00 +0000".to_string(),
                    line: "2".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );

        let json = to_pretty_json(&result).unwrap();

        assert!(json.contains(r#""blame""#));
        assert!(json.contains(r#""author": "Alice""#));
        assert!(json.contains(r#""rev": "abc123""#));
    }
}
