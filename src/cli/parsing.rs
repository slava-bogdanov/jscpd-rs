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
    let trimmed = value.trim();
    if let Some(bytes) = parse_bytes_unit(trimmed) {
        return Ok(bytes);
    }
    Ok(parse_js_int_bytes(trimmed))
}

fn parse_bytes_unit(value: &str) -> Option<u64> {
    let (number_part, rest) = split_decimal_prefix(value)?;
    let suffix = rest.trim_start().to_ascii_lowercase();
    let multiplier = match suffix.as_str() {
        "kb" => 1024.0,
        "mb" => 1024.0 * 1024.0,
        "gb" => 1024.0 * 1024.0 * 1024.0,
        "tb" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        "pb" => 1024.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    let number = number_part.parse::<f64>().ok()?;
    Some(float_bytes_to_u64(number * multiplier))
}

fn split_decimal_prefix(value: &str) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let mut idx = 0;
    if matches!(bytes.first(), Some(b'-' | b'+')) {
        idx = 1;
    }

    let digit_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == digit_start {
        return None;
    }

    if idx < bytes.len() && bytes[idx] == b'.' {
        let dot = idx;
        idx += 1;
        let fraction_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        if idx == fraction_start {
            idx = dot;
        }
    }

    Some((&value[..idx], &value[idx..]))
}

fn parse_js_int_bytes(value: &str) -> u64 {
    let bytes = value.as_bytes();
    let mut idx = 0;
    let negative = match bytes.first() {
        Some(b'-') => {
            idx = 1;
            true
        }
        Some(b'+') => {
            idx = 1;
            false
        }
        _ => false,
    };

    if negative {
        return 0;
    }

    let mut result = 0_u64;
    let mut saw_digit = false;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        saw_digit = true;
        result = result
            .saturating_mul(10)
            .saturating_add((bytes[idx] - b'0') as u64);
        idx += 1;
    }

    if saw_digit { result } else { 0 }
}

fn float_bytes_to_u64(bytes: f64) -> u64 {
    if !bytes.is_finite() || bytes <= 0.0 {
        return 0;
    }
    if bytes >= u64::MAX as f64 {
        return u64::MAX;
    }
    bytes.floor() as u64
}
