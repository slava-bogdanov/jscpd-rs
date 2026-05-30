use super::source::absolute_report_path;
use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult};

pub(super) fn write(result: &DetectionResult, options: &Options) {
    for clone in &result.clones {
        println!("{}", XcodeWarning::from_clone(clone, options));
    }
    println!("Found {} clones.", result.clones.len());
}

struct XcodeWarning {
    message: String,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_clone;

    #[test]
    fn xcode_warning_matches_upstream_shape() {
        let options = Options::default();
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
}
