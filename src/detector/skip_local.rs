use std::path::{Path, PathBuf};

use crate::cli::Options;

pub(super) fn same_configured_root(a: &str, b: &str, options: &Options) -> bool {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let a = normalize_for_prefix(Path::new(a), &cwd);
    let b = normalize_for_prefix(Path::new(b), &cwd);

    options.paths.iter().any(|root| {
        let root = normalize_for_prefix(root, &cwd);
        is_under_root(&a, &root) && is_under_root(&b, &root)
    })
}

fn is_under_root(path: &[PathBuf], root: &[PathBuf]) -> bool {
    path.len() > root.len() && path.starts_with(root)
}

fn normalize_for_prefix(path: &Path, cwd: &Path) -> Vec<PathBuf> {
    let full_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut normalized = Vec::new();

    for component in full_path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(value) => normalized.push(PathBuf::from(value)),
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {}
        }
    }

    normalized
}
