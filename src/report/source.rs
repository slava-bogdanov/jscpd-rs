use std::path::Path;

use crate::detector::{DetectionResult, Fragment};
use crate::tokenizer::Location;

pub(super) fn slice_range(content: &str, range: [usize; 2]) -> String {
    let start = range[0].min(content.len());
    let end = range[1].min(content.len());
    content.get(start..end).unwrap_or_default().to_string()
}

pub(super) fn clone_fragment(result: &DetectionResult, fragment: &Fragment) -> String {
    result
        .source_contents
        .get(&fragment.source_id)
        .map(|content| slice_range(content, fragment.range))
        .unwrap_or_default()
}

pub(super) fn source_location(start: &Location, end: &Location) -> String {
    format!(
        "{}:{} - {}:{}",
        start.line, start.column, end.line, end.column
    )
}

pub(super) fn absolute_report_path(source_id: &str) -> String {
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
