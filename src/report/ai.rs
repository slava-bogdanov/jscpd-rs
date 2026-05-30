use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult};

pub(super) fn write(result: &DetectionResult, options: &Options) {
    if options.silent {
        return;
    }

    println!("Clones:");
    for clone in &result.clones {
        println!("{}", clone_line(clone));
    }
    println!("---");
    println!(
        "{} clones · {:.1}% duplication",
        result.clones.len(),
        result.statistics.total.percentage
    );
}

fn clone_line(clone: &CloneMatch) -> String {
    compress_clone_line(
        &clone.duplication_a.source_id,
        &clone.duplication_b.source_id,
        &format_range(clone.duplication_a.start.line, clone.duplication_a.end.line),
        &format_range(clone.duplication_b.start.line, clone.duplication_b.end.line),
    )
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn format_range(start: usize, end: usize) -> String {
    format!("{start}-{end}")
}

fn compress_clone_line(path_a: &str, path_b: &str, range_a: &str, range_b: &str) -> String {
    let norm_a = normalize_path(path_a);
    let norm_b = normalize_path(path_b);

    if norm_a == norm_b {
        return format!("{norm_a} {range_a} ~ {range_b}");
    }

    let parts_a = norm_a.split('/').collect::<Vec<_>>();
    let parts_b = norm_b.split('/').collect::<Vec<_>>();
    let mut common_len = 0;
    let min_len = parts_a.len().min(parts_b.len());
    while common_len < min_len.saturating_sub(1) && parts_a[common_len] == parts_b[common_len] {
        common_len += 1;
    }

    if common_len == 0 {
        return format!("{norm_a}:{range_a} ~ {norm_b}:{range_b}");
    }

    let prefix = parts_a[..common_len].join("/");
    let rem_a = parts_a[common_len..].join("/");
    let rem_b = parts_b[common_len..].join("/");
    format!("{prefix}/ {rem_a}:{range_a} ~ {rem_b}:{range_b}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_clone;

    #[test]
    fn compress_clone_line_same_file_matches_upstream() {
        assert_eq!(
            compress_clone_line("src/utils/auth.ts", "src/utils/auth.ts", "10-25", "80-95"),
            "src/utils/auth.ts 10-25 ~ 80-95"
        );
    }

    #[test]
    fn compress_clone_line_same_directory_matches_upstream() {
        assert_eq!(
            compress_clone_line(
                "src/utils/auth.ts",
                "src/utils/helpers.ts",
                "10-25",
                "40-55"
            ),
            "src/utils/ auth.ts:10-25 ~ helpers.ts:40-55"
        );
    }

    #[test]
    fn compress_clone_line_cross_directory_matches_upstream() {
        assert_eq!(
            compress_clone_line("src/utils/auth.ts", "src/api/routes.ts", "10-25", "5-20"),
            "src/ utils/auth.ts:10-25 ~ api/routes.ts:5-20"
        );
    }

    #[test]
    fn compress_clone_line_no_common_prefix_matches_upstream() {
        assert_eq!(
            compress_clone_line("apps/a/foo.ts", "packages/b/bar.ts", "1-10", "5-15"),
            "apps/a/foo.ts:1-10 ~ packages/b/bar.ts:5-15"
        );
    }

    #[test]
    fn compress_clone_line_normalizes_windows_paths() {
        assert_eq!(
            compress_clone_line(
                "src\\utils\\auth.ts",
                "src\\utils\\helpers.ts",
                "10-25",
                "40-55"
            ),
            "src/utils/ auth.ts:10-25 ~ helpers.ts:40-55"
        );
    }

    #[test]
    fn clone_line_uses_clone_ranges() {
        let clone = make_test_clone("src/a.js", "src/b.js");

        assert_eq!(clone_line(&clone), "src/ a.js:2-5 ~ b.js:8-11");
    }
}
