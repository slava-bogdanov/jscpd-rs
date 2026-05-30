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
        for format in formats::supported_formats() {
            println!("{format}");
        }
        return Ok(());
    }

    let options = Options::from_cli(cli)?;
    let files = files::discover(&options)?;
    if options.debug {
        print_debug(&options, &files);
        return Ok(());
    }

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
}
