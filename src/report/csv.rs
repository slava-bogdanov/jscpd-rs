use std::fs;

use anyhow::{Context, Result};

use super::summary::statistic_to_summary_row;
use crate::cli::Options;
use crate::detector::Statistics;

pub(super) fn write(result: &crate::detector::DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.csv");
    let csv = CsvReport::from_statistics(&result.statistics).to_string();
    fs::write(&path, csv).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("CSV report saved to {}", path.display());
    Ok(())
}

struct CsvReport {
    rows: Vec<[String; 7]>,
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
        formats.sort_by_key(|(format, _)| *format);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::{make_test_result_with_clone, make_test_statistics};
    use crate::report::write_reports;

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
        let output = crate::report::test_support::temp_output("csv-report");
        let options = Options {
            output: output.clone(),
            reporters: vec!["csv".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let csv = std::fs::read_to_string(output.join("jscpd-report.csv")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(csv.starts_with("Format,Files analyzed,Total lines"));
    }
}
