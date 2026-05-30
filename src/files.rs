use std::fs;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::cli::Options;
use crate::formats;

mod gitignore;
mod paths;
mod shebang;

use gitignore::collect_gitignore_patterns;
use paths::{display_relative_to, fast_glob_like_path_cmp};
use shebang::shebang_format_for_path;

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
    let needs_compat_discovery = options
        .reporters
        .iter()
        .any(|reporter| reporter_needs_report_paths(reporter))
        || !options.silent;
    let mut ignore_patterns = options.ignore.clone();
    if options.gitignore && needs_compat_discovery {
        ignore_patterns.extend(collect_gitignore_patterns(&options.paths));
    }
    let ignore_set = Arc::new(build_glob_set(&ignore_patterns).context("invalid ignore pattern")?);
    let mut candidates = Vec::new();
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;

    for root in &options.paths {
        let metadata = fs::metadata(root)
            .with_context(|| format!("failed to inspect path `{}`", root.display()))?;
        if metadata.is_file() {
            collect_candidate(root, options, &ignore_set, &cwd, &mut candidates)?;
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
            let walk_cwd = cwd.clone();
            builder.filter_entry(move |entry| {
                entry.path() == root_path
                    || !entry
                        .file_type()
                        .is_some_and(|file_type| file_type.is_dir())
                    || !is_ignored(entry.path(), &walk_ignore_set, &walk_cwd)
            });
        }

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
            collect_candidate(path, options, &ignore_set, &cwd, &mut candidates)?;
        }
    }

    candidates.sort_by(|left, right| fast_glob_like_path_cmp(&left.path, &right.path));

    let mut files = candidates
        .into_par_iter()
        .enumerate()
        .map(|(idx, candidate)| {
            read_candidate(candidate, options, &cwd).map(|file| file.map(|file| (idx, file)))
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
    cwd: &Path,
    candidates: &mut Vec<CandidateFile>,
) -> Result<()> {
    if is_ignored(path, ignore_set, cwd) {
        return Ok(());
    }

    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to inspect file `{}`", path.display()))?;
    let format = if let Some(format) =
        formats::format_for_path(path, &options.formats_exts, &options.formats_names)
    {
        Some(format.to_string())
    } else {
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
        return Ok(());
    }

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
        format,
    });

    Ok(())
}

fn read_candidate(
    candidate: CandidateFile,
    options: &Options,
    cwd: &Path,
) -> Result<Option<SourceFile>> {
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

    let needs_report_paths = options
        .reporters
        .iter()
        .any(|reporter| reporter_needs_report_paths(reporter))
        || !options.silent;
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

fn reporter_needs_report_paths(reporter: &str) -> bool {
    matches!(reporter, "json" | "xml" | "html" | "sarif" | "xcode")
}

fn is_ignored(path: &Path, ignore_set: &GlobSet, cwd: &Path) -> bool {
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
    use std::collections::HashSet;
    use std::path::Path;

    use crate::cli::Options;

    use super::gitignore::gitignore_line_to_globs;
    use super::paths::relative_path;
    use super::{discover, display_relative_to, fast_glob_like_path_cmp};

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
        assert_eq!(
            fast_glob_like_path_cmp(
                Path::new("../dream/landing/.next/types/validator.ts"),
                Path::new("../dream/landing/.next/dev/types/validator.ts"),
            ),
            Ordering::Less
        );
    }

    #[test]
    fn relative_path_formats_sibling_paths_like_upstream() {
        assert_eq!(
            relative_path(
                Path::new("/home/dev/dream/file.ts"),
                Path::new("/home/dev/jscpd-rs")
            )
            .unwrap(),
            Path::new("../dream/file.ts")
        );
    }

    #[test]
    fn gitignore_line_to_globs_anchors_rooted_patterns_to_base_dir() {
        let globs = gitignore_line_to_globs("/node_modules/", Some(Path::new("/repo/app")));
        assert!(globs.iter().any(|glob| glob == "/repo/app/node_modules"));
        assert!(globs.iter().any(|glob| glob == "/repo/app/node_modules/**"));
    }

    #[cfg(unix)]
    #[test]
    fn discovers_executable_node_shebang_without_extension() {
        use std::os::unix::fs::PermissionsExt;

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "jscpd-rs-node-shebang-{}-{nonce}",
            std::process::id()
        ));
        std::fs::write(&path, "#!/usr/bin/env node\nconsole.log(1);\n").unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();

        let options = Options {
            paths: vec![path.clone()],
            formats: Some(HashSet::from(["javascript".to_string()])),
            min_lines: 1,
            reporters: vec!["json".to_string()],
            silent: true,
            gitignore: false,
            ..Options::default()
        };

        let files = discover(&options).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].format, "javascript");
    }

    #[test]
    fn discovers_common_non_native_formats() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("jscpd-rs-formats-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("style.css"), "body { color: red; }\n").unwrap();
        std::fs::write(dir.join("index.html"), "<main>hello</main>\n").unwrap();
        std::fs::write(dir.join("config.yaml"), "enabled: true\n").unwrap();
        std::fs::write(dir.join("settings.toml"), "enabled = true\n").unwrap();
        std::fs::write(dir.join("Component.vue"), "<template><div /></template>\n").unwrap();

        let options = Options {
            paths: vec![dir.clone()],
            min_lines: 1,
            reporters: vec!["json".to_string()],
            silent: true,
            gitignore: false,
            ..Options::default()
        };

        let files = discover(&options).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let formats = files
            .iter()
            .map(|file| file.format.as_str())
            .collect::<HashSet<_>>();

        assert!(formats.contains("css"));
        assert!(formats.contains("markup"));
        assert!(formats.contains("yaml"));
        assert!(formats.contains("toml"));
        assert!(formats.contains("vue"));
    }

    #[test]
    fn discovers_custom_extension_mappings() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "jscpd-rs-custom-exts-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("component.foo"), "const answer = 42;\n").unwrap();

        let options = Options {
            paths: vec![dir.clone()],
            min_lines: 1,
            reporters: vec!["json".to_string()],
            silent: true,
            gitignore: false,
            formats_exts: crate::cli::FormatMappings::from_pairs(vec![(
                "javascript".to_string(),
                vec!["foo".to_string()],
            )]),
            ..Options::default()
        };

        let files = discover(&options).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].format, "javascript");
    }

    #[test]
    fn discovers_custom_extensionless_name_mappings() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "jscpd-rs-custom-names-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Recipe"), "target:\n\tprintf ok\n").unwrap();

        let options = Options {
            paths: vec![dir.clone()],
            min_lines: 1,
            reporters: vec!["json".to_string()],
            silent: true,
            gitignore: false,
            formats_names: crate::cli::FormatMappings::from_pairs(vec![(
                "makefile".to_string(),
                vec!["Recipe".to_string()],
            )]),
            ..Options::default()
        };

        let files = discover(&options).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].format, "makefile");
    }

    #[test]
    fn reporter_uses_report_paths_when_silent() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "jscpd-rs-reporter-paths-{}-{nonce}",
            std::process::id()
        ));
        let path = dir.join("file.js");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "const alpha = 1;\n").unwrap();

        let options = Options {
            paths: vec![path.clone()],
            min_lines: 1,
            reporters: vec!["html".to_string()],
            silent: true,
            gitignore: false,
            ..Options::default()
        };
        let cwd = std::env::current_dir().unwrap();

        let files = discover(&options).unwrap();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].source_id, display_relative_to(&path, &cwd));
    }
}
