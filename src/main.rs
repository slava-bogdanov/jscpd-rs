use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;

use jscpd_rs::cli::{Cli, ExitCode, Options};
use jscpd_rs::files::SourceFile;
use jscpd_rs::{cli, files, formats, report, verbose};

fn main() {
    if let Err(error) = run() {
        if let Some(threshold) = error.downcast_ref::<report::ThresholdExceeded>() {
            eprintln!("{}", threshold.message());
            std::process::exit(1);
        }
        let message = error.to_string();
        if let Some(stdout_error) = upstream_stdout_error(&message) {
            println!("{stdout_error}");
            std::process::exit(1);
        }
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn upstream_stdout_error(message: &str) -> Option<String> {
    if message.starts_with("TypeError [ERR_INVALID_ARG_TYPE]")
        || message.starts_with("TypeError:")
        || message.starts_with("SyntaxError:")
    {
        return Some(message.to_string());
    }
    if message.starts_with("Mode ") && message.ends_with(" does not supported yet.") {
        return Some(format!("Error: {message}"));
    }
    None
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let list = cli.list;

    let options = Options::from_cli(cli)?;
    if list {
        print!("{}", list_output());
        return Ok(());
    }

    let files = files::discover(&options)?;
    if options.debug {
        print_debug(&options, &files);
        return Ok(());
    }

    print_store_warning(&options);
    report::write_unknown_reporter_warnings(&options);

    let started = Instant::now();
    if files.is_empty() {
        print_terminal_footer(&options, started.elapsed());
        return Ok(());
    }

    let result = jscpd_rs::detect_source_files(files, &options);

    if options.verbose {
        verbose::write_detection_events(&result);
    }
    report::write_progress(&result, &options);
    report::write_reports(&result, &options)?;
    print_terminal_footer(&options, started.elapsed());

    if !result.clones.is_empty() {
        match cli::resolve_node_exit_code(&options.exit_code) {
            Ok(code) if code != 0 => std::process::exit(code),
            Ok(_) => {}
            Err(message) => {
                println!("{message}");
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn print_debug(options: &Options, files: &[SourceFile]) {
    print!("{}", debug_output(options, files));
}

fn print_store_warning(options: &Options) {
    if let Some(warning) = cli::store_warning(options) {
        eprintln!("{warning}");
    }
}

fn print_terminal_footer(options: &Options, elapsed: Duration) {
    if let Some(output) = terminal_footer_output(options, elapsed) {
        print!("{output}");
    }
}

fn terminal_footer_output(options: &Options, elapsed: Duration) -> Option<String> {
    if options.silent {
        return None;
    }

    let mut output = format!("time: {:.3}ms\n", elapsed.as_secs_f64() * 1000.0);
    if !options.no_tips {
        output.push('\n');
        for tip in TIPS {
            output.push_str(tip);
            output.push('\n');
        }
    }
    Some(output)
}

const TIPS: &[&str] = &[
    "💡 Auto-refactor with AI: npx skills add kucherenko/jscpd",
    "🎩 New: Gangsta Agents — discipline your AI coding → gangsta.page",
    "💖 Support jscpd project → https://opencollective.com/jscpd",
];

fn debug_output(options: &Options, files: &[SourceFile]) -> String {
    let mut output = String::new();
    output.push_str("Options:\n");
    output.push_str(&debug_options_output(options));
    output.push('\n');
    for file in files {
        output.push_str(&file.source_id);
        output.push('\n');
    }
    output.push_str(&format!("Found {} files to detect.\n", files.len()));
    output
}

fn debug_options_output(options: &Options) -> String {
    let mut fields = vec![
        debug_string_field("executionId", options.execution_id.as_deref().unwrap_or("")),
        debug_array_field(
            "path",
            &options
                .paths
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>(),
        ),
        format!("  mode: [Function: {}]", mode_name(options.mode)),
        format!("  minLines: {}", options.min_lines),
        format!("  maxLines: {}", options.max_lines),
        debug_string_field("maxSize", &debug_size(options.max_size_bytes)),
        format!("  minTokens: {}", options.min_tokens),
        debug_output_field(options),
        debug_array_field("reporters", &options.reporters),
        debug_array_field("ignore", &options.ignore),
        debug_optional_number_field("threshold", options.threshold),
        debug_format_mappings_field("formatsExts", &options.formats_exts),
        debug_format_mappings_field("formatsNames", &options.formats_names),
        format!("  debug: {}", options.debug),
        format!("  silent: {}", options.silent),
        format!("  blame: {}", options.blame),
        format!("  cache: {}", options.cache),
        format!("  absolute: {}", options.absolute),
        format!("  noSymlinks: {}", options.no_symlinks),
        format!("  skipLocal: {}", options.skip_local),
        format!("  ignoreCase: {}", options.ignore_case),
        format!("  gitignore: {}", options.gitignore),
        debug_reporter_options_field(options),
        debug_exit_code_field(&options.exit_code),
        format!("  noTips: {}", options.no_tips),
    ];
    if let Some(config) = &options.config {
        fields.push(debug_string_field("config", &config.display().to_string()));
    }
    fields.extend([
        debug_array_field("listeners", &options.listeners),
        debug_array_field("format", &debug_formats(options)),
    ]);

    if options.pattern != "**/*" {
        fields.push(debug_string_field("pattern", &options.pattern));
    }
    if let Some(store) = &options.store {
        fields.push(debug_string_field("store", store));
    }
    if let Some(store_path) = &options.store_path {
        fields.push(debug_string_field(
            "storePath",
            &store_path.display().to_string(),
        ));
    }
    if !options.tokens_to_skip.is_empty() {
        fields.push(debug_array_field("tokensToSkip", &options.tokens_to_skip));
    }

    format!("{{\n{}\n}}", fields.join(",\n"))
}

fn debug_string_field(name: &str, value: &str) -> String {
    format!("  {name}: '{}'", js_quote(value))
}

fn debug_output_field(options: &Options) -> String {
    if options.output_is_bare {
        "  output: true".to_string()
    } else {
        debug_string_field("output", &options.output.display().to_string())
    }
}

fn debug_array_field(name: &str, values: &[String]) -> String {
    if values.is_empty() {
        return format!("  {name}: []");
    }
    let values = values
        .iter()
        .map(|value| format!("'{}'", js_quote(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("  {name}: [ {values} ]")
}

fn debug_optional_number_field(name: &str, value: Option<f64>) -> String {
    match value {
        Some(value) => format!("  {name}: {value}"),
        None => format!("  {name}: undefined"),
    }
}

fn debug_exit_code_field(exit_code: &ExitCode) -> String {
    match exit_code {
        ExitCode::Number(value) => format!("  exitCode: {}", debug_js_number(*value)),
        ExitCode::String(value) => debug_string_field("exitCode", value),
        ExitCode::Boolean(value) => format!("  exitCode: {value}"),
    }
}

fn debug_js_number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn debug_format_mappings_field(name: &str, mappings: &cli::FormatMappings) -> String {
    if mappings.is_empty() {
        return format!("  {name}: {{}}");
    }
    let entries = mappings
        .iter()
        .map(|(format, values)| {
            let values = values
                .iter()
                .map(|value| format!("'{}'", js_quote(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}: [ {values} ]", js_quote(format))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("  {name}: {{ {entries} }}")
}

fn debug_reporter_options_field(options: &Options) -> String {
    if options.reporters_options.is_empty() {
        return "  reportersOptions: {}".to_string();
    }
    let json = serde_json::to_string(&options.reporters_options).unwrap_or_else(|_| "{}".into());
    format!("  reportersOptions: {json}")
}

fn debug_formats(options: &Options) -> Vec<String> {
    if let Some(formats) = &options.format_order {
        return formats.clone();
    }

    let supported = formats::supported_formats();
    match &options.formats {
        Some(selected) => supported
            .into_iter()
            .filter(|format| selected.contains(*format))
            .map(str::to_string)
            .collect(),
        None => supported.into_iter().map(str::to_string).collect(),
    }
}

fn debug_size(bytes: u64) -> String {
    if bytes.is_multiple_of(1024 * 1024) {
        format!("{}mb", bytes / (1024 * 1024))
    } else if bytes.is_multiple_of(1024) {
        format!("{}kb", bytes / 1024)
    } else {
        format!("{bytes}b")
    }
}

fn mode_name(mode: cli::Mode) -> &'static str {
    match mode {
        cli::Mode::Strict => "strict",
        cli::Mode::Mild => "mild",
        cli::Mode::Weak => "weak",
    }
}

fn js_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn list_output() -> String {
    format!(
        "Supported formats: \n{}\n",
        formats::supported_formats().join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_lists_options_and_files() {
        let options = Options {
            debug: true,
            config: Some(std::path::PathBuf::from("/repo/.jscpd.json")),
            formats: Some(std::collections::HashSet::from(["typescript".to_string()])),
            format_order: Some(vec!["typescript".to_string(), "javascript".to_string()]),
            ..Options::default()
        };
        let files = vec![
            SourceFile {
                source_id: "src/a.ts".to_string(),
                format: "typescript".to_string(),
                content: "const a = 1;".to_string(),
            },
            SourceFile {
                source_id: "src/b.ts".to_string(),
                format: "typescript".to_string(),
                content: "const b = 1;".to_string(),
            },
        ];

        let output = debug_output(&options, &files);

        assert!(output.starts_with("Options:\n"));
        assert!(!output.contains("Options {"));
        assert!(output.contains("executionId: '"));
        assert!(output.contains("path: [ '"));
        assert!(output.contains("debug: true"));
        assert!(output.contains("config: '/repo/.jscpd.json'"));
        assert!(output.contains("mode: [Function: mild]"));
        assert!(output.contains("maxSize: '100kb'"));
        assert!(output.contains("format: [ 'typescript', 'javascript' ]"));
        assert!(output.contains("src/a.ts\nsrc/b.ts"));
        assert!(output.ends_with("Found 2 files to detect.\n"));
        assert!(!output.contains("const a = 1"));
    }

    #[test]
    fn list_output_matches_upstream_shape() {
        let output = list_output();

        assert!(output.starts_with("Supported formats: \n"));
        assert!(output.contains("abap, actionscript, ada"));
        assert!(output.contains(", typescript, "));
        assert!(!output.lines().skip(1).any(|line| line == "typescript"));
    }

    #[test]
    fn store_warning_matches_upstream_fallback_shape() {
        let options = Options {
            store: Some("leveldb".to_string()),
            ..Options::default()
        };

        assert_eq!(
            cli::store_warning(&options).as_deref(),
            Some("store name leveldb not installed.")
        );
        assert!(cli::store_warning(&Options::default()).is_none());
    }

    #[test]
    fn node_like_errors_match_upstream_stdout_shape() {
        assert_eq!(
            upstream_stdout_error("Mode zzz does not supported yet.").as_deref(),
            Some("Error: Mode zzz does not supported yet.")
        );
        assert_eq!(
            upstream_stdout_error(
                "TypeError [ERR_INVALID_ARG_TYPE]: The \"paths[0]\" argument must be of type string."
            )
            .as_deref(),
            Some(
                "TypeError [ERR_INVALID_ARG_TYPE]: The \"paths[0]\" argument must be of type string."
            )
        );
        assert_eq!(
            upstream_stdout_error("TypeError: cli.ignore.split is not a function").as_deref(),
            Some("TypeError: cli.ignore.split is not a function")
        );
        assert!(upstream_stdout_error("regular anyhow failure").is_none());
    }

    #[test]
    fn terminal_footer_matches_upstream_silent_and_tips_rules() {
        let elapsed = Duration::from_millis(42);
        let verbose = Options {
            no_tips: false,
            ..Options::default()
        };
        let output = terminal_footer_output(&verbose, elapsed).unwrap();

        assert!(output.starts_with("time: "));
        assert!(output.contains("Auto-refactor with AI"));
        assert!(output.contains("Support jscpd project"));

        let no_tips = Options {
            no_tips: true,
            ..Options::default()
        };
        let output = terminal_footer_output(&no_tips, elapsed).unwrap();
        assert!(output.starts_with("time: "));
        assert!(!output.contains("Auto-refactor with AI"));

        let silent = Options {
            silent: true,
            ..Options::default()
        };
        assert!(terminal_footer_output(&silent, elapsed).is_none());
    }
}
