use crate::cli::Options;

pub(super) fn find_ignore_regions(content: &str, options: &Options) -> Vec<[usize; 2]> {
    let mut regions = Vec::new();
    let start_marker = "jscpd:ignore-start";
    let end_marker = "jscpd:ignore-end";
    let mut search_from = 0;

    while let Some(marker_start) = content[search_from..].find(start_marker) {
        let marker_start = search_from + marker_start;
        let line_start = content[..marker_start]
            .rfind('\n')
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let after_start = marker_start + start_marker.len();
        let Some(marker_end_rel) = content[after_start..].find(end_marker) else {
            break;
        };
        let marker_end = after_start + marker_end_rel;
        let line_end = content[marker_end..]
            .find('\n')
            .map(|idx| marker_end + idx)
            .unwrap_or(content.len());
        regions.push([line_start, line_end]);
        search_from = line_end;
    }

    for pattern in &options.ignore_pattern {
        regions.extend(pattern.find_iter(content).map(|m| [m.start(), m.end()]));
    }

    regions
}
