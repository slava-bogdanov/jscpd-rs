use serde::Serialize;
use xxhash_rust::xxh3::xxh3_128;

use crate::cli::{Mode, Options};

#[derive(Clone, Debug, Serialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub position: usize,
}

#[derive(Clone, Debug)]
pub struct DetectionToken {
    pub hash: u128,
    pub start: Location,
    pub end: Location,
    pub range: [usize; 2],
}

pub fn tokenize_for_detection(content: &str, options: &Options) -> Vec<DetectionToken> {
    let ignore_regions = find_ignore_regions(content);
    let mut tokens = Vec::new();
    let mut line = 1usize;
    let mut column = 1usize;
    let mut chars = content.char_indices().peekable();

    while let Some((start_byte, ch)) = chars.next() {
        if ch.is_whitespace() {
            advance_position(ch, &mut line, &mut column);
            continue;
        }

        let start = Location {
            line,
            column,
            position: start_byte,
        };
        let mut end_byte = start_byte + ch.len_utf8();
        let mut end_line;
        let mut end_column;
        advance_position(ch, &mut line, &mut column);
        end_line = line;
        end_column = column;

        while let Some((next_byte, next_ch)) = chars.peek().copied() {
            if next_ch.is_whitespace() {
                break;
            }
            chars.next();
            advance_position(next_ch, &mut line, &mut column);
            end_byte = next_byte + next_ch.len_utf8();
            end_line = line;
            end_column = column;
        }

        let value = &content[start_byte..end_byte];
        if options.mode == Mode::Weak && is_commentish(value) {
            continue;
        }

        if overlaps_ignore_region(start_byte, end_byte, &ignore_regions) {
            continue;
        }

        tokens.push(DetectionToken {
            hash: hash_token(value, options.ignore_case),
            start,
            end: Location {
                line: end_line,
                column: end_column,
                position: end_byte,
            },
            range: [start_byte, end_byte],
        });
    }

    tokens
}

fn advance_position(ch: char, line: &mut usize, column: &mut usize) {
    if ch == '\n' {
        *line += 1;
        *column = 1;
    } else {
        *column += 1;
    }
}

fn is_commentish(value: &str) -> bool {
    value.starts_with("//")
        || value.starts_with("/*")
        || value.starts_with('*')
        || value.starts_with('#')
        || value.starts_with("<!--")
}

fn hash_token(value: &str, ignore_case: bool) -> u128 {
    if ignore_case {
        xxh3_128(value.to_lowercase().as_bytes())
    } else {
        xxh3_128(value.as_bytes())
    }
}

fn find_ignore_regions(content: &str) -> Vec<[usize; 2]> {
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

    regions
}

fn overlaps_ignore_region(start: usize, end: usize, regions: &[[usize; 2]]) -> bool {
    regions
        .iter()
        .any(|[region_start, region_end]| start < *region_end && end > *region_start)
}

#[cfg(test)]
mod tests {
    use crate::cli::Options;

    #[test]
    fn tokenizes_non_whitespace_tokens_with_locations() {
        let tokens = super::tokenize_for_detection("let a = 1;\nlet b = 2;", &Options::default());
        assert_eq!(tokens[0].start.line, 1);
        assert_eq!(tokens[4].start.line, 2);
    }

    #[test]
    fn skips_ignore_regions() {
        let content = "keep\n// jscpd:ignore-start\nskip\n// jscpd:ignore-end\nkeep2\n";
        let tokens = super::tokenize_for_detection(content, &Options::default());
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn detection_tokenizer_avoids_token_value_allocations() {
        let tokens = super::tokenize_for_detection("let a = 1;\nlet b = 2;", &Options::default());
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].start.line, 1);
        assert_eq!(tokens[4].start.line, 2);
    }
}
