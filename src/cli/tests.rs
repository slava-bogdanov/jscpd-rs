use super::{
    BARE_CONFIG_VALUE, Cli, ExitCode, FileConfig, Mode, Options, apply_config,
    apply_gitignore_patterns_from, normalize_reporters, parse_format_mappings, parse_js_number,
    parse_js_usize, parse_size, resolve_config_ignore, resolve_node_exit_code, store_warning,
};
use clap::{CommandFactory, Parser};

#[test]
fn parses_size_suffixes() {
    assert_eq!(parse_size("1b").unwrap(), 1);
    assert_eq!(parse_size("100kb").unwrap(), 102400);
    assert_eq!(parse_size("100KB").unwrap(), 102400);
    assert_eq!(parse_size("2mb").unwrap(), 2 * 1024 * 1024);
    assert_eq!(parse_size("1024").unwrap(), 1024);
    assert_eq!(parse_size("1.5kb").unwrap(), 1536);
    assert_eq!(parse_size("1.5 kb").unwrap(), 1536);
    assert_eq!(parse_size("1.1kb").unwrap(), 1126);
    assert_eq!(parse_size("+1kb").unwrap(), 1024);
    assert_eq!(parse_size("1tb").unwrap(), 1024_u64.pow(4));
    assert_eq!(
        parse_size("1.5tb").unwrap(),
        1024_u64.pow(4) + 1024_u64.pow(4) / 2
    );
    assert_eq!(parse_size("1pb").unwrap(), 1024_u64.pow(5));
    assert_eq!(parse_size("1k").unwrap(), 1);
    assert_eq!(parse_size("1m").unwrap(), 1);
    assert_eq!(parse_size("1 kb extra").unwrap(), 1);
    assert_eq!(parse_size(".5kb").unwrap(), 0);
    assert_eq!(parse_size("nope").unwrap(), 0);
    assert_eq!(parse_size("-1mb").unwrap(), 0);
}

#[test]
fn parses_cli_integer_flags_like_upstream_parse_int() {
    assert_eq!(parse_js_usize("1.5").unwrap(), 1);
    assert_eq!(parse_js_usize("20.9tokens").unwrap(), 20);
    assert_eq!(parse_js_usize("+1000.9").unwrap(), 1000);
    assert_eq!(parse_js_usize("0x10").unwrap(), 16);
    assert!(parse_js_usize(".5").is_err());
    assert!(parse_js_usize("nope").is_err());
    assert!(parse_js_usize("-1").is_err());
}

#[test]
fn parses_threshold_like_upstream_number() {
    assert_eq!(parse_js_number("7").unwrap(), 7.0);
    assert_eq!(parse_js_number("7.5").unwrap(), 7.5);
    assert_eq!(parse_js_number("0x10").unwrap(), 16.0);
    assert_eq!(parse_js_number("0b10").unwrap(), 2.0);
    assert_eq!(parse_js_number("").unwrap(), 0.0);
    assert!(parse_js_number("nope").unwrap().is_nan());
    assert!(parse_js_number("true").unwrap().is_nan());
}

#[test]
fn parses_exit_code_values_like_node_process_exit() {
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("7".to_string())).unwrap(),
        7
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("007".to_string())).unwrap(),
        7
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("0x10".to_string())).unwrap(),
        16
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("0b10".to_string())).unwrap(),
        2
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("-1".to_string())).unwrap(),
        -1
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::Boolean(false)).unwrap(),
        0
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("7.5".to_string())).unwrap_err(),
        "RangeError [ERR_OUT_OF_RANGE]: The value of \"code\" is out of range. It must be an integer. Received 7.5"
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::String("nope".to_string())).unwrap_err(),
        "TypeError [ERR_INVALID_ARG_TYPE]: The \"code\" argument must be of type number. Received type string ('nope')"
    );
    assert_eq!(
        resolve_node_exit_code(&ExitCode::Boolean(true)).unwrap_err(),
        "TypeError [ERR_INVALID_ARG_TYPE]: The \"code\" argument must be of type number. Received type boolean (true)"
    );
}

#[test]
fn accepts_missing_cli_integer_values_like_upstream() {
    let cli = Cli::parse_from([
        "jscpd-rs",
        ".",
        "--min-lines",
        "--min-tokens",
        "--max-lines",
    ]);
    let options = Options::from_cli(cli).unwrap();

    assert_eq!(options.min_lines, 0);
    assert_eq!(options.min_tokens, 50);
    assert_eq!(options.max_lines, usize::MAX);
}

#[test]
fn accepts_bare_optional_cli_values_that_upstream_continues_with() {
    let cli = Cli::parse_from([
        "jscpd-rs",
        ".",
        "--threshold",
        "--max-size",
        "--output",
        "--pattern",
        "--store",
        "--store-path",
        "--exitCode",
    ]);
    let options = Options::from_cli(cli).unwrap();

    assert_eq!(options.threshold, Some(1.0));
    assert_eq!(options.max_size_bytes, 0);
    assert_eq!(options.output, std::path::PathBuf::from("true"));
    assert!(options.output_is_bare);
    assert_eq!(options.pattern, "true");
    assert_eq!(options.store.as_deref(), Some("true"));
    assert_eq!(
        options.store_path.as_deref(),
        Some(std::path::Path::new("true"))
    );
    assert_eq!(options.exit_code, ExitCode::Boolean(true));
}

#[test]
fn accepts_bare_config_during_cli_parse_then_matches_upstream_runtime_error() {
    let cli = Cli::parse_from(["jscpd-rs", "--config"]);
    assert_eq!(
        cli.config.as_deref(),
        Some(std::path::Path::new(BARE_CONFIG_VALUE))
    );

    let error = Options::from_cli(cli).unwrap_err();
    assert_eq!(
        error.to_string(),
        "TypeError [ERR_INVALID_ARG_TYPE]: The \"paths[0]\" argument must be of type string. Received type boolean (true)"
    );
}

#[test]
fn bare_optional_string_flags_match_upstream_runtime_errors() {
    for (flag, expected) in [
        ("--ignore", "TypeError: cli.ignore.split is not a function"),
        (
            "--ignore-pattern",
            "TypeError: cli.ignorePattern.split is not a function",
        ),
        (
            "--reporters",
            "TypeError: cli.reporters.split is not a function",
        ),
        ("--mode", "TypeError: mode is not a function"),
        ("--format", "TypeError: cli.format.split is not a function"),
        (
            "--formats-exts",
            "TypeError: extensions.split is not a function",
        ),
        (
            "--formats-names",
            "TypeError: extensions.split is not a function",
        ),
    ] {
        let cli = Cli::parse_from(["jscpd-rs", ".", flag]);
        let error = Options::from_cli(cli).unwrap_err();
        assert_eq!(error.to_string(), expected, "{flag}");
    }
}

#[test]
fn malformed_cli_format_mappings_match_upstream_runtime_error() {
    for flag in ["--formats-exts", "--formats-names"] {
        let cli = Cli::parse_from(["jscpd-rs", ".", flag, "javascript"]);
        let error = Options::from_cli(cli).unwrap_err();

        assert_eq!(
            error.to_string(),
            "TypeError: Cannot read properties of undefined (reading 'split')",
            "{flag}"
        );
    }
}

#[test]
fn help_output_keeps_upstream_cli_contract_text() {
    let mut command = Cli::command();
    let mut output = Vec::new();
    command.write_long_help(&mut output).unwrap();
    let help = String::from_utf8(output).unwrap();

    assert!(help.contains("detector of copy/paste in files"));
    assert!(help.contains("Usage: jscpd [options] <path ...>"));
    assert!(help.contains("min size of duplication in code lines (Default is 5)"));
    assert!(help.contains("reporters or list of reporters separated with comma"));
    assert!(help.contains("ignore comments during detection (alias for --mode weak)"));
    assert!(help.contains("output the version number"));
    assert!(!help.contains("[possible values: strict, mild, weak]"));
}

#[test]
fn parses_version_flag_for_upstream_output_shape() {
    let cli = Cli::parse_from(["jscpd-rs", "--version"]);

    assert!(cli.version);
}

#[test]
fn parses_format_mappings() {
    let mappings = parse_format_mappings("javascript:js,ts;python:py");
    assert_eq!(mappings.find_format_for_value("ts"), Some("javascript"));
    assert_eq!(mappings.find_format_for_value("py"), Some("python"));
    assert_eq!(mappings.find_format_for_value("rs"), None);
}

#[test]
fn preserves_cli_format_order_for_debug_output_like_upstream() {
    let cli = Cli::parse_from(["jscpd-rs", "--format", "typescript,javascript", "."]);
    let options = Options::from_cli(cli).unwrap();

    assert_eq!(
        options.format_order.as_deref(),
        Some(["typescript".to_string(), "javascript".to_string()].as_slice())
    );
    assert!(options.formats.as_ref().unwrap().contains("typescript"));
    assert!(options.formats.as_ref().unwrap().contains("javascript"));
}

#[test]
fn appends_cwd_gitignore_patterns_to_debug_option_surface_like_upstream() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "jscpd-rs-cli-cwd-gitignore-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".gitignore"), "/target/\nreport\n").unwrap();

    let mut options = Options::default();
    apply_gitignore_patterns_from(&mut options, &dir);

    assert_eq!(
        options.ignore,
        vec![
            "target".to_string(),
            "target/**".to_string(),
            "**/report".to_string(),
            "**/report/**".to_string()
        ]
    );

    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn config_format_mappings_preserve_upstream_object_order() {
    let config: FileConfig = serde_json::from_str(
        r#"{
            "formatsExts": {
                "first": ["dup"],
                "second": ["dup"]
            },
            "formatsNames": {
                "name-first": ["Samefile"],
                "name-second": ["Samefile"]
            }
        }"#,
    )
    .unwrap();
    let mut options = Options::default();

    apply_config(&mut options, config, std::path::Path::new(".")).unwrap();

    assert_eq!(
        options.formats_exts.find_format_for_value("dup"),
        Some("first")
    );
    assert_eq!(
        options.formats_names.find_format_for_value("Samefile"),
        Some("name-first")
    );
}

#[test]
fn config_format_order_is_preserved_for_debug_output_like_upstream() {
    let config: FileConfig =
        serde_json::from_str(r#"{ "format": "typescript,javascript" }"#).unwrap();
    let mut options = Options::default();

    apply_config(&mut options, config, std::path::Path::new(".")).unwrap();

    assert_eq!(
        options.format_order.as_deref(),
        Some(["typescript".to_string(), "javascript".to_string()].as_slice())
    );
}

#[test]
fn config_accepts_string_numbers_that_upstream_coerces() {
    let config: FileConfig = serde_json::from_str(
        r#"{
            "minLines": "0x3",
            "maxLines": "1000",
            "threshold": "0x10"
        }"#,
    )
    .unwrap();
    let mut options = Options::default();

    apply_config(&mut options, config, std::path::Path::new(".")).unwrap();

    assert_eq!(options.min_lines, 3);
    assert_eq!(options.max_lines, 1000);
    assert_eq!(options.threshold, Some(16.0));
}

#[test]
fn default_execution_id_matches_upstream_shape() {
    let options = Options::default();
    let execution_id = options.execution_id.as_deref().unwrap();

    assert!(execution_id.ends_with('Z'));
    assert!(
        regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T")
            .unwrap()
            .is_match(execution_id)
    );
}

#[test]
fn default_path_matches_upstream_cwd() {
    let options = Options::default();

    assert_eq!(options.paths, vec![std::env::current_dir().unwrap()]);
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

    let mut duplicate = Options {
        silent: true,
        reporters: vec!["silent".to_string()],
        ..Options::default()
    };
    normalize_reporters(&mut duplicate);
    assert_eq!(duplicate.reporters, vec!["silent", "silent"]);
}

#[test]
fn normalizes_threshold_reporter_like_upstream() {
    let mut options = Options {
        threshold: Some(10.0),
        reporters: vec!["json".to_string()],
        ..Options::default()
    };

    normalize_reporters(&mut options);

    assert_eq!(options.reporters, vec!["json", "threshold"]);

    let mut duplicate = Options {
        threshold: Some(10.0),
        reporters: vec!["threshold".to_string()],
        ..Options::default()
    };
    normalize_reporters(&mut duplicate);
    assert_eq!(duplicate.reporters, vec!["threshold", "threshold"]);
}

#[test]
fn parses_upstream_workflow_options() {
    let cli = Cli::parse_from([
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
            "noTips": true,
            "listeners": ["console"],
            "tokensToSkip": ["comment", "block-comment"],
            "exitCode": "0x10",
            "reportersOptions": {
                "badge": {
                    "subject": "Duplication"
                }
            }
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
    assert_eq!(options.exit_code, ExitCode::String("0x10".to_string()));
    assert_eq!(options.listeners, vec!["console"]);
    assert_eq!(options.tokens_to_skip, vec!["comment", "block-comment"]);
    assert_eq!(
        options.reporters_options["badge"]["subject"].as_str(),
        Some("Duplication")
    );
}

#[test]
fn store_warning_matches_upstream_missing_store_fallback() {
    let options = Options {
        store: Some("leveldb".to_string()),
        ..Options::default()
    };

    assert_eq!(
        store_warning(&options).as_deref(),
        Some("store name leveldb not installed.")
    );

    assert!(store_warning(&Options::default()).is_none());
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
fn config_output_stays_cwd_relative_like_upstream() {
    let config: FileConfig = serde_json::from_str(r#"{ "output": "nested-report" }"#).unwrap();
    let mut options = Options::default();

    apply_config(
        &mut options,
        config,
        std::path::Path::new("/repo/configs/nested"),
    )
    .unwrap();

    assert_eq!(options.output, std::path::PathBuf::from("nested-report"));
}

#[test]
fn skip_comments_does_not_override_explicit_mode() {
    let cli = Cli::parse_from(["jscpd-rs", "--skipComments", "."]);
    let options = Options::from_cli(cli).unwrap();
    assert_eq!(options.mode, Mode::Weak);

    let cli = Cli::parse_from(["jscpd-rs", "--mode", "strict", "--skipComments", "."]);
    let options = Options::from_cli(cli).unwrap();
    assert_eq!(options.mode, Mode::Strict);

    let cli = Cli::parse_from(["jscpd-rs", "--mode", "mild", "--skipComments", "."]);
    let options = Options::from_cli(cli).unwrap();
    assert_eq!(options.mode, Mode::Mild);
}

#[test]
fn invalid_mode_reports_upstream_error_after_cli_parsing() {
    let cli = Cli::parse_from(["jscpd-rs", "--mode", "zzz", "."]);
    let error = Options::from_cli(cli).unwrap_err();

    assert_eq!(error.to_string(), "Mode zzz does not supported yet.");
}
