use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::cli::Options;
use crate::formats;

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub source_id: String,
    pub format: String,
    pub content: String,
    pub lines: usize,
}

#[derive(Clone, Debug)]
struct CandidateFile {
    path: std::path::PathBuf,
    format: String,
}

pub fn discover(options: &Options) -> Result<Vec<SourceFile>> {
    if options.debug {
        eprintln!("options: {options:#?}");
    }

    let pattern_set = build_glob_set(std::slice::from_ref(&options.pattern))
        .with_context(|| format!("invalid pattern `{}`", options.pattern))?;
    let ignore_set = build_glob_set(&options.ignore).context("invalid ignore pattern")?;
    let mut candidates = Vec::new();

    for root in &options.paths {
        let metadata = fs::metadata(root)
            .with_context(|| format!("failed to inspect path `{}`", root.display()))?;
        if metadata.is_file() {
            collect_candidate(root, options, &ignore_set, &mut candidates)?;
            continue;
        }

        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(false)
            .git_ignore(options.gitignore)
            .git_exclude(options.gitignore)
            .git_global(options.gitignore)
            .follow_links(!options.no_symlinks);

        for entry in builder.build() {
            let entry =
                entry.with_context(|| format!("failed to walk path `{}`", root.display()))?;
            let Some(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap_or(path);
            if !pattern_set.is_match(relative) {
                continue;
            }
            collect_candidate(path, options, &ignore_set, &mut candidates)?;
        }
    }

    candidates.sort_by(|left, right| left.path.cmp(&right.path));

    let mut files = candidates
        .into_par_iter()
        .enumerate()
        .map(|(idx, candidate)| {
            read_candidate(candidate, options).map(|file| file.map(|file| (idx, file)))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    files.sort_by_key(|(idx, _)| *idx);

    Ok(files.into_iter().map(|(_, file)| file).collect())
}

fn collect_candidate(
    path: &Path,
    options: &Options,
    ignore_set: &GlobSet,
    candidates: &mut Vec<CandidateFile>,
) -> Result<()> {
    if is_ignored(path, ignore_set) {
        return Ok(());
    }

    let Some(format) =
        formats::format_for_path(path, &options.formats_exts, &options.formats_names)
    else {
        if options.verbose {
            eprintln!("skipped unsupported format: {}", path.display());
        }
        return Ok(());
    };
    if let Some(formats) = &options.formats
        && !formats.contains(format)
    {
        return Ok(());
    }

    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect file `{}`", path.display()))?;
    if metadata.len() > options.max_size_bytes {
        if options.verbose {
            eprintln!(
                "skipped large file: {} ({} > {})",
                path.display(),
                metadata.len(),
                options.max_size_bytes
            );
        }
        return Ok(());
    }

    candidates.push(CandidateFile {
        path: path.to_path_buf(),
        format: format.to_string(),
    });

    Ok(())
}

fn read_candidate(candidate: CandidateFile, options: &Options) -> Result<Option<SourceFile>> {
    let bytes = fs::read(&candidate.path)
        .with_context(|| format!("failed to read `{}`", candidate.path.display()))?;
    let content = String::from_utf8_lossy(&bytes).into_owned();
    let lines = if content.is_empty() {
        0
    } else {
        content
            .as_bytes()
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
            + 1
    };
    if lines < options.min_lines || lines > options.max_lines {
        return Ok(None);
    }

    let source_id = if options.absolute {
        candidate
            .path
            .canonicalize()
            .unwrap_or_else(|_| candidate.path.clone())
            .display()
            .to_string()
    } else {
        candidate.path.display().to_string()
    };

    Ok(Some(SourceFile {
        source_id,
        format: candidate.format,
        content,
        lines,
    }))
}

fn is_ignored(path: &Path, ignore_set: &GlobSet) -> bool {
    if ignore_set.is_empty() {
        return false;
    }
    if ignore_set.is_match(path) {
        return true;
    }
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    path.strip_prefix(cwd)
        .map(|relative| ignore_set.is_match(relative))
        .unwrap_or(false)
}

fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    if patterns.is_empty() {
        return Ok(builder.build()?);
    }
    for pattern in patterns {
        builder.add(Glob::new(pattern)?);
    }
    Ok(builder.build()?)
}
