use std::fs;

use anyhow::{Context, Result};

use super::summary::statistic_to_summary_row;
use crate::cli::Options;
use crate::detector::DetectionResult;

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.md");
    let md = MarkdownReport::from_detection(result).to_string();
    fs::write(&path, md).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("Markdown report saved to {}", path.display());
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;
    use crate::report::write_reports;

    #[test]
    fn write_reports_writes_markdown_report() {
        let output = crate::report::test_support::temp_output("markdown-report");
        let options = Options {
            output: output.clone(),
            reporters: vec!["markdown".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let md = std::fs::read_to_string(output.join("jscpd-report.md")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(md.starts_with("# Copy/paste detection report"));
        assert!(md.contains("> Duplications detection:"));
    }

    #[test]
    fn markdown_report_matches_upstream_summary_shape() {
        let result = crate::detector::DetectionResult {
            clones: Vec::new(),
            statistics: crate::report::test_support::make_test_statistics(),
            sources: Vec::new(),
            source_contents: std::collections::HashMap::new(),
        };
        let md = MarkdownReport::from_detection(&result).to_string();

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
}
