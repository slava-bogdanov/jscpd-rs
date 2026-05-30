use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::Deserialize;

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

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileConfig {
    execution_id: Option<String>,
    path: Option<OneOrMany>,
    pattern: Option<String>,
    ignore: Option<OneOrMany>,
    reporters: Option<OneOrMany>,
    output: Option<PathBuf>,
    format: Option<OneOrMany>,
    formats_exts: Option<FormatMappingsConfig>,
    formats_names: Option<FormatMappingsConfig>,
    ignore_pattern: Option<OneOrMany>,
    min_lines: Option<usize>,
    min_tokens: Option<usize>,
    max_lines: Option<usize>,
    max_size: Option<String>,
    threshold: Option<f64>,
    mode: Option<Mode>,
    store: Option<String>,
    store_path: Option<PathBuf>,
    blame: Option<bool>,
    cache: Option<bool>,
    silent: Option<bool>,
    absolute: Option<bool>,
    no_symlinks: Option<bool>,
    ignore_case: Option<bool>,
    gitignore: Option<bool>,
    debug: Option<bool>,
    verbose: Option<bool>,
    skip_local: Option<bool>,
    exit_code: Option<i32>,
    no_tips: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::One(value) => split_csv(&value),
            Self::Many(values) => values,
        }
    }
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FormatMappingsConfig {
    String(String),
    Map(std::collections::HashMap<String, Vec<String>>),
}

impl FormatMappingsConfig {
    fn into_mappings(self) -> FormatMappings {
        match self {
            Self::String(value) => parse_format_mappings(&value),
            Self::Map(map) => {
                let mut items = map.into_iter().collect::<Vec<_>>();
                items.sort_by(|a, b| a.0.cmp(&b.0));
                FormatMappings(items)
            }
        }
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

fn read_config(path: Option<&Path>) -> Result<Option<(FileConfig, PathBuf)>> {
    let path = path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".jscpd.json"));
    if !path.exists() {
        return Ok(None);
    }

    let path = path
        .canonicalize()
        .with_context(|| format!("failed to resolve config path `{}`", path.display()))?;
    let data = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config `{}`", path.display()))?;
    let config = serde_json::from_str::<FileConfig>(&data)
        .with_context(|| format!("failed to parse config `{}`", path.display()))?;
    let config_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    Ok(Some((config, config_dir)))
}

fn read_package_json_config() -> Result<Option<(FileConfig, PathBuf)>> {
    let path = std::env::current_dir()?.join("package.json");
    if !path.exists() {
        return Ok(None);
    }

    let data = match fs::read_to_string(&path) {
        Ok(data) => data,
        Err(error) => {
            eprintln!("Warning: Could not read {}: {error}", path.display());
            return Ok(None);
        }
    };
    let value = match serde_json::from_str::<serde_json::Value>(&data) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Warning: Could not parse {}: {error}", path.display());
            return Ok(None);
        }
    };
    let Some(jscpd) = value.get("jscpd") else {
        return Ok(None);
    };
    let config = serde_json::from_value::<FileConfig>(jscpd.clone())
        .with_context(|| format!("failed to parse jscpd config in `{}`", path.display()))?;
    let config_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    Ok(Some((config, config_dir)))
}

fn apply_config(options: &mut Options, config: FileConfig, config_dir: &Path) -> Result<()> {
    if let Some(execution_id) = config.execution_id {
        options.execution_id = Some(execution_id);
    }
    if let Some(paths) = config.path {
        options.paths = paths
            .into_vec()
            .into_iter()
            .map(|path| resolve_config_path(config_dir, path))
            .collect();
    }
    if let Some(pattern) = config.pattern {
        options.pattern = pattern;
    }
    if let Some(ignore) = config.ignore {
        options.ignore = ignore
            .into_vec()
            .into_iter()
            .map(|pattern| resolve_config_ignore(config_dir, pattern))
            .collect::<Result<Vec<_>>>()?;
    }
    if let Some(reporters) = config.reporters {
        options.reporters = reporters.into_vec();
    }
    if let Some(output) = config.output {
        options.output = resolve_config_path(config_dir, output);
    }
    if let Some(format) = config.format {
        options.formats = Some(format.into_vec().into_iter().collect());
    }
    if let Some(formats_exts) = config.formats_exts {
        options.formats_exts = formats_exts.into_mappings();
    }
    if let Some(formats_names) = config.formats_names {
        options.formats_names = formats_names.into_mappings();
    }
    if let Some(ignore_pattern) = config.ignore_pattern {
        options.ignore_pattern = compile_patterns(ignore_pattern.into_vec())
            .context("invalid ignorePattern in config")?;
    }
    if let Some(min_lines) = config.min_lines {
        options.min_lines = min_lines;
    }
    if let Some(min_tokens) = config.min_tokens {
        options.min_tokens = min_tokens;
    }
    if let Some(max_lines) = config.max_lines {
        options.max_lines = max_lines;
    }
    if let Some(max_size) = config.max_size {
        options.max_size_bytes = parse_size(&max_size)
            .with_context(|| format!("invalid maxSize value `{max_size}` in config"))?;
    }
    if let Some(threshold) = config.threshold {
        options.threshold = Some(threshold);
    }
    if let Some(mode) = config.mode {
        options.mode = mode;
    }
    if let Some(store) = config.store {
        options.store = Some(store);
    }
    if let Some(store_path) = config.store_path {
        options.store_path = Some(store_path);
    }
    if let Some(blame) = config.blame {
        options.blame = blame;
    }
    if let Some(cache) = config.cache {
        options.cache = cache;
    }
    if let Some(silent) = config.silent {
        options.silent = silent;
    }
    if let Some(absolute) = config.absolute {
        options.absolute = absolute;
    }
    if let Some(no_symlinks) = config.no_symlinks {
        options.no_symlinks = no_symlinks;
    }
    if let Some(ignore_case) = config.ignore_case {
        options.ignore_case = ignore_case;
    }
    if let Some(gitignore) = config.gitignore {
        options.gitignore = gitignore;
    }
    if let Some(debug) = config.debug {
        options.debug = debug;
    }
    if let Some(verbose) = config.verbose {
        options.verbose = verbose;
    }
    if let Some(skip_local) = config.skip_local {
        options.skip_local = skip_local;
    }
    if let Some(exit_code) = config.exit_code {
        options.exit_code = exit_code;
    }
    if let Some(no_tips) = config.no_tips {
        options.no_tips = no_tips;
    }
    Ok(())
}

fn resolve_config_path<T: Into<PathBuf>>(config_dir: &Path, path: T) -> PathBuf {
    let path = path.into();
    if path.is_absolute() {
        path
    } else {
        config_dir.join(path)
    }
}

fn resolve_config_ignore(config_dir: &Path, pattern: String) -> Result<String> {
    let path = Path::new(&pattern);
    if path.is_absolute() || pattern.starts_with("**/") {
        return Ok(pattern);
    }

    let absolute = config_dir.join(&pattern);
    let cwd = std::env::current_dir().context("failed to resolve current directory")?;
    if let Ok(relative) = absolute.strip_prefix(cwd) {
        return Ok(relative.display().to_string());
    }

    Ok(absolute.display().to_string())
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_format_mappings(value: &str) -> FormatMappings {
    let mappings = value
        .split(';')
        .filter_map(|entry| {
            let (format, values) = entry.split_once(':')?;
            let values = split_csv(values);
            (!format.trim().is_empty() && !values.is_empty())
                .then(|| (format.trim().to_string(), values))
        })
        .collect();
    FormatMappings(mappings)
}

fn compile_patterns(patterns: Vec<String>) -> Result<Vec<Regex>> {
    patterns
        .into_iter()
        .map(|pattern| Regex::new(&pattern).with_context(|| format!("invalid regex `{pattern}`")))
        .collect()
}

fn parse_size(value: &str) -> Result<u64> {
    let trimmed = value.trim().to_ascii_lowercase();
    let split_at = trimmed
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(trimmed.len());
    let number = trimmed[..split_at]
        .parse::<u64>()
        .with_context(|| format!("missing numeric size in `{value}`"))?;
    let suffix = trimmed[split_at..].trim();
    let multiplier = match suffix {
        "" | "b" => 1,
        "k" | "kb" => 1024,
        "m" | "mb" => 1024 * 1024,
        "g" | "gb" => 1024 * 1024 * 1024,
        _ => anyhow::bail!("unsupported size suffix `{suffix}`"),
    };
    Ok(number * multiplier)
}

#[cfg(test)]
mod tests {
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
}
