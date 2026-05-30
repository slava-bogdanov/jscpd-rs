use super::{
    Cli, FileConfig, Mode, Options, apply_config, normalize_reporters, parse_format_mappings,
    parse_size, resolve_config_ignore,
};
use clap::Parser;

#[test]
fn parses_size_suffixes() {
    assert_eq!(parse_size("1b").unwrap(), 1);
    assert_eq!(parse_size("100kb").unwrap(), 102400);
    assert_eq!(parse_size("2mb").unwrap(), 2 * 1024 * 1024);
}

#[test]
fn parses_format_mappings() {
    let mappings = parse_format_mappings("javascript:js,ts;python:py");
    assert_eq!(mappings.find_format_for_value("ts"), Some("javascript"));
    assert_eq!(mappings.find_format_for_value("py"), Some("python"));
    assert_eq!(mappings.find_format_for_value("rs"), None);
}

#[test]
fn normalizes_silent_reporter_like_upstream() {
    let mut options = Options {
        silent: true,
        reporters: vec!["console".to_string(), "json".to_string()],
        ..Options::default()
    };

    normalize_reporters(&mut options);

    assert_eq!(options.reporters, vec!["json", "silent"]);
}

#[test]
fn normalizes_threshold_reporter_like_upstream() {
    let mut options = Options {
        threshold: Some(10.0),
        reporters: vec!["json".to_string()],
        ..Options::default()
    };

    normalize_reporters(&mut options);
    normalize_reporters(&mut options);

    assert_eq!(options.reporters, vec!["json", "threshold"]);
}

#[test]
fn parses_upstream_workflow_options() {
    let cli = Cli::parse_from(&[
        "jscpd-rs",
        "--blame",
        "--store",
        "leveldb",
        "--store-path",
        ".jscpd-cache",
        "--noTips",
        ".",
    ]);
    let options = Options::from_cli(cli).unwrap();

    assert!(options.blame);
    assert_eq!(options.store.as_deref(), Some("leveldb"));
    assert_eq!(
        options.store_path.as_deref(),
        Some(std::path::Path::new(".jscpd-cache"))
    );
    assert!(options.no_tips);

    let config: FileConfig = serde_json::from_str(
        r#"{
            "executionId": "run-1",
            "store": "leveldb",
            "storePath": "cache",
            "blame": true,
            "cache": false,
            "noTips": true
        }"#,
    )
    .unwrap();
    let mut options = Options::default();
    apply_config(&mut options, config, std::path::Path::new(".")).unwrap();

    assert_eq!(options.execution_id.as_deref(), Some("run-1"));
    assert_eq!(options.store.as_deref(), Some("leveldb"));
    assert_eq!(
        options.store_path.as_deref(),
        Some(std::path::Path::new("cache"))
    );
    assert!(options.blame);
    assert!(!options.cache);
    assert!(options.no_tips);
}

#[test]
fn resolves_config_ignore_relative_to_config_dir() {
    let cwd = std::env::current_dir().unwrap();
    let config_dir = cwd.join("configs").join("nested");

    assert_eq!(
        resolve_config_ignore(&config_dir, "dist/**".to_string()).unwrap(),
        "configs/nested/dist/**"
    );
    assert_eq!(
        resolve_config_ignore(&config_dir, "**/generated/**".to_string()).unwrap(),
        "**/generated/**"
    );
}

#[test]
fn skip_comments_does_not_override_explicit_mode() {
    let cli = Cli::parse_from(&["jscpd-rs", "--skipComments", "."]);
    let options = Options::from_cli(cli).unwrap();
    assert_eq!(options.mode, Mode::Weak);

    let cli = Cli::parse_from(&["jscpd-rs", "--mode", "strict", "--skipComments", "."]);
    let options = Options::from_cli(cli).unwrap();
    assert_eq!(options.mode, Mode::Strict);

    let cli = Cli::parse_from(&["jscpd-rs", "--mode", "mild", "--skipComments", "."]);
    let options = Options::from_cli(cli).unwrap();
    assert_eq!(options.mode, Mode::Mild);
}
