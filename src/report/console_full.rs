use super::source::{clone_fragment, source_location};
use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult};

const BOLD_GREEN: &str = "\x1b[1m\x1b[32m";
const GREY: &str = "\x1b[90m";
const RESET_COLOR: &str = "\x1b[39m";
const RESET_BOLD: &str = "\x1b[22m";

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

fn clone_header(clone: &CloneMatch, _options: &Options) -> String {
    let path_a = colored_path(&clone.duplication_a.source_id);
    let path_b = colored_path(&clone.duplication_b.source_id);
    format!(
        "Clone found ({}):\n - {} [{}] ({} lines, {} tokens)\n   {} [{}]\n",
        clone.format,
        path_a,
        source_location(&clone.duplication_a.start, &clone.duplication_a.end),
        clone
            .duplication_a
            .end
            .line
            .saturating_sub(clone.duplication_a.start.line),
        clone
            .duplication_a
            .end
            .position
            .saturating_sub(clone.duplication_a.start.position),
        path_b,
        source_location(&clone.duplication_b.start, &clone.duplication_b.end),
    )
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
        output.push_str(&format!(
            " {line_a:>width_a$} {GREY}│{RESET_COLOR} {line_b:<width_b$} {GREY}│{RESET_COLOR} {GREY}{line}{RESET_COLOR} ",
        ));
    }

    output.push('\n');
    output
}

fn colored_path(path: &str) -> String {
    format!("{BOLD_GREEN}{path}{RESET_COLOR}{RESET_BOLD}")
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn console_full_report_prints_final_clone_count() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let report = console_full_report(&result, &Options::default());

        assert!(report.contains("Clone found (javascript):"));
        assert!(report.contains("Found 1 clones."));
    }
}
