use anyhow::{Context, Result};
use regex::Regex;

use super::FormatMappings;

pub(super) fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn parse_format_mappings(value: &str) -> FormatMappings {
    let mappings = value
        .split(';')
        .filter_map(|entry| {
            let (format, values) = entry.split_once(':')?;
            let values = split_csv(values);
            (!format.trim().is_empty() && !values.is_empty())
                .then(|| (format.trim().to_string(), values))
        })
        .collect();
    FormatMappings(mappings)
}

pub(super) fn compile_patterns(patterns: Vec<String>) -> Result<Vec<Regex>> {
    patterns
        .into_iter()
        .map(|pattern| Regex::new(&pattern).with_context(|| format!("invalid regex `{pattern}`")))
        .collect()
}

pub(super) fn parse_size(value: &str) -> Result<u64> {
    let trimmed = value.trim().to_ascii_lowercase();
    let mut split_at = 0;
    let mut saw_dot = false;
    for (idx, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() {
            split_at = idx + ch.len_utf8();
        } else if ch == '.' && !saw_dot {
            saw_dot = true;
            split_at = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    let number_part = &trimmed[..split_at];
    if number_part.is_empty() || number_part.starts_with('.') {
        anyhow::bail!("missing numeric size in `{value}`");
    }
    let number = number_part
        .parse::<f64>()
        .with_context(|| format!("missing numeric size in `{value}`"))?;
    let suffix = trimmed[split_at..].trim();
    let multiplier = match suffix {
        "" | "b" => 1.0,
        "k" | "kb" => 1024.0,
        "m" | "mb" => 1024.0 * 1024.0,
        "g" | "gb" => 1024.0 * 1024.0 * 1024.0,
        _ => anyhow::bail!("unsupported size suffix `{suffix}`"),
    };
    let bytes = (number * multiplier).floor();
    if !bytes.is_finite() || bytes < 0.0 {
        anyhow::bail!("invalid size `{value}`");
    }
    Ok(bytes as u64)
}
