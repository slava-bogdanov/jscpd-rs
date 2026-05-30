use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;

use crate::cli::Options;

use super::discover;
use super::discovery::{count_lines, decode_source, format_filter_skip_message};
use super::gitignore::{
    collect_cwd_gitignore_patterns, collect_gitignore_patterns_with_global, gitignore_line_to_globs,
};
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
fn explicit_file_paths_preserve_cli_order_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-explicit-order-{}-{nonce}",
        std::process::id()
    ));
    let setup = dir.join("fixtures").join("setupTests.js");
    let utils = dir
        .join("packages")
        .join("react-devtools-shared")
        .join("utils.js");
    let console_mock = dir
        .join("packages")
        .join("internal-test-utils")
        .join("consoleMock.js");
    std::fs::create_dir_all(setup.parent().unwrap()).unwrap();
    std::fs::create_dir_all(utils.parent().unwrap()).unwrap();
    std::fs::create_dir_all(console_mock.parent().unwrap()).unwrap();
    std::fs::write(&setup, "const setup = 1;\n").unwrap();
    std::fs::write(&utils, "const utils = 1;\n").unwrap();
    std::fs::write(&console_mock, "const consoleMock = 1;\n").unwrap();

    let options = Options {
        paths: vec![setup.clone(), utils.clone(), console_mock.clone()],
        formats: Some(HashSet::from(["javascript".to_string()])),
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let paths = files
        .iter()
        .map(|file| file.source_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths.len(), 3);
    assert!(paths[0].ends_with("fixtures/setupTests.js"));
    assert!(paths[1].ends_with("packages/react-devtools-shared/utils.js"));
    assert!(paths[2].ends_with("packages/internal-test-utils/consoleMock.js"));
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

#[test]
fn gitignore_line_to_globs_preserves_negations_like_upstream() {
    let globs = gitignore_line_to_globs("!ignored/keep.js", Some(Path::new("/repo/app")));

    assert!(
        globs
            .iter()
            .any(|glob| glob == "!/repo/app/ignored/keep.js")
    );
    assert!(
        globs
            .iter()
            .any(|glob| glob == "!/repo/app/ignored/keep.js/**")
    );
}

#[test]
fn gitignore_line_to_globs_matches_upstream_conversion_without_base_dir() {
    assert_eq!(
        gitignore_line_to_globs("/node_modules", None),
        vec!["node_modules", "node_modules/**"]
    );
    assert_eq!(
        gitignore_line_to_globs("src/dist", None),
        vec!["src/dist", "src/dist/**", "**/src/dist", "**/src/dist/**"]
    );
    assert_eq!(
        gitignore_line_to_globs("**/dist", None),
        vec!["**/dist", "**/dist/**"]
    );
    assert_eq!(
        gitignore_line_to_globs("!test.js", None),
        vec!["!**/test.js", "!**/test.js/**"]
    );
    assert!(gitignore_line_to_globs("# ignored", None).is_empty());
    assert!(gitignore_line_to_globs("  ", None).is_empty());
}

#[test]
fn gitignore_line_to_globs_keeps_upstream_variants_for_cwd_base_dir() {
    let cwd = std::env::current_dir().unwrap();

    let globs = gitignore_line_to_globs("src/dist", Some(&cwd));
    assert!(globs.iter().any(|glob| glob == "src/dist"));
    assert!(globs.iter().any(|glob| glob == "src/dist/**"));
    assert!(globs.iter().any(|glob| glob == "**/src/dist"));
    assert!(globs.iter().any(|glob| glob == "**/src/dist/**"));

    let negated = gitignore_line_to_globs("!test.js", Some(&cwd));
    assert!(negated.iter().any(|glob| glob == "!**/test.js"));
    assert!(negated.iter().any(|glob| glob == "!**/test.js/**"));
}

#[test]
fn collect_gitignore_patterns_includes_global_excludes_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-global-excludes-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let global_excludes = dir.join("globalignore");
    std::fs::write(&global_excludes, "*.swp\n.DS_Store\n# comment\n\n").unwrap();

    let patterns =
        collect_gitignore_patterns_with_global(std::slice::from_ref(&dir), Some(&global_excludes));
    let _ = std::fs::remove_dir_all(&dir);

    assert!(patterns.iter().any(|pattern| pattern == "**/*.swp"));
    assert!(patterns.iter().any(|pattern| pattern == "**/*.swp/**"));
    assert!(patterns.iter().any(|pattern| pattern == "**/.DS_Store"));
    assert!(patterns.iter().all(|pattern| !pattern.contains("comment")));
}

#[test]
fn collect_cwd_gitignore_patterns_uses_upstream_unscoped_conversion() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-cwd-gitignore-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".gitignore"), "/target/\nreport\n# comment\n\n").unwrap();

    let patterns = collect_cwd_gitignore_patterns(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(patterns.iter().any(|pattern| pattern == "target"));
    assert!(patterns.iter().any(|pattern| pattern == "target/**"));
    assert!(patterns.iter().any(|pattern| pattern == "**/report"));
    assert!(patterns.iter().all(|pattern| !pattern.contains("comment")));
}

#[test]
fn format_filter_skip_message_matches_upstream_shape() {
    let cwd = Path::new("/repo");
    let path = Path::new("/repo/src/file.ts");

    assert_eq!(
        format_filter_skip_message(path, "typescript", cwd),
        "File src/file.ts skipped! Format \"typescript\" does not included to supported formats."
    );
}

#[test]
fn decode_source_reuses_valid_utf8_and_falls_back_to_lossy() {
    assert_eq!(
        decode_source(b"const answer = 42;\n".to_vec()),
        "const answer = 42;\n"
    );
    assert_eq!(decode_source(vec![b'a', 0xff, b'b']), "a\u{fffd}b");
}

#[test]
fn count_lines_matches_upstream_empty_and_newline_rules() {
    assert_eq!(count_lines(b""), 1);
    assert_eq!(count_lines(b"one"), 1);
    assert_eq!(count_lines(b"one\n"), 2);
    assert_eq!(count_lines(b"one\ntwo"), 2);
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
fn relative_ignore_pattern_matches_absolute_scan_root_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-relative-ignore-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(dir.join("patches")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("patches").join("patch.js"), "const patch = 1;\n").unwrap();
    std::fs::write(dir.join("src").join("main.js"), "const main = 1;\n").unwrap();

    let options = Options {
        paths: vec![dir.clone()],
        ignore: vec!["patches/**".to_string()],
        formats: Some(HashSet::from(["javascript".to_string()])),
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let paths = files
        .iter()
        .map(|file| file.source_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("src/main.js"));
}

#[test]
fn dot_relative_ignore_pattern_matches_absolute_scan_root_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-dot-relative-ignore-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(dir.join("patches")).unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("patches").join("patch.js"), "const patch = 1;\n").unwrap();
    std::fs::write(dir.join("src").join("main.js"), "const main = 1;\n").unwrap();

    let options = Options {
        paths: vec![dir.clone()],
        ignore: vec!["./patches/**".to_string()],
        formats: Some(HashSet::from(["javascript".to_string()])),
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let paths = files
        .iter()
        .map(|file| file.source_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("src/main.js"));
}

#[cfg(unix)]
#[test]
fn no_symlinks_skips_symlink_scan_directory_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-no-symlink-dir-{}-{nonce}",
        std::process::id()
    ));
    let real_dir = dir.join("real");
    let link_dir = dir.join("linkdir");
    std::fs::create_dir_all(&real_dir).unwrap();
    std::fs::write(real_dir.join("file.js"), "const linked = 1;\n").unwrap();
    std::os::unix::fs::symlink(&real_dir, &link_dir).unwrap();

    let options = Options {
        paths: vec![link_dir],
        formats: Some(HashSet::from(["javascript".to_string()])),
        no_symlinks: true,
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(files.is_empty());
}

#[cfg(unix)]
#[test]
fn no_symlinks_skips_symlink_scan_file_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-no-symlink-file-{}-{nonce}",
        std::process::id()
    ));
    let real_file = dir.join("real.js");
    let link_file = dir.join("link.js");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&real_file, "const linked = 1;\n").unwrap();
    std::os::unix::fs::symlink(&real_file, &link_file).unwrap();

    let options = Options {
        paths: vec![link_file],
        formats: Some(HashSet::from(["javascript".to_string()])),
        no_symlinks: true,
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(files.is_empty());
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

#[test]
fn known_extension_files_over_max_size_are_filtered() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-max-size-filter-{}-{nonce}",
        std::process::id()
    ));
    let path = dir.join("large.js");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&path, "const value = 'larger than the configured size';\n").unwrap();

    let options = Options {
        paths: vec![path],
        min_lines: 1,
        max_size_bytes: 10,
        reporters: Vec::new(),
        silent: true,
        gitignore: false,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(files.is_empty());
}

#[test]
fn gitignore_negation_reincludes_files_during_compat_discovery() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-gitignore-negation-{}-{nonce}",
        std::process::id()
    ));
    let ignored = dir.join("ignored");
    std::fs::create_dir_all(&ignored).unwrap();
    std::fs::write(dir.join(".gitignore"), "ignored/**\n!ignored/keep.js\n").unwrap();
    std::fs::write(ignored.join("drop.js"), "const drop = 1;\n").unwrap();
    std::fs::write(ignored.join("keep.js"), "const keep = 1;\n").unwrap();

    let options = Options {
        paths: vec![dir.clone()],
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: true,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let paths = files
        .iter()
        .map(|file| file.source_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("ignored/keep.js"));
}

#[test]
fn gitignore_broad_ignore_with_negated_filename_keeps_nested_file() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-gitignore-issue-723-{}-{nonce}",
        std::process::id()
    ));
    let nested = dir.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(dir.join(".gitignore"), "**/**/*\n!test.js\n").unwrap();
    std::fs::write(nested.join("drop.js"), "const drop = 1;\n").unwrap();
    std::fs::write(nested.join("test.js"), "const keep = 1;\n").unwrap();

    let options = Options {
        paths: vec![dir.clone()],
        min_lines: 1,
        reporters: vec!["json".to_string()],
        silent: true,
        gitignore: true,
        ..Options::default()
    };

    let files = discover(&options).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let paths = files
        .iter()
        .map(|file| file.source_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(paths.len(), 1);
    assert!(paths[0].ends_with("nested/test.js"));
}
