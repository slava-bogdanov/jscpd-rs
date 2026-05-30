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
    if should_write_report("markdown", options) {
        write_markdown(result, options)?;
    }
    if should_write_report("xml", options) {
        write_xml(result, options)?;
    }
    if should_write_report("sarif", options) {
        write_sarif(result, options)?;
    }
    if should_write_report("xcode", options) {
        write_xcode(result, options);
    }
    if should_write_report("silent", options) {
        write_silent(result);
    }
    if should_write_report("threshold", options) {
        write_threshold(result, options)?;
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
    println!("JSON report saved to {}", path.display());
    Ok(())
}

fn write_csv(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.csv");
    let csv = CsvReport::from_statistics(&result.statistics).to_string();
    fs::write(&path, csv).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("CSV report saved to {}", path.display());
    Ok(())
}

fn write_markdown(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.md");
    let md = MarkdownReport::from_detection(result).to_string();
    fs::write(&path, md).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("Markdown report saved to {}", path.display());
    Ok(())
}

fn write_xml(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.xml");
    let xml = XmlReport::from_detection(result).to_string();
    fs::write(&path, xml).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("XML report saved to {}", path.display());
    Ok(())
}

fn write_sarif(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-sarif.json");
    let sarif = SarifReport::from_detection(result, options);
    let json = serde_json::to_string(&sarif)?;
    fs::write(&path, json).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("SARIF report saved to {}", path.display());
    Ok(())
}

fn write_xcode(result: &DetectionResult, options: &Options) {
    for clone in &result.clones {
        println!("{}", XcodeWarning::from_clone(clone, options));
    }
    println!("Found {} clones.", result.clones.len());
}

fn write_silent(result: &DetectionResult) {
    println!("{}", silent_summary(result));
}

fn write_threshold(result: &DetectionResult, options: &Options) -> Result<()> {
    let Some(threshold) = options.threshold else {
        return Ok(());
    };
    if threshold < result.statistics.total.percentage {
        anyhow::bail!(
            "ERROR: jscpd found too many duplicates ({}%) over threshold ({}%)",
            result.statistics.total.percentage,
            threshold,
        );
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

struct XmlReport {
    duplications: Vec<XmlDuplication>,
}

struct XmlDuplication {
    lines: usize,
    first_file: XmlFile,
    second_file: XmlFile,
    fragment: String,
}

struct XmlFile {
    path: String,
    line: usize,
    fragment: String,
}

struct XcodeWarning {
    message: String,
}

#[derive(Serialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
    artifacts: Vec<SarifArtifact>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    rules: Vec<SarifRule>,
    version: &'static str,
    information_uri: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: &'static str,
    short_description: SarifMessage,
    help_uri: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    level: &'static str,
    message: SarifMessage,
    rule_id: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    locations: Vec<SarifLocation>,
    rule_index: usize,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifact {
    source_language: String,
    location: SarifArtifactLocation,
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
            rows.push(statistic_to_summary_row(format, &statistic.total));
        }
        rows.push(statistic_to_summary_row("Total:", &statistics.total));

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

impl XmlReport {
    fn from_detection(result: &DetectionResult) -> Self {
        Self {
            duplications: result
                .clones
                .iter()
                .map(|clone| XmlDuplication::from_clone(clone, result))
                .collect(),
        }
    }
}

impl XmlDuplication {
    fn from_clone(clone: &CloneMatch, result: &DetectionResult) -> Self {
        let first_fragment = clone_fragment(result, &clone.duplication_a);
        let second_fragment = clone_fragment(result, &clone.duplication_b);

        Self {
            lines: clone
                .duplication_a
                .end
                .line
                .saturating_sub(clone.duplication_a.start.line),
            first_file: XmlFile {
                path: escape_xml(&clone.duplication_a.source_id),
                line: clone.duplication_a.start.line,
                fragment: first_fragment.clone(),
            },
            second_file: XmlFile {
                path: escape_xml(&clone.duplication_b.source_id),
                line: clone.duplication_b.start.line,
                fragment: second_fragment,
            },
            fragment: first_fragment,
        }
    }
}

impl std::fmt::Display for XmlReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"<?xml version="1.0" encoding="UTF-8" ?><pmd-cpd>"#)?;
        for duplication in &self.duplications {
            write!(
                f,
                "\n      <duplication lines=\"{}\">\n            <file path=\"{}\" line=\"{}\">\n              <codefragment><![CDATA[{}]]></codefragment>\n            </file>\n            <file path=\"{}\" line=\"{}\">\n              <codefragment><![CDATA[{}]]></codefragment>\n            </file>\n            <codefragment><![CDATA[{}]]></codefragment>\n        </duplication>\n      ",
                duplication.lines,
                duplication.first_file.path,
                duplication.first_file.line,
                cdata_fragment(&duplication.first_file.fragment),
                duplication.second_file.path,
                duplication.second_file.line,
                cdata_fragment(&duplication.second_file.fragment),
                cdata_fragment(&duplication.fragment),
            )?;
        }
        write!(f, "</pmd-cpd>")
    }
}

impl XcodeWarning {
    fn from_clone(clone: &CloneMatch, options: &Options) -> Self {
        let start_line_a = clone.duplication_a.start.line;
        let end_line_a = clone.duplication_a.end.line;
        let path_a = absolute_report_path(&clone.duplication_a.source_id);
        let path_b = if options.absolute {
            absolute_report_path(&clone.duplication_b.source_id)
        } else {
            clone.duplication_b.source_id.clone()
        };

        Self {
            message: format!(
                "{}:{}:{}: warning: Found {} lines ({}-{}) duplicated on file {} ({}-{})",
                path_a,
                start_line_a,
                clone.duplication_a.start.column,
                end_line_a.saturating_sub(start_line_a),
                start_line_a,
                end_line_a,
                path_b,
                clone.duplication_b.start.line,
                clone.duplication_b.end.line,
            ),
        }
    }
}

impl std::fmt::Display for XcodeWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl SarifReport {
    fn from_detection(result: &DetectionResult, options: &Options) -> Self {
        const URL: &str = "https://github.com/kucherenko/jscpd/";

        let mut artifacts = Vec::new();
        let mut artifact_indices = std::collections::HashMap::new();
        let mut results = Vec::new();

        for clone in &result.clones {
            let uri = clone.duplication_a.source_id.clone();
            let artifact_index = *artifact_indices.entry(uri.clone()).or_insert_with(|| {
                let index = artifacts.len();
                artifacts.push(SarifArtifact {
                    source_language: sarif_source_language(&clone.format),
                    location: SarifArtifactLocation {
                        uri: uri.clone(),
                        index: None,
                    },
                });
                index
            });

            results.push(SarifResult {
                level: "warning",
                message: SarifMessage {
                    text: format!(
                        "Clone detected in {}, - {}[{}] and {}[{}]",
                        clone.format,
                        clone.duplication_a.source_id,
                        source_location(&clone.duplication_a.start, &clone.duplication_a.end),
                        clone.duplication_b.source_id,
                        source_location(&clone.duplication_b.start, &clone.duplication_b.end),
                    ),
                },
                rule_id: "duplication",
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri,
                            index: Some(artifact_index),
                        },
                        region: SarifRegion {
                            start_line: clone.duplication_a.start.line,
                            start_column: clone.duplication_a.start.column,
                            end_line: clone.duplication_a.end.line,
                            end_column: clone.duplication_a.end.column,
                        },
                    },
                }],
                rule_index: 0,
            });
        }

        if result.statistics.total.percentage >= options.threshold.unwrap_or(100.0) {
            results.push(SarifResult {
                level: "error",
                message: SarifMessage {
                    text: format!(
                        "The duplication level ({}%) is bigger than threshold ({}%)",
                        result.statistics.total.percentage,
                        options
                            .threshold
                            .map(|threshold| threshold.to_string())
                            .unwrap_or_else(|| "undefined".to_string()),
                    ),
                },
                rule_id: "duplications-threshold",
                locations: Vec::new(),
                rule_index: 1,
            });
        }

        Self {
            schema: "http://json.schemastore.org/sarif-2.1.0.json",
            version: "2.1.0",
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "jscpd",
                        rules: vec![
                            SarifRule {
                                id: "duplication",
                                short_description: SarifMessage {
                                    text: "Found code duplication".to_string(),
                                },
                                help_uri: URL,
                            },
                            SarifRule {
                                id: "duplications-threshold",
                                short_description: SarifMessage {
                                    text: "Level of duplication is too high".to_string(),
                                },
                                help_uri: URL,
                            },
                        ],
                        version: "4.2.4",
                        information_uri: URL,
                    },
                },
                results,
                artifacts,
            }],
        }
    }
}

fn statistic_to_summary_row(
    format: &str,
    statistic: &crate::detector::StatisticRow,
) -> [String; 7] {
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

struct MarkdownReport {
    summary_line: String,
    rows: Vec<[String; 7]>,
}

impl MarkdownReport {
    fn from_detection(result: &DetectionResult) -> Self {
        let stats = &result.statistics;
        let clone_count = result.clones.len();
        let total = &stats.total;
        let format_count = stats.formats.len();

        let summary_line = format!(
            "> Duplications detection: Found {} exact clones with {}({}%) duplicated lines in {} ({} formats) files.",
            clone_count, total.duplicated_lines, total.percentage, total.sources, format_count,
        );

        let mut rows: Vec<[String; 7]> = vec![[
            "Format".to_string(),
            "Files analyzed".to_string(),
            "Total lines".to_string(),
            "Total tokens".to_string(),
            "Clones found".to_string(),
            "Duplicated lines".to_string(),
            "Duplicated tokens".to_string(),
        ]];

        let mut formats: Vec<_> = stats.formats.iter().collect();
        formats.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (format, statistic) in formats {
            rows.push(statistic_to_summary_row(format, &statistic.total));
        }
        rows.push(
            statistic_to_summary_row("Total:", &stats.total).map(|cell| format!("**{cell}**")),
        );

        Self { summary_line, rows }
    }
}

impl std::fmt::Display for MarkdownReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "# Copy/paste detection report")?;
        writeln!(f)?;
        writeln!(f, "{}", self.summary_line)?;
        writeln!(f)?;
        let widths = markdown_column_widths(&self.rows);
        for (row_idx, row) in self.rows.iter().enumerate() {
            write_markdown_row(f, row, &widths)?;
            if row_idx == 0 {
                write_markdown_separator(f, &widths)?;
            }
        }
        Ok(())
    }
}

fn markdown_column_widths(rows: &[[String; 7]]) -> [usize; 7] {
    let mut widths = [0usize; 7];
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.len());
        }
    }
    widths
}

fn write_markdown_row(
    f: &mut std::fmt::Formatter<'_>,
    row: &[String; 7],
    widths: &[usize; 7],
) -> std::fmt::Result {
    write!(f, "|")?;
    for (idx, cell) in row.iter().enumerate() {
        write!(f, " {cell:<width$} |", width = widths[idx])?;
    }
    writeln!(f)
}

fn write_markdown_separator(
    f: &mut std::fmt::Formatter<'_>,
    widths: &[usize; 7],
) -> std::fmt::Result {
    write!(f, "|")?;
    for width in widths {
        write!(f, " {:-<width$} |", "", width = *width)?;
    }
    writeln!(f)
}

fn slice_range(content: &str, range: [usize; 2]) -> String {
    let start = range[0].min(content.len());
    let end = range[1].min(content.len());
    content.get(start..end).unwrap_or_default().to_string()
}

fn clone_fragment(result: &DetectionResult, fragment: &crate::detector::Fragment) -> String {
    result
        .source_contents
        .get(&fragment.source_id)
        .map(|content| slice_range(content, fragment.range))
        .unwrap_or_default()
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

fn cdata_fragment(value: &str) -> String {
    value.replacen("]]>", "CDATA_END", 1)
}

fn absolute_report_path(source_id: &str) -> String {
    let path = Path::new(source_id);
    if path.is_absolute() {
        source_id.to_string()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| Path::new(".").to_path_buf())
            .join(path)
            .display()
            .to_string()
    }
}

fn silent_summary(result: &DetectionResult) -> String {
    format!(
        "Duplications detection: Found {} exact clones with {}({}%) duplicated lines in {} ({} formats) files.",
        result.clones.len(),
        result.statistics.total.duplicated_lines,
        result.statistics.total.percentage,
        result.statistics.total.sources,
        result.statistics.formats.len(),
    )
}

fn source_location(start: &crate::tokenizer::Location, end: &crate::tokenizer::Location) -> String {
    format!(
        "{}:{} - {}:{}",
        start.line, start.column, end.line, end.column
    )
}

fn sarif_source_language(format: &str) -> String {
    match format {
        "javascript" => "JavaScript".to_string(),
        "typescript" => "TypeScript".to_string(),
        "jsx" => "JSX".to_string(),
        "tsx" => "TSX".to_string(),
        "css" => "CSS".to_string(),
        "html" | "markup" => "HTML".to_string(),
        "json" => "JSON".to_string(),
        "markdown" => "Markdown".to_string(),
        "rust" => "Rust".to_string(),
        "python" => "Python".to_string(),
        "ruby" => "Ruby".to_string(),
        "go" => "Go".to_string(),
        "java" => "Java".to_string(),
        "csharp" => "C#".to_string(),
        "cpp" => "C++".to_string(),
        "c" => "C".to_string(),
        other => {
            let mut chars = other.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_uppercase(), chars.as_str())
        }
    }
}

#[allow(dead_code)]
fn normalize_report_path(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::{CloneMatch, FormatStatistic, Fragment, StatisticRow};
    use crate::tokenizer::Location;
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

    fn make_test_clone(source_a: &str, source_b: &str) -> CloneMatch {
        CloneMatch {
            format: "javascript".to_string(),
            duplication_a: Fragment {
                source_id: source_a.to_string(),
                start: location(2, 3, 0),
                end: location(5, 1, 18),
                range: [0, 18],
            },
            duplication_b: Fragment {
                source_id: source_b.to_string(),
                start: location(8, 1, 0),
                end: location(11, 1, 18),
                range: [0, 18],
            },
            tokens: 6,
        }
    }

    fn make_test_result_with_clone(source_a: &str, source_b: &str) -> DetectionResult {
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

    fn location(line: usize, column: usize, position: usize) -> Location {
        Location {
            line,
            column,
            position,
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

    #[test]
    fn write_reports_writes_markdown_report() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "jscpd-rs-markdown-report-{}-{nonce}",
            std::process::id()
        ));
        let options = Options {
            output: output.clone(),
            reporters: vec!["markdown".to_string()],
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
        let md = std::fs::read_to_string(output.join("jscpd-report.md")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(md.starts_with("# Copy/paste detection report"));
        assert!(md.contains("| javascript | 2"));
    }

    #[test]
    fn markdown_report_matches_upstream_summary_shape() {
        let stats = make_test_statistics();
        let report = MarkdownReport::from_detection(&DetectionResult {
            clones: Vec::new(),
            statistics: stats,
            sources: Vec::new(),
            source_contents: HashMap::new(),
        });
        let md = report.to_string();

        assert_eq!(
            md,
            [
                "# Copy/paste detection report",
                "",
                "> Duplications detection: Found 0 exact clones with 5(25%) duplicated lines in 2 (1 formats) files.",
                "",
                "| Format     | Files analyzed | Total lines | Total tokens | Clones found | Duplicated lines | Duplicated tokens |",
                "| ---------- | -------------- | ----------- | ------------ | ------------ | ---------------- | ----------------- |",
                "| javascript | 2              | 20          | 100          | 1            | 5 (25%)          | 30 (30%)          |",
                "| **Total:** | **2**          | **20**      | **100**      | **1**        | **5 (25%)**      | **30 (30%)**      |",
                "",
            ]
            .join("\n")
        );
    }

    #[test]
    fn xcode_warning_matches_upstream_shape() {
        let options = Options {
            absolute: false,
            ..Options::default()
        };
        let clone = make_test_clone("src/a.js", "src/b.js");
        let warning = XcodeWarning::from_clone(&clone, &options).to_string();
        let expected_prefix = std::env::current_dir()
            .unwrap()
            .join("src/a.js")
            .display()
            .to_string();

        assert_eq!(
            warning,
            format!(
                "{expected_prefix}:2:3: warning: Found 3 lines (2-5) duplicated on file src/b.js (8-11)"
            )
        );
    }

    #[test]
    fn xcode_warning_respects_absolute_second_path() {
        let options = Options {
            absolute: true,
            ..Options::default()
        };
        let clone = make_test_clone("src/a.js", "src/b.js");
        let warning = XcodeWarning::from_clone(&clone, &options).to_string();
        let expected_second = std::env::current_dir()
            .unwrap()
            .join("src/b.js")
            .display()
            .to_string();

        assert!(warning.ends_with(&format!("duplicated on file {expected_second} (8-11)")));
    }

    #[test]
    fn xml_report_matches_upstream_pmd_cpd_shape() {
        let result = make_test_result_with_clone("src/a<&>.js", "src/b.js");
        let xml = XmlReport::from_detection(&result).to_string();

        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8" ?><pmd-cpd>"#));
        assert!(xml.ends_with("</pmd-cpd>"));
        assert!(xml.contains(r#"<duplication lines="3">"#));
        assert!(xml.contains(r#"<file path="src/a&lt;&amp;&gt;.js" line="2">"#));
        assert!(xml.contains("<![CDATA[alpha <beta> CDATA_END\n]]>"));
        assert!(xml.contains(r#"<file path="src/b.js" line="8">"#));
    }

    #[test]
    fn write_reports_writes_xml_report() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "jscpd-rs-xml-report-{}-{nonce}",
            std::process::id()
        ));
        let options = Options {
            output: output.clone(),
            reporters: vec!["xml".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let xml = std::fs::read_to_string(output.join("jscpd-report.xml")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(xml.contains("<pmd-cpd>"));
        assert!(xml.contains(r#"<file path="src/a.js" line="2">"#));
    }

    #[test]
    fn sarif_report_matches_upstream_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let report = SarifReport::from_detection(&result, &Options::default());
        let json = serde_json::to_value(report).unwrap();

        assert_eq!(
            json["$schema"],
            "http://json.schemastore.org/sarif-2.1.0.json"
        );
        assert_eq!(json["version"], "2.1.0");
        assert_eq!(json["runs"][0]["tool"]["driver"]["name"], "jscpd");
        assert_eq!(
            json["runs"][0]["tool"]["driver"]["rules"][0]["id"],
            "duplication"
        );
        assert_eq!(
            json["runs"][0]["results"][0]["message"]["text"],
            "Clone detected in javascript, - src/a.js[2:3 - 5:1] and src/b.js[8:1 - 11:1]"
        );
        assert_eq!(
            json["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["index"],
            0
        );
        assert_eq!(
            json["runs"][0]["artifacts"][0]["sourceLanguage"],
            "JavaScript"
        );
    }

    #[test]
    fn sarif_report_includes_threshold_result_like_upstream() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.statistics.total.percentage = 25.0;
        let options = Options {
            threshold: Some(10.0),
            ..Options::default()
        };
        let report = SarifReport::from_detection(&result, &options);
        let json = serde_json::to_value(report).unwrap();

        assert_eq!(json["runs"][0]["results"][1]["level"], "error");
        assert_eq!(
            json["runs"][0]["results"][1]["message"]["text"],
            "The duplication level (25%) is bigger than threshold (10%)"
        );
        assert!(json["runs"][0]["results"][1]["locations"].is_null());
    }

    #[test]
    fn write_reports_writes_sarif_report() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "jscpd-rs-sarif-report-{}-{nonce}",
            std::process::id()
        ));
        let options = Options {
            output: output.clone(),
            reporters: vec!["sarif".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let sarif = std::fs::read_to_string(output.join("jscpd-sarif.json")).unwrap();
        let _ = std::fs::remove_dir_all(output);
        let json: serde_json::Value = serde_json::from_str(&sarif).unwrap();

        assert_eq!(json["version"], "2.1.0");
        assert_eq!(json["runs"][0]["results"][0]["ruleId"], "duplication");
    }

    #[test]
    fn silent_reporter_matches_upstream_summary_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        assert_eq!(
            silent_summary(&result),
            "Duplications detection: Found 1 exact clones with 5(25%) duplicated lines in 2 (1 formats) files."
        );
    }

    #[test]
    fn threshold_reporter_uses_strictly_greater_percentage_like_upstream() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.statistics.total.percentage = 25.0;

        let equal = Options {
            threshold: Some(25.0),
            reporters: vec!["threshold".to_string()],
            ..Options::default()
        };
        assert!(write_reports(&result, &equal).is_ok());

        let below = Options {
            threshold: Some(24.9),
            reporters: vec!["threshold".to_string()],
            ..Options::default()
        };
        let error = write_reports(&result, &below).unwrap_err().to_string();
        assert_eq!(
            error,
            "ERROR: jscpd found too many duplicates (25%) over threshold (24.9%)"
        );
    }
}
