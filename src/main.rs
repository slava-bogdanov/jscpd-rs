mod cli;
mod detector;
mod files;
mod formats;
mod report;
mod tokenizer;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Options};
use crate::files::SourceFile;

fn main() {
    if let Err(error) = run() {
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

    let result = detector::detect(files, &options);

    report::write_reports(&result, &options)?;

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
}
