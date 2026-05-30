mod blame;
mod cli;
mod detector;
mod files;
mod formats;
mod report;
mod tokenizer;
mod verbose;

use std::time::{Duration, Instant};

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Options};
use crate::files::SourceFile;

fn main() {
    if let Err(error) = run() {
        if let Some(threshold) = error.downcast_ref::<report::ThresholdExceeded>() {
            eprintln!("{}", threshold.message());
            std::process::exit(1);
        }
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.list {
        print!("{}", list_output());
        return Ok(());
    }

    let options = Options::from_cli(cli)?;
    let files = files::discover(&options)?;
    if options.debug {
        print_debug(&options, &files);
        return Ok(());
    }

    print_store_warning(&options);

    let started = Instant::now();
    let mut result = detector::detect(files, &options);
    if options.blame {
        blame::apply_blame(&mut result);
    }

    if options.verbose {
        verbose::write_detection_events(&result);
    }
    report::write_progress(&result, &options);
    report::write_reports(&result, &options)?;
    print_terminal_footer(&options, started.elapsed());

    if !result.clones.is_empty() && options.exit_code != 0 {
        std::process::exit(options.exit_code);
    }

    Ok(())
}

fn print_debug(options: &Options, files: &[SourceFile]) {
    print!("{}", debug_output(options, files));
}

fn print_store_warning(options: &Options) {
    if let Some(warning) = store_warning(options) {
        eprintln!("{warning}");
    }
}

fn store_warning(options: &Options) -> Option<String> {
    options
        .store
        .as_ref()
        .map(|store| format!("store name {store} not installed."))
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
    output.push_str(&format!("{options:#?}\n"));
    for file in files {
        output.push_str(&file.source_id);
        output.push('\n');
    }
    output.push_str(&format!("Found {} files to detect.\n", files.len()));
    output
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
        assert!(output.contains("debug: true"));
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
            store_warning(&options).as_deref(),
            Some("store name leveldb not installed.")
        );
        assert!(store_warning(&Options::default()).is_none());
    }

    #[test]
    fn terminal_footer_matches_upstream_silent_and_tips_rules() {
        let elapsed = Duration::from_millis(42);
        let verbose = Options::default();
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
