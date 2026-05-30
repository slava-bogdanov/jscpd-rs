use std::cmp::Ordering;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) fn display_relative_to(path: &Path, cwd: &Path) -> String {
    relative_path(path, cwd)
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string()
}

pub(super) fn relative_path(path: &Path, base: &Path) -> Option<PathBuf> {
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

pub(super) fn fast_glob_like_path_cmp(left: &Path, right: &Path) -> Ordering {
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
