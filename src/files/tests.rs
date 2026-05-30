use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;

use crate::cli::Options;

use super::discover;
use super::gitignore::gitignore_line_to_globs;
use super::paths::{display_relative_to, fast_glob_like_path_cmp, relative_path};

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
    let dir = std::env::temp_dir().join(format!("jscpd-rs-formats-{}-{nonce}", std::process::id()));
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

#[test]
fn empty_file_counts_as_one_line_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-empty-lines-{}-{nonce}",
        std::process::id()
    ));
    let path = dir.join("empty.js");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&path, "").unwrap();

    let options = Options {
        paths: vec![path.clone()],
        min_lines: 1,
        max_lines: 1,
        reporters: Vec::new(),
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].source_id, path.display().to_string());
}
