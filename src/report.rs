use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult, Statistics, clone_lines};

pub fn write_reports(result: &DetectionResult, options: &Options) -> Result<()> {
    if should_write_report("console", options) && !options.silent {
        write_console(result);
    }
    if should_write_report("json", options) {
        write_json(result, options)?;
    }
    Ok(())
}

fn should_write_report(name: &str, options: &Options) -> bool {
    options.reporters.iter().any(|reporter| reporter == name)
}

fn write_console(result: &DetectionResult) {
    println!("jscpd-rs MVP");
    println!("Files analyzed: {}", result.statistics.total.sources);
    println!("Total lines: {}", result.statistics.total.lines);
    println!("Total tokens: {}", result.statistics.total.tokens);
    println!("Clones found: {}", result.clones.len());
    println!(
        "Duplicated lines: {} ({:.2}%)",
        result.statistics.total.duplicated_lines, result.statistics.total.percentage
    );

    for clone in &result.clones {
        println!(
            "{}:{}-{} duplicates {}:{}-{} ({} lines, {} tokens)",
            clone.duplication_a.source_id,
            clone.duplication_a.start.line,
            clone.duplication_a.end.line,
            clone.duplication_b.source_id,
            clone.duplication_b.start.line,
            clone.duplication_b.end.line,
            clone_lines(clone),
            clone.tokens
        );
    }
}

fn write_json(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.json");
    let report = JsonReport::from_detection(result);
    let json = serde_json::to_string_pretty(&report)?;
    fs::write(&path, json).with_context(|| format!("failed to write `{}`", path.display()))?;
    if !options.silent {
        println!("JSON report saved to {}", path.display());
    }
    Ok(())
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
            },
            second_file: JsonFile {
                name: clone.duplication_b.source_id.clone(),
                start: clone.duplication_b.start.line,
                end: clone.duplication_b.end.line,
                start_loc: clone.duplication_b.start.clone(),
                end_loc: clone.duplication_b.end.clone(),
            },
            fragment,
        }
    }
}

fn slice_range(content: &str, range: [usize; 2]) -> String {
    let start = range[0].min(content.len());
    let end = range[1].min(content.len());
    content.get(start..end).unwrap_or_default().to_string()
}

#[allow(dead_code)]
fn normalize_report_path(path: &Path) -> String {
    path.display().to_string()
}
