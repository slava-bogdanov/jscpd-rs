use super::console_common::{BOLD, GREY, RED, RESET_BOLD, RESET_COLOR};
use super::summary::statistic_to_summary_row;
use crate::cli::Options;
use crate::detector::DetectionResult;

const HEADERS: [&str; 7] = [
    "Format",
    "Files analyzed",
    "Total lines",
    "Total tokens",
    "Clones found",
    "Duplicated lines",
    "Duplicated tokens",
];

pub(super) fn write(result: &DetectionResult, options: &Options) {
    print!("{}", console_report(result, options));
}

fn console_report(result: &DetectionResult, _options: &Options) -> String {
    let mut output = String::new();
    output.push_str(&summary_table(result));
    output.push_str(&format!(
        "{GREY}Found {} clones.{RESET_COLOR}\n",
        result.clones.len()
    ));
    output
}

fn summary_table(result: &DetectionResult) -> String {
    let mut rows = Vec::new();
    let mut formats = result.statistics.formats.iter().collect::<Vec<_>>();
    formats.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (format, statistic) in formats {
        rows.push(statistic_to_summary_row(format, &statistic.total));
    }
    rows.push(statistic_to_summary_row("Total:", &result.statistics.total));

    let widths = column_widths(&rows);
    let mut table = String::new();
    table.push_str(&divider('┌', '┬', '┐', &widths));
    table.push_str(&row(
        &HEADERS.map(str::to_string),
        &widths,
        RowStyle::Header,
    ));
    table.push_str(&divider('├', '┼', '┤', &widths));
    for (index, row_cells) in rows.iter().enumerate() {
        let is_total = index + 1 == rows.len();
        if is_total && index > 0 {
            table.push_str(&divider('├', '┼', '┤', &widths));
        }
        table.push_str(&row(
            row_cells,
            &widths,
            if is_total {
                RowStyle::Total
            } else {
                RowStyle::Body
            },
        ));
    }
    table.push_str(&divider('└', '┴', '┘', &widths));
    table
}

fn column_widths(rows: &[[String; 7]]) -> [usize; 7] {
    let mut widths = HEADERS.map(str::len);
    for row in rows {
        for (idx, cell) in row.iter().enumerate() {
            widths[idx] = widths[idx].max(cell.len());
        }
    }
    widths
}

fn divider(left: char, middle: char, right: char, widths: &[usize; 7]) -> String {
    let mut line = String::new();
    line.push_str(GREY);
    line.push(left);
    for (idx, width) in widths.iter().enumerate() {
        line.push_str(&"─".repeat(width + 2));
        if idx + 1 == widths.len() {
            line.push(right);
        } else {
            line.push(middle);
        }
    }
    line.push_str(RESET_COLOR);
    line.push('\n');
    line
}

#[derive(Clone, Copy)]
enum RowStyle {
    Header,
    Body,
    Total,
}

fn row(cells: &[String; 7], widths: &[usize; 7], style: RowStyle) -> String {
    let mut line = String::new();
    line.push_str(GREY);
    line.push('│');
    line.push_str(RESET_COLOR);

    for (idx, cell) in cells.iter().enumerate() {
        let padded = format!(" {cell:<width$} ", width = widths[idx]);
        match style {
            RowStyle::Header => {
                line.push_str(RED);
                line.push_str(&padded);
                line.push_str(RESET_COLOR);
            }
            RowStyle::Total if idx == 0 => {
                line.push_str(BOLD);
                line.push_str(&padded);
                line.push_str(RESET_BOLD);
            }
            RowStyle::Total | RowStyle::Body => line.push_str(&padded),
        }
        line.push_str(GREY);
        line.push('│');
        line.push_str(RESET_COLOR);
    }

    line.push('\n');
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::{make_test_result_with_clone, make_test_statistics};

    #[test]
    fn console_report_matches_upstream_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let report = console_report(&result, &Options::default());

        assert!(!report.contains("Clone found (javascript):"));
        assert!(report.contains("Format"));
        assert!(report.contains("Files analyzed"));
        assert!(report.contains("javascript"));
        assert!(report.contains("Total:"));
        assert!(report.contains("Found 1 clones."));
    }

    #[test]
    fn console_report_includes_zero_clone_table() {
        let mut statistics = make_test_statistics();
        statistics.total.clones = 0;
        statistics.total.duplicated_lines = 0;
        statistics.total.duplicated_tokens = 0;
        statistics.total.percentage = 0.0;
        statistics.total.percentage_tokens = 0.0;
        for statistic in statistics.formats.values_mut() {
            statistic.total.clones = 0;
            statistic.total.duplicated_lines = 0;
            statistic.total.duplicated_tokens = 0;
            statistic.total.percentage = 0.0;
            statistic.total.percentage_tokens = 0.0;
        }
        let result = DetectionResult {
            clones: Vec::new(),
            skipped_clones: Vec::new(),
            statistics,
            sources: Vec::new(),
            source_contents: std::collections::HashMap::new(),
        };

        let report = console_report(&result, &Options::default());

        assert!(!report.contains("Clone found ("));
        assert!(report.contains("javascript"));
        assert!(report.contains("0 (0%)"));
        assert!(report.contains("Found 0 clones."));
    }
}
