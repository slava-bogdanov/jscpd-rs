use super::summary::silent_summary;
use crate::detector::DetectionResult;

pub(super) fn write(result: &DetectionResult) {
    println!("{}", silent_summary(result));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;

    #[test]
    fn silent_reporter_matches_upstream_summary_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        assert_eq!(
            silent_summary(&result),
            "Duplications detection: Found 1 exact clones with 5(25%) duplicated lines in 2 (1 formats) files."
        );
    }
}
