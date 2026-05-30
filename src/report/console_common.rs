use super::source::source_location;
use crate::cli::Options;
use crate::detector::CloneMatch;

pub(super) const BOLD: &str = "\x1b[1m";
pub(super) const BOLD_GREEN: &str = "\x1b[1m\x1b[32m";
pub(super) const GREY: &str = "\x1b[90m";
pub(super) const RED: &str = "\x1b[31m";
pub(super) const RESET_BOLD: &str = "\x1b[22m";
pub(super) const RESET_COLOR: &str = "\x1b[39m";

pub(super) fn clone_header(clone: &CloneMatch, _options: &Options) -> String {
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

fn colored_path(path: &str) -> String {
    format!("{BOLD_GREEN}{path}{RESET_COLOR}{RESET_BOLD}")
}
