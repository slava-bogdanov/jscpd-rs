use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::paths::relative_path;

pub(super) fn collect_gitignore_patterns(roots: &[PathBuf]) -> Vec<String> {
    let global_excludes_file = global_gitignore_path();
    collect_gitignore_patterns_with_global(roots, global_excludes_file.as_deref())
}

pub(crate) fn collect_cwd_gitignore_patterns(cwd: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(cwd.join(".gitignore")) else {
        return Vec::new();
    };
    content
        .lines()
        .flat_map(|line| gitignore_line_to_globs(line, None))
        .collect()
}

pub(super) fn collect_gitignore_patterns_with_global(
    roots: &[PathBuf],
    global_excludes_file: Option<&Path>,
) -> Vec<String> {
    let mut patterns = Vec::new();
    let mut visited_dirs = HashSet::new();
    let mut visited_repos = HashSet::new();

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

    if let Some(global_excludes_file) = global_excludes_file
        && let Ok(content) = fs::read_to_string(global_excludes_file)
    {
        for line in content.lines() {
            patterns.extend(gitignore_line_to_globs(line, None));
        }
    }

    patterns
}

fn global_gitignore_path() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["config", "--global", "core.excludesFile"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }
    if value == "~" {
        return home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return home_dir().map(|home| home.join(rest));
    }

    Some(PathBuf::from(value))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

pub(super) fn gitignore_line_to_globs(line: &str, base_dir: Option<&Path>) -> Vec<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Vec::new();
    }
    if let Some(pattern) = trimmed.strip_prefix('!') {
        return gitignore_line_to_globs(pattern, base_dir)
            .into_iter()
            .map(|glob| format!("!{glob}"))
            .collect();
    }

    let is_rooted = trimmed.starts_with('/');
    let pattern = trimmed
        .trim_start_matches('/')
        .trim_end_matches('/')
        .replace('\\', "/");
    if pattern.is_empty() {
        return Vec::new();
    }

    if let Some(base_dir) = base_dir {
        return scoped_gitignore_globs(base_dir, &pattern, is_rooted);
    }

    upstream_gitignore_globs(&pattern, is_rooted)
}

fn scoped_gitignore_globs(base_dir: &Path, pattern: &str, is_rooted: bool) -> Vec<String> {
    let mut globs = Vec::new();

    if is_rooted {
        push_gitignore_glob_variants(&mut globs, &base_dir.join(pattern));
        return globs;
    }

    if pattern.contains('/') {
        push_gitignore_glob_variants(&mut globs, &base_dir.join(pattern));
        if !pattern.starts_with("**/") {
            push_gitignore_glob_variants(&mut globs, &base_dir.join("**").join(pattern));
        }
        return globs;
    }

    push_gitignore_glob_variants(&mut globs, &base_dir.join("**").join(pattern));
    globs
}

fn upstream_gitignore_globs(pattern: &str, is_rooted: bool) -> Vec<String> {
    if is_rooted {
        return vec![pattern.to_string(), format!("{pattern}/**")];
    }

    if pattern.contains('/') {
        let mut globs = vec![pattern.to_string(), format!("{pattern}/**")];
        if !pattern.starts_with("**/") {
            globs.push(format!("**/{pattern}"));
            globs.push(format!("**/{pattern}/**"));
        }
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
