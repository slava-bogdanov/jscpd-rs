mod cli;
mod detector;
mod files;
mod formats;
mod report;
mod tokenizer;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Options};

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
    let result = detector::detect(files, &options);

    report::write_reports(&result, &options)?;

    if !result.clones.is_empty() && options.exit_code != 0 {
        std::process::exit(options.exit_code);
    }

    Ok(())
}
