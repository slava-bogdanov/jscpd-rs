use std::cmp::Ordering;
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

    candidates.sort_by(|left, right| fast_glob_like_path_cmp(&left.path, &right.path));

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

fn fast_glob_like_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left_components = left.components().collect::<Vec<_>>();
    let right_components = right.components().collect::<Vec<_>>();
    let common_len = left_components.len().min(right_components.len());

    for idx in 0..common_len {
        let left_component = left_components[idx].as_os_str();
        let right_component = right_components[idx].as_os_str();
        if left_component == right_component {
            continue;
        }

        let left_remaining = left_components.len() - idx;
        let right_remaining = right_components.len() - idx;
        if left_remaining == 1 && right_remaining > 1 {
            return Ordering::Less;
        }
        if right_remaining == 1 && left_remaining > 1 {
            return Ordering::Greater;
        }

        return left_component
            .to_string_lossy()
            .cmp(&right_component.to_string_lossy());
    }

    left_components.len().cmp(&right_components.len())
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

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::path::Path;

    use super::fast_glob_like_path_cmp;

    #[test]
    fn fast_glob_like_order_places_parent_files_before_child_files() {
        assert_eq!(
            fast_glob_like_path_cmp(
                Path::new("pkg/tokenizer/src/tokenize.ts"),
                Path::new("pkg/tokenizer/src/languages/markdown-tokenizer.ts"),
            ),
            Ordering::Less
        );
        assert_eq!(
            fast_glob_like_path_cmp(
                Path::new("pkg/tokenizer/src/languages/astro.ts"),
                Path::new("pkg/tokenizer/src/languages/vue.ts"),
            ),
            Ordering::Less
        );
    }
}
