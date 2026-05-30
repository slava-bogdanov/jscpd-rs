pub(super) fn scan_line_comment(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start + 2;
    while idx < limit && bytes[idx] != b'\n' {
        idx += 1;
    }
    idx
}

pub(super) fn scan_block_comment(bytes: &[u8], start: usize, limit: usize) -> usize {
    let mut idx = start + 2;
    while idx + 1 < limit {
        if bytes[idx] == b'*' && bytes[idx + 1] == b'/' {
            return idx + 2;
        }
        idx += 1;
    }
    limit
}

pub(super) fn has_code_in_gap(content: &str, start: usize, end: usize) -> bool {
    let bytes = content.as_bytes();
    let mut idx = start;
    while idx < end {
        let ch = content[idx..].chars().next().unwrap_or('\0');
        if ch.is_whitespace() {
            idx += ch.len_utf8();
        } else if idx + 1 < end && bytes[idx] == b'/' && bytes[idx + 1] == b'/' {
            idx = scan_line_comment(bytes, idx, end);
        } else if idx + 1 < end && bytes[idx] == b'/' && bytes[idx + 1] == b'*' {
            idx = scan_block_comment(bytes, idx, end);
        } else {
            return true;
        }
    }
    false
}

pub(super) fn count_prism_whitespace_tokens(content: &str, start: usize, end: usize) -> usize {
    let bytes = content.as_bytes();
    let mut idx = start;
    let mut count = 0usize;

    while idx < end {
        match bytes[idx] {
            b'\n' => {
                count += 1;
                idx += 1;
            }
            b' ' | b'\t' | b'\r' | b'\x0c' | b'\x0b' => {
                count += 1;
                idx += 1;
                while idx < end && matches!(bytes[idx], b' ' | b'\t' | b'\r' | b'\x0c' | b'\x0b') {
                    idx += 1;
                }
            }
            _ => idx += 1,
        }
    }

    count
}
