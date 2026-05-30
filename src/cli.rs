use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::Deserialize;

mod config;
mod parsing;
#[cfg(test)]
mod tests;

#[cfg(test)]
use config::{FileConfig, resolve_config_ignore};
use config::{apply_config, read_config, read_package_json_config};
use parsing::{compile_patterns, parse_format_mappings, parse_size, split_csv};

#[derive(Debug, Parser)]
#[command(name = "jscpd-rs", version, about = "Fast Rust clone of jscpd")]
pub struct Cli {
    #[arg(value_name = "path")]
    pub paths: Vec<PathBuf>,

    #[arg(short = 'l', long = "min-lines")]
    pub min_lines: Option<usize>,

    #[arg(short = 'k', long = "min-tokens")]
    pub min_tokens: Option<usize>,

    #[arg(short = 'x', long = "max-lines")]
    pub max_lines: Option<usize>,

    #[arg(short = 'z', long = "max-size")]
    pub max_size: Option<String>,

    #[arg(short = 't', long = "threshold")]
    pub threshold: Option<f64>,

    #[arg(short = 'c', long = "config")]
    pub config: Option<PathBuf>,

    #[arg(short = 'i', long = "ignore")]
    pub ignore: Option<String>,

    #[arg(short = 'r', long = "reporters")]
    pub reporters: Option<String>,

    #[arg(short = 'o', long = "output")]
    pub output: Option<PathBuf>,

    #[arg(short = 'm', long = "mode")]
    pub mode: Option<Mode>,

    #[arg(short = 'f', long = "format")]
    pub format: Option<String>,

    #[arg(short = 'p', long = "pattern")]
    pub pattern: Option<String>,

    #[arg(short = 'b', long = "blame")]
    pub blame: bool,

    #[arg(short = 's', long = "silent")]
    pub silent: bool,

    #[arg(long = "store")]
    pub store: Option<String>,

    #[arg(long = "store-path")]
    pub store_path: Option<PathBuf>,

    #[arg(short = 'a', long = "absolute")]
    pub absolute: bool,

    #[arg(short = 'n', long = "noSymlinks")]
    pub no_symlinks: bool,

    #[arg(long = "ignoreCase")]
    pub ignore_case: bool,

    #[arg(short = 'g', long = "gitignore")]
    pub gitignore: bool,

    #[arg(long = "no-gitignore")]
    pub no_gitignore: bool,

    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    #[arg(long = "list")]
    pub list: bool,

    #[arg(long = "skipLocal")]
    pub skip_local: bool,

    #[arg(long = "exitCode")]
    pub exit_code: Option<i32>,

    #[arg(long = "noTips")]
    pub no_tips: bool,

    #[arg(long = "skipComments")]
    pub skip_comments: bool,

    #[arg(long = "ignore-pattern")]
    pub ignore_pattern: Option<String>,

    #[arg(long = "formats-exts")]
    pub formats_exts: Option<String>,

    #[arg(long = "formats-names")]
    pub formats_names: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Strict,
    Mild,
    Weak,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub execution_id: Option<String>,
    pub paths: Vec<PathBuf>,
    pub pattern: String,
    pub ignore: Vec<String>,
    pub reporters: Vec<String>,
    pub output: PathBuf,
    pub formats: Option<HashSet<String>>,
    pub formats_exts: FormatMappings,
    pub formats_names: FormatMappings,
    pub ignore_pattern: Vec<Regex>,
    pub min_lines: usize,
    pub min_tokens: usize,
    pub max_lines: usize,
    pub max_size_bytes: u64,
    pub threshold: Option<f64>,
    pub mode: Mode,
    pub store: Option<String>,
    pub store_path: Option<PathBuf>,
    pub blame: bool,
    pub cache: bool,
    pub silent: bool,
    pub absolute: bool,
    pub no_symlinks: bool,
    pub ignore_case: bool,
    pub gitignore: bool,
    pub debug: bool,
    pub verbose: bool,
    pub skip_local: bool,
    pub exit_code: i32,
    pub no_tips: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FormatMappings(Vec<(String, Vec<String>)>);

impl FormatMappings {
    #[cfg(test)]
    pub fn from_pairs(pairs: Vec<(String, Vec<String>)>) -> Self {
        Self(pairs)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn find_format_for_value(&self, value: &str) -> Option<&str> {
        self.0.iter().find_map(|(format, values)| {
            values
                .iter()
                .any(|item| item == value)
                .then_some(format.as_str())
        })
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            execution_id: None,
            paths: vec![PathBuf::from(".")],
            pattern: "**/*".to_string(),
            ignore: Vec::new(),
            reporters: vec!["console".to_string()],
            output: PathBuf::from("./report"),
            formats: None,
            formats_exts: FormatMappings::default(),
            formats_names: FormatMappings::default(),
            ignore_pattern: Vec::new(),
            min_lines: 5,
            min_tokens: 50,
            max_lines: 1000,
            max_size_bytes: 100 * 1024,
            threshold: None,
            mode: Mode::Mild,
            store: None,
            store_path: None,
            blame: false,
            cache: true,
            silent: false,
            absolute: false,
            no_symlinks: false,
            ignore_case: false,
            gitignore: true,
            debug: false,
            verbose: false,
            skip_local: false,
            exit_code: 0,
            no_tips: std::env::var_os("CI").is_some(),
        }
    }
}

impl Options {
    pub fn from_cli(cli: Cli) -> Result<Self> {
        let mut options = Self::default();

        if let Some((config, config_dir)) = read_package_json_config()? {
            apply_config(&mut options, config, &config_dir)?;
        }
        if let Some((config, config_dir)) = read_config(cli.config.as_deref())? {
            apply_config(&mut options, config, &config_dir)?;
        }

        if !cli.paths.is_empty() {
            options.paths = cli.paths;
        }
        if let Some(pattern) = cli.pattern {
            options.pattern = pattern;
        }
        if let Some(ignore) = cli.ignore {
            options.ignore = split_csv(&ignore);
        }
        if let Some(reporters) = cli.reporters {
            options.reporters = split_csv(&reporters);
        }
        if let Some(output) = cli.output {
            options.output = output;
        }
        if let Some(format) = cli.format {
            options.formats = Some(split_csv(&format).into_iter().collect());
        }
        if let Some(formats_exts) = cli.formats_exts {
            options.formats_exts = parse_format_mappings(&formats_exts);
        }
        if let Some(formats_names) = cli.formats_names {
            options.formats_names = parse_format_mappings(&formats_names);
        }
        if let Some(ignore_pattern) = cli.ignore_pattern {
            options.ignore_pattern = compile_patterns(split_csv(&ignore_pattern))
                .context("invalid --ignore-pattern value")?;
        }
        if let Some(min_lines) = cli.min_lines {
            options.min_lines = min_lines;
        }
        if let Some(min_tokens) = cli.min_tokens {
            options.min_tokens = min_tokens;
        }
        if let Some(max_lines) = cli.max_lines {
            options.max_lines = max_lines;
        }
        if let Some(max_size) = cli.max_size {
            options.max_size_bytes = parse_size(&max_size)
                .with_context(|| format!("invalid --max-size value `{max_size}`"))?;
        }
        if let Some(threshold) = cli.threshold {
            options.threshold = Some(threshold);
        }
        if let Some(mode) = cli.mode {
            options.mode = mode;
        }
        if cli.skip_comments && cli.mode.is_none() {
            options.mode = Mode::Weak;
        }
        if let Some(store) = cli.store {
            options.store = Some(store);
        }
        if let Some(store_path) = cli.store_path {
            options.store_path = Some(store_path);
        }
        if cli.blame {
            options.blame = true;
        }
        if cli.silent {
            options.silent = true;
        }
        if cli.absolute {
            options.absolute = true;
        }
        if cli.no_symlinks {
            options.no_symlinks = true;
        }
        if cli.ignore_case {
            options.ignore_case = true;
        }
        if cli.no_gitignore {
            options.gitignore = false;
        } else if cli.gitignore {
            options.gitignore = true;
        }
        if cli.debug {
            options.debug = true;
        }
        if cli.verbose {
            options.verbose = true;
        }
        if cli.skip_local {
            options.skip_local = true;
        }
        if let Some(exit_code) = cli.exit_code {
            options.exit_code = exit_code;
        }
        if cli.no_tips {
            options.no_tips = true;
        }

        normalize_reporters(&mut options);

        Ok(options)
    }
}

fn normalize_reporters(options: &mut Options) {
    if options.silent {
        options
            .reporters
            .retain(|reporter| !reporter.contains("console"));
        push_reporter_once(&mut options.reporters, "silent");
    }
    if options.threshold.is_some() {
        push_reporter_once(&mut options.reporters, "threshold");
    }
}

fn push_reporter_once(reporters: &mut Vec<String>, reporter: &str) {
    if !reporters.iter().any(|candidate| candidate == reporter) {
        reporters.push(reporter.to_string());
    }
}
