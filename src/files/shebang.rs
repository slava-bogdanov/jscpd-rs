use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result};

pub(super) fn shebang_format_for_path(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<Option<&'static str>> {
    if !is_executable(metadata) || is_symlink(path) {
        return Ok(None);
    }

    let mut file =
        fs::File::open(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let mut buf = [0u8; 128];
    let read = file
        .read(&mut buf)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    let head = String::from_utf8_lossy(&buf[..read]);
    let Some(first_line) = head.lines().next() else {
        return Ok(None);
    };
    if !first_line.starts_with("#!") {
        return Ok(None);
    }

    let mut tokens = first_line[2..].split_whitespace();
    let Some(first_token) = tokens.next() else {
        return Ok(None);
    };
    let interpreter = if Path::new(first_token)
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with("env"))
    {
        let Some(second_token) = tokens.next() else {
            return Ok(None);
        };
        if second_token.starts_with('-') {
            return Ok(None);
        }
        second_token
    } else {
        first_token
    };

    let Some(raw_name) = Path::new(interpreter).file_name() else {
        return Ok(None);
    };
    let raw_name = raw_name.to_string_lossy();
    if raw_name.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        return Ok(None);
    }

    Ok(shebang_name_to_format(&normalize_shebang_name(&raw_name)))
}

fn shebang_name_to_format(name: &str) -> Option<&'static str> {
    match name {
        "bash" | "sh" | "zsh" | "dash" | "ksh" => Some("bash"),
        "python" => Some("python"),
        "ruby" => Some("ruby"),
        "perl" => Some("perl"),
        "php" => Some("php"),
        "node" | "nodejs" => Some("javascript"),
        "lua" => Some("lua"),
        "tclsh" | "wish" => Some("tcl"),
        "groovy" => Some("groovy"),
        "awk" | "gawk" | "nawk" => Some("awk"),
        "rscript" => Some("r"),
        _ => None,
    }
}

fn normalize_shebang_name(raw_name: &str) -> String {
    let mut end = raw_name.len();
    if raw_name.as_bytes().last().is_some_and(u8::is_ascii_digit) {
        while end > 0
            && raw_name.as_bytes()[end - 1].is_ascii()
            && (raw_name.as_bytes()[end - 1].is_ascii_digit()
                || raw_name.as_bytes()[end - 1] == b'.')
        {
            end -= 1;
        }
    }
    raw_name[..end].to_ascii_lowercase()
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &fs::Metadata) -> bool {
    false
}
