use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    let needs_compat_discovery =
        options.reporters.iter().any(|reporter| reporter == "json") || !options.silent;
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

fn shebang_format_for_path(path: &Path, metadata: &fs::Metadata) -> Result<Option<&'static str>> {
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

    let needs_report_paths =
        options.reporters.iter().any(|reporter| reporter == "json") || !options.silent;
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

fn collect_gitignore_patterns(roots: &[PathBuf]) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut visited_dirs = std::collections::HashSet::new();
    let mut visited_repos = std::collections::HashSet::new();

    for root in roots {
        let abs_root = root.canonicalize().unwrap_or_else(|_| root.clone());
        let mut current = if abs_root.is_file() {
            abs_root
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| abs_root.clone())
        } else {
            abs_root
        };
        let mut dirs = Vec::new();
        let mut repo_root = None;

        loop {
            if !visited_dirs.contains(&current) {
                dirs.push(current.clone());
            }
            if current.join(".git").exists() {
                repo_root = Some(current.clone());
                break;
            }
            let Some(parent) = current.parent() else {
                break;
            };
            if parent == current {
                break;
            }
            current = parent.to_path_buf();
        }

        for dir in dirs {
            if !visited_dirs.insert(dir.clone()) {
                continue;
            }
            let Ok(content) = fs::read_to_string(dir.join(".gitignore")) else {
                continue;
            };
            for line in content.lines() {
                patterns.extend(gitignore_line_to_globs(line, Some(&dir)));
            }
        }

        if let Some(repo_root) = repo_root
            && visited_repos.insert(repo_root.clone())
        {
            let exclude = repo_root.join(".git").join("info").join("exclude");
            if let Ok(content) = fs::read_to_string(exclude) {
                for line in content.lines() {
                    patterns.extend(gitignore_line_to_globs(line, Some(&repo_root)));
                }
            }
        }
    }

    patterns
}

fn gitignore_line_to_globs(line: &str, base_dir: Option<&Path>) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
        return Vec::new();
    }

    let is_rooted = trimmed.starts_with('/');
    let pattern = trimmed
        .trim_start_matches('/')
        .trim_end_matches('/')
        .replace('\\', "/");
    if pattern.is_empty() {
        return Vec::new();
    }

    let has_middle_slash = pattern.contains('/');
    if (is_rooted || has_middle_slash)
        && let Some(base_dir) = base_dir
    {
        let mut globs = Vec::new();
        push_gitignore_glob_variants(&mut globs, &base_dir.join(&pattern));
        return globs;
    }

    vec![format!("**/{pattern}"), format!("**/{pattern}/**")]
}

fn push_gitignore_glob_variants(globs: &mut Vec<String>, path: &Path) {
    let absolute = path.display().to_string().replace('\\', "/");
    globs.push(absolute.clone());
    globs.push(format!("{absolute}/**"));

    if let Ok(cwd) = std::env::current_dir()
        && let Some(relative) = relative_path(path, &cwd)
    {
        let relative = relative.display().to_string().replace('\\', "/");
        globs.push(relative.clone());
        globs.push(format!("{relative}/**"));
    }
}

fn display_relative_to(path: &Path, cwd: &Path) -> String {
    relative_path(path, cwd)
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
}

fn relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return Some(path.to_path_buf());
    }
    if !base.is_absolute() {
        return None;
    }

    let path_components = normal_components(path);
    let base_components = normal_components(base);
    let common_len = path_components
        .iter()
        .zip(&base_components)
        .take_while(|(left, right)| left == right)
        .count();

    let mut relative = PathBuf::new();
    for _ in common_len..base_components.len() {
        relative.push("..");
    }
    for component in &path_components[common_len..] {
        relative.push(component);
    }
    Some(relative)
}

fn normal_components(path: &Path) -> Vec<OsString> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_os_string()),
            _ => None,
        })
        .collect()
}

fn fast_glob_like_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left_components = left.components().collect::<Vec<_>>();
    let right_components = right.components().collect::<Vec<_>>();
    match left_components.len().cmp(&right_components.len()) {
        Ordering::Equal => {}
        ordering => return ordering,
    }

    for idx in 0..left_components.len() {
        let left_component = left_components[idx].as_os_str();
        let right_component = right_components[idx].as_os_str();
        if left_component == right_component {
            continue;
        }

        return left_component
            .to_string_lossy()
            .cmp(&right_component.to_string_lossy());
    }

    Ordering::Equal
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

    use super::{discover, fast_glob_like_path_cmp, gitignore_line_to_globs, relative_path};

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
}
