use super::console_common::{GREY, RESET_COLOR, clone_header};
use super::source::clone_fragment;
use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult};

pub(super) fn write(result: &DetectionResult, options: &Options) {
    print!("{}", console_full_report(result, options));
}

fn console_full_report(result: &DetectionResult, options: &Options) -> String {
    let mut output = String::new();
    for clone in &result.clones {
        output.push_str(&clone_header(clone, options));
        output.push('\n');
        output.push_str(&fragment_table(result, clone));
        output.push('\n');
    }
    output.push_str(&format!(
        "{GREY}Found {} clones.{RESET_COLOR}\n",
        result.clones.len()
    ));
    output
}

fn fragment_table(result: &DetectionResult, clone: &CloneMatch) -> String {
    let fragment = clone_fragment(result, &clone.duplication_a);
    let lines = fragment.split('\n').collect::<Vec<_>>();
    let max_line_a = clone.duplication_a.start.line + lines.len().saturating_sub(1);
    let max_line_b = clone.duplication_b.start.line + lines.len().saturating_sub(1);
    let width_a = max_line_a.to_string().len();
    let width_b = max_line_b.to_string().len();
    let mut output = String::new();

    for (idx, line) in lines.iter().enumerate() {
        if idx > 0 {
            output.push('\n');
        }
        let line_a = clone.duplication_a.start.line + idx;
        let line_b = clone.duplication_b.start.line + idx;
        if let (Some(blame_a), Some(blame_b)) =
            (&clone.duplication_a.blame, &clone.duplication_b.blame)
        {
            let key_a = line_a.to_string();
            let key_b = line_b.to_string();
            let author_a = blame_a
                .get(&key_a)
                .map(|line| line.author.as_str())
                .unwrap_or("");
            let author_b = blame_b
                .get(&key_b)
                .map(|line| line.author.as_str())
                .unwrap_or("");
            let date_cmp = blame_a
                .get(&key_a)
                .zip(blame_b.get(&key_b))
                .map(|(left, right)| compare_dates(&left.date, &right.date))
                .unwrap_or("");
            output.push_str(&format!(
                " {line_a:>width_a$} {GREY}│{RESET_COLOR} {author_a} {GREY}│{RESET_COLOR} {date_cmp} {GREY}│{RESET_COLOR} {line_b:<width_b$} {GREY}│{RESET_COLOR} {author_b} {GREY}│{RESET_COLOR} {GREY}{line}{RESET_COLOR} ",
            ));
        } else {
            output.push_str(&format!(
                " {line_a:>width_a$} {GREY}│{RESET_COLOR} {line_b:<width_b$} {GREY}│{RESET_COLOR} {GREY}{line}{RESET_COLOR} ",
            ));
        }
    }

    output.push('\n');
    output
}

fn compare_dates(first: &str, second: &str) -> &'static str {
    match first.cmp(second) {
        std::cmp::Ordering::Less => "=>",
        std::cmp::Ordering::Greater => "<=",
        std::cmp::Ordering::Equal => "==",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::console_common::clone_header;
    use crate::report::test_support::make_test_result_with_clone;

    #[test]
    fn console_full_header_matches_upstream_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let header = clone_header(&result.clones[0], &Options::default());

        assert!(header.starts_with("Clone found (javascript):\n - "));
        assert!(header.contains("src/a.js"));
        assert!(header.contains("[2:3 - 5:1] (3 lines, 18 tokens)"));
        assert!(header.contains("src/b.js"));
        assert!(header.contains("[8:1 - 11:1]"));
    }

    #[test]
    fn console_full_fragment_table_uses_source_fragment_lines() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let table = fragment_table(&result, &result.clones[0]);

        assert!(table.contains(" 2 "));
        assert!(table.contains(" 8 "));
        assert!(table.contains("alpha <beta> ]]>"));
    }

    #[test]
    fn console_full_fragment_table_uses_blame_columns_when_available() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.clones[0].duplication_a.blame = Some(
            [(
                "2".to_string(),
                crate::detector::BlamedLine {
                    rev: "a".to_string(),
                    author: "Alice".to_string(),
                    date: "2024-01-01 00:00:00 +0000".to_string(),
                    line: "2".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );
        result.clones[0].duplication_b.blame = Some(
            [(
                "8".to_string(),
                crate::detector::BlamedLine {
                    rev: "b".to_string(),
                    author: "Bob".to_string(),
                    date: "2024-01-02 00:00:00 +0000".to_string(),
                    line: "8".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        );

        let table = fragment_table(&result, &result.clones[0]);

        assert!(table.contains("Alice"));
        assert!(table.contains("Bob"));
        assert!(table.contains("=>"));
    }

    #[test]
    fn console_full_report_prints_final_clone_count() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let report = console_full_report(&result, &Options::default());

        assert!(report.contains("Clone found (javascript):"));
        assert!(report.contains("Found 1 clones."));
    }
}
