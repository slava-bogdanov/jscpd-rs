use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use serde::de::{MapAccess, Visitor};

use super::parsing::{compile_patterns, parse_format_mappings, parse_size, split_csv};
use super::{FormatMappings, Options};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FileConfig {
    execution_id: Option<String>,
    path: Option<OneOrMany>,
    pattern: Option<String>,
    ignore: Option<OneOrMany>,
    reporters: Option<OneOrMany>,
    listeners: Option<OneOrMany>,
    reporters_options: Option<serde_json::Map<String, serde_json::Value>>,
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
    mode: Option<super::Mode>,
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
    tokens_to_skip: Option<OneOrMany>,
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FormatMappingsConfig {
    String(String),
    Map(OrderedFormatMappings),
}

#[derive(Debug)]
struct OrderedFormatMappings(Vec<(String, Vec<String>)>);

impl<'de> Deserialize<'de> for OrderedFormatMappings {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(OrderedFormatMappingsVisitor)
    }
}

struct OrderedFormatMappingsVisitor;

impl<'de> Visitor<'de> for OrderedFormatMappingsVisitor {
    type Value = OrderedFormatMappings;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a format-to-values mapping object")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut items = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((format, values)) = map.next_entry::<String, Vec<String>>()? {
            items.push((format, values));
        }
        Ok(OrderedFormatMappings(items))
    }
}

impl FormatMappingsConfig {
    fn into_mappings(self) -> FormatMappings {
        match self {
            Self::String(value) => parse_format_mappings(&value),
            Self::Map(map) => FormatMappings(map.0),
        }
    }
}

pub(super) fn read_config(path: Option<&Path>) -> Result<Option<(FileConfig, PathBuf)>> {
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

pub(super) fn read_package_json_config() -> Result<Option<(FileConfig, PathBuf)>> {
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

pub(super) fn apply_config(
    options: &mut Options,
    config: FileConfig,
    config_dir: &Path,
) -> Result<()> {
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
    if let Some(listeners) = config.listeners {
        options.listeners = listeners.into_vec();
    }
    if let Some(reporters_options) = config.reporters_options {
        options.reporters_options = reporters_options;
    }
    if let Some(output) = config.output {
        options.output = output;
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
    if let Some(tokens_to_skip) = config.tokens_to_skip {
        options.tokens_to_skip = tokens_to_skip.into_vec();
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

pub(super) fn resolve_config_ignore(config_dir: &Path, pattern: String) -> Result<String> {
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
