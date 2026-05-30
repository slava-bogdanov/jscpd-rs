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
    if should_write_report("csv", options) {
        write_csv(result, options)?;
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

fn write_csv(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.csv");
    let csv = CsvReport::from_statistics(&result.statistics).to_string();
    fs::write(&path, csv).with_context(|| format!("failed to write `{}`", path.display()))?;
    if !options.silent {
        println!("CSV report saved to {}", path.display());
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

struct CsvReport {
    rows: Vec<[String; 7]>,
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

impl CsvReport {
    fn from_statistics(statistics: &Statistics) -> Self {
        let mut rows = vec![[
            "Format".to_string(),
            "Files analyzed".to_string(),
            "Total lines".to_string(),
            "Total tokens".to_string(),
            "Clones found".to_string(),
            "Duplicated lines".to_string(),
            "Duplicated tokens".to_string(),
        ]];

        let mut formats = statistics.formats.iter().collect::<Vec<_>>();
        formats.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (format, statistic) in formats {
            rows.push(statistic_to_csv_row(format, &statistic.total));
        }
        rows.push(statistic_to_csv_row("Total:", &statistics.total));

        Self { rows }
    }
}

impl std::fmt::Display for CsvReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, row) in self.rows.iter().enumerate() {
            if idx > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", row.join(","))?;
        }
        Ok(())
    }
}

fn statistic_to_csv_row(format: &str, statistic: &crate::detector::StatisticRow) -> [String; 7] {
    [
        format.to_string(),
        statistic.sources.to_string(),
        statistic.lines.to_string(),
        statistic.tokens.to_string(),
        statistic.clones.to_string(),
        format!("{} ({}%)", statistic.duplicated_lines, statistic.percentage),
        format!(
            "{} ({}%)",
            statistic.duplicated_tokens, statistic.percentage_tokens
        ),
    ]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::FormatStatistic;
    use crate::detector::StatisticRow;
    use std::collections::HashMap;

    fn make_test_statistics() -> Statistics {
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
        Statistics {
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

    #[test]
    fn csv_report_matches_upstream_summary_shape() {
        let stats = make_test_statistics();
        let report = CsvReport::from_statistics(&stats);
        let csv = report.to_string();

        assert_eq!(
            csv,
            [
                "Format,Files analyzed,Total lines,Total tokens,Clones found,Duplicated lines,Duplicated tokens",
                "javascript,2,20,100,1,5 (25%),30 (30%)",
                "Total:,2,20,100,1,5 (25%),30 (30%)",
            ]
            .join("\n")
        );
    }

    #[test]
    fn write_reports_writes_csv_report() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "jscpd-rs-csv-report-{}-{nonce}",
            std::process::id()
        ));
        let options = Options {
            output: output.clone(),
            reporters: vec!["csv".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = DetectionResult {
            clones: Vec::new(),
            statistics: make_test_statistics(),
            sources: Vec::new(),
            source_contents: HashMap::new(),
        };

        write_reports(&result, &options).unwrap();
        let csv = std::fs::read_to_string(output.join("jscpd-report.csv")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(csv.starts_with("Format,Files analyzed,Total lines"));
    }
}
