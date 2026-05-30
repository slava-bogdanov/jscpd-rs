use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;

use crate::cli::Options;
use crate::formats;

use super::SourceFile;
use super::gitignore::{collect_cwd_gitignore_patterns, collect_gitignore_patterns};
use super::paths::{display_relative_to, fast_glob_like_path_cmp};
use super::shebang::shebang_format_for_path;

#[derive(Clone, Debug)]
struct CandidateFile {
    path: PathBuf,
    format: String,
    root_index: usize,
}

struct CandidateCollectionContext<'a> {
    root: &'a Path,
    root_index: usize,
    options: &'a Options,
    pattern_set: &'a GlobSet,
    ignore_set: &'a IgnoreMatcher,
    cwd: &'a Path,
}

pub fn discover(options: &Options) -> Result<Vec<SourceFile>> {
    let pattern_set = build_glob_set(std::slice::from_ref(&options.pattern))
        .with_context(|| format!("invalid pattern `{}`", options.pattern))?;
    let needs_compat_discovery = options
        .reporters
        .iter()
        .any(|reporter| reporter_needs_report_paths(reporter))
        || !options.silent;
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    let mut explicit_ignore = options.ignore.clone();
    if options.gitignore {
        explicit_ignore.extend(collect_cwd_gitignore_patterns(&cwd));
    }
    let mut ignore_patterns = normalize_ignore_patterns(&explicit_ignore, &options.paths, &cwd);
    if options.gitignore && needs_compat_discovery {
        ignore_patterns.extend(collect_gitignore_patterns(&options.paths));
    }
    let ignore_set =
        Arc::new(build_ignore_matcher(&ignore_patterns).context("invalid ignore pattern")?);
    let mut candidates = Vec::new();

    for (root_index, root) in options.paths.iter().enumerate() {
        if options.no_symlinks && is_symlink(root) {
            continue;
        }
        let metadata = fs::metadata(root)
            .with_context(|| format!("failed to inspect path `{}`", root.display()))?;
        if metadata.is_file() {
            collect_candidate(
                root,
                root_index,
                options,
                &ignore_set,
                &cwd,
                &mut candidates,
            )?;
            continue;
        }

        let mut builder = WalkBuilder::new(root);
        builder
            .hidden(false)
            .ignore(!needs_compat_discovery)
            .git_ignore(options.gitignore && !needs_compat_discovery)
            .git_exclude(options.gitignore)
            .git_global(options.gitignore)
            .follow_links(!options.no_symlinks);

        if needs_compat_discovery {
            let root_path = root.clone();
            let walk_ignore_set = Arc::clone(&ignore_set);
            let has_negations = walk_ignore_set.has_negations();
            let walk_cwd = cwd.clone();
            builder.filter_entry(move |entry| {
                entry.path() == root_path
                    || !entry
                        .file_type()
                        .is_some_and(|file_type| file_type.is_dir())
                    || has_negations
                    || !is_ignored(entry.path(), &walk_ignore_set, &walk_cwd)
            });
        }

        let collection_context = CandidateCollectionContext {
            root,
            root_index,
            options,
            pattern_set: &pattern_set,
            ignore_set: ignore_set.as_ref(),
            cwd: &cwd,
        };

        if parallel_walk_enabled(options) {
            collect_candidates_parallel(&builder, &collection_context, &mut candidates)?;
        } else {
            collect_candidates_sequential(&builder, &collection_context, &mut candidates)?;
        }
    }

    candidates.sort_by(|left, right| {
        left.root_index
            .cmp(&right.root_index)
            .then_with(|| fast_glob_like_path_cmp(&left.path, &right.path))
    });

    let mut files = candidates
        .into_par_iter()
        .enumerate()
        .map(|(idx, candidate)| {
            read_candidate(candidate, options, &cwd, needs_compat_discovery)
                .map(|file| file.map(|file| (idx, file)))
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

fn parallel_walk_enabled(options: &Options) -> bool {
    !options.debug && !options.verbose
}

fn collect_candidates_sequential(
    builder: &WalkBuilder,
    context: &CandidateCollectionContext<'_>,
    candidates: &mut Vec<CandidateFile>,
) -> Result<()> {
    for entry in builder.build() {
        let entry =
            entry.with_context(|| format!("failed to walk path `{}`", context.root.display()))?;
        let Some(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path.strip_prefix(context.root).unwrap_or(path);
        if !context.pattern_set.is_match(relative) {
            continue;
        }
        collect_candidate(
            path,
            context.root_index,
            context.options,
            context.ignore_set,
            context.cwd,
            candidates,
        )?;
    }

    Ok(())
}

fn collect_candidates_parallel(
    builder: &WalkBuilder,
    context: &CandidateCollectionContext<'_>,
    candidates: &mut Vec<CandidateFile>,
) -> Result<()> {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let error = Arc::new(Mutex::new(None));

    builder.build_parallel().run(|| {
        let collected = Arc::clone(&collected);
        let error = Arc::clone(&error);
        Box::new(move |entry| {
            if error.lock().unwrap().is_some() {
                return WalkState::Quit;
            }

            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    *error.lock().unwrap() = Some(anyhow!(
                        "failed to walk path `{}`: {err}",
                        context.root.display()
                    ));
                    return WalkState::Quit;
                }
            };

            let Some(file_type) = entry.file_type() else {
                return WalkState::Continue;
            };
            if !file_type.is_file() {
                return WalkState::Continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(context.root).unwrap_or(path);
            if !context.pattern_set.is_match(relative) {
                return WalkState::Continue;
            }

            let mut local = Vec::with_capacity(1);
            if let Err(err) = collect_candidate(
                path,
                context.root_index,
                context.options,
                context.ignore_set,
                context.cwd,
                &mut local,
            ) {
                *error.lock().unwrap() = Some(err);
                return WalkState::Quit;
            }
            if !local.is_empty() {
                collected.lock().unwrap().extend(local);
            }
            WalkState::Continue
        })
    });

    if let Some(error) = Arc::try_unwrap(error).unwrap().into_inner().unwrap() {
        return Err(error);
    }

    candidates.extend(Arc::try_unwrap(collected).unwrap().into_inner().unwrap());

    Ok(())
}

fn collect_candidate(
    path: &Path,
    root_index: usize,
    options: &Options,
    ignore_set: &IgnoreMatcher,
    cwd: &Path,
    candidates: &mut Vec<CandidateFile>,
) -> Result<()> {
    if options.no_symlinks && is_symlink(path) {
        return Ok(());
    }
    if is_ignored(path, ignore_set, cwd) {
        return Ok(());
    }

    let format = if let Some(format) =
        formats::format_for_path(path, &options.formats_exts, &options.formats_names)
    {
        Some(format.to_string())
    } else {
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
        shebang_format_for_path(path, &metadata)?.map(str::to_string)
    };
    let Some(format) = format else {
        if options.verbose {
            eprintln!("skipped unsupported format: {}", path.display());
        }
        return Ok(());
    };
    if let Some(formats) = &options.formats
        && !formats.contains(format.as_str())
    {
        if options.verbose || options.debug {
            println!("{}", format_filter_skip_message(path, &format, cwd));
        }
        return Ok(());
    }
    candidates.push(CandidateFile {
        path: path.to_path_buf(),
        format,
        root_index,
    });

    Ok(())
}

fn read_candidate(
    candidate: CandidateFile,
    options: &Options,
    cwd: &Path,
    needs_report_paths: bool,
) -> Result<Option<SourceFile>> {
    let bytes = fs::read(&candidate.path)
        .with_context(|| format!("failed to read `{}`", candidate.path.display()))?;
    if bytes.len() as u64 > options.max_size_bytes {
        if options.verbose {
            eprintln!(
                "skipped large file: {} ({} > {})",
                candidate.path.display(),
                bytes.len(),
                options.max_size_bytes
            );
        }
        return Ok(None);
    }
    let lines = count_lines(&bytes);
    if lines < options.min_lines || lines > options.max_lines {
        return Ok(None);
    }
    let content = decode_source(bytes);

    let source_id = if options.absolute {
        candidate
            .path
            .canonicalize()
            .unwrap_or_else(|_| candidate.path.clone())
            .display()
            .to_string()
    } else if !needs_report_paths {
        candidate.path.display().to_string()
    } else {
        display_relative_to(&candidate.path, cwd)
    };

    Ok(Some(SourceFile {
        source_id,
        format: candidate.format,
        content,
    }))
}

pub(super) fn decode_source(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
    }
}

pub(super) fn count_lines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| **byte == b'\n').count() + 1
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn reporter_needs_report_paths(reporter: &str) -> bool {
    matches!(reporter, "json" | "xml" | "html" | "sarif" | "xcode")
}

fn normalize_ignore_patterns(patterns: &[String], roots: &[PathBuf], cwd: &Path) -> Vec<String> {
    let scan_dirs = roots
        .iter()
        .map(|root| scan_dir_for_root(root))
        .collect::<Vec<_>>();
    let mut normalized = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for pattern in patterns {
        let path = Path::new(pattern);
        if path.is_absolute() || pattern.starts_with("**/") {
            push_pattern_once(&mut normalized, &mut seen, pattern.clone());
            continue;
        }

        push_pattern_once(&mut normalized, &mut seen, pattern.clone());
        for scan_dir in scan_dirs.iter().chain(std::iter::once(&PathBuf::from("."))) {
            push_pattern_once(
                &mut normalized,
                &mut seen,
                normalize_glob_path(scan_dir.join(pattern)),
            );
            push_pattern_once(
                &mut normalized,
                &mut seen,
                normalize_glob_path(cwd.join(scan_dir).join(pattern)),
            );
        }
    }

    normalized
}

fn scan_dir_for_root(root: &Path) -> PathBuf {
    match fs::canonicalize(root) {
        Ok(real_path) if real_path.is_file() => root
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
        _ => root.to_path_buf(),
    }
}

fn push_pattern_once(
    patterns: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    pattern: String,
) {
    if seen.insert(pattern.clone()) {
        patterns.push(pattern);
    }
}

fn normalize_glob_path(path: PathBuf) -> String {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized.display().to_string()
}

pub(super) fn format_filter_skip_message(path: &Path, format: &str, cwd: &Path) -> String {
    format!(
        "File {} skipped! Format \"{}\" does not included to supported formats.",
        display_relative_to(path, cwd),
        format
    )
}

fn is_ignored(path: &Path, ignore_set: &IgnoreMatcher, cwd: &Path) -> bool {
    if ignore_set.is_empty() {
        return false;
    }
    if ignore_set.is_match(path) {
        return true;
    }
    path.strip_prefix(cwd)
        .map(|relative| ignore_set.is_match(relative))
        .unwrap_or(false)
}

struct IgnoreMatcher {
    ignored: GlobSet,
    negated: GlobSet,
}

impl IgnoreMatcher {
    fn is_empty(&self) -> bool {
        self.ignored.is_empty()
    }

    fn has_negations(&self) -> bool {
        !self.negated.is_empty()
    }

    fn is_match(&self, path: &Path) -> bool {
        self.ignored.is_match(path) && !self.negated.is_match(path)
    }
}

fn build_ignore_matcher(patterns: &[String]) -> Result<IgnoreMatcher> {
    let mut ignored = GlobSetBuilder::new();
    let mut negated = GlobSetBuilder::new();
    for pattern in patterns {
        if let Some(pattern) = pattern.strip_prefix('!') {
            negated.add(Glob::new(pattern)?);
        } else {
            ignored.add(Glob::new(pattern)?);
        }
    }
    Ok(IgnoreMatcher {
        ignored: ignored.build()?,
        negated: negated.build()?,
    })
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
