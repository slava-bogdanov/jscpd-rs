pub mod app;
pub mod blame;
pub mod cli;
pub mod detector;
pub mod files;
pub mod formats;
pub mod report;
pub mod server;
pub mod tokenizer;
pub mod verbose;

use std::{ffi::OsString, path::Path};

use anyhow::Result;

pub use app::{JscpdOutcome, jscpd, jscpd_with_exit_callback, run_cli_args};
pub use cli::{FormatMappings, Options};
pub use detector::{
    CloneMatch, DetectionResult, Detector, MemoryStore, MemoryStoreError, Statistic, StatisticRow,
    Statistics,
};
pub use files::SourceFile;
pub use tokenizer::{DetectionToken, Location, SourceTokenMap, TokenMap, Tokenizer};

pub fn get_default_options() -> Options {
    Options::default()
}

pub fn get_options_from_args<I, T>(args: I) -> Result<Options>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Options::from_args(args)
}

pub fn get_supported_formats() -> Vec<&'static str> {
    formats::supported_formats()
}

pub fn get_format_by_file(path: impl AsRef<Path>) -> Option<String> {
    get_format_by_file_with_mappings(path, &FormatMappings::default(), &FormatMappings::default())
}

pub fn get_format_by_file_with_mappings(
    path: impl AsRef<Path>,
    formats_exts: &FormatMappings,
    formats_names: &FormatMappings,
) -> Option<String> {
    formats::format_for_path(path.as_ref(), formats_exts, formats_names).map(str::to_string)
}

pub fn detect_clones(options: &Options) -> Result<Vec<CloneMatch>> {
    Ok(detect_clones_and_statistics(options)?.clones)
}

pub fn detect_clones_and_statistic(options: &Options) -> Result<DetectionResult> {
    detect_clones_and_statistics(options)
}

pub fn detect_clones_and_statistics(options: &Options) -> Result<DetectionResult> {
    let files = files::discover(options)?;
    Ok(detect_source_files(files, options))
}

pub fn detect_source_files(files: Vec<SourceFile>, options: &Options) -> DetectionResult {
    let mut result = detector::detect(files, options);
    if options.blame {
        blame::apply_blame(&mut result);
    }
    result
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn fixture_options(path: &str) -> Options {
        Options {
            paths: vec![PathBuf::from(path)],
            reporters: Vec::new(),
            silent: true,
            no_tips: true,
            min_tokens: 20,
            min_lines: 3,
            max_size_bytes: 1024 * 1024,
            ..Options::default()
        }
    }

    #[test]
    fn public_api_detects_clones_from_paths() {
        let options = fixture_options("jscpd/fixtures/clike/file2.c");

        let clones = detect_clones(&options).expect("detect clones");

        assert_eq!(clones.len(), 1);
        assert_eq!(clones[0].duplication_a.start.line, 18);
        assert_eq!(clones[0].duplication_b.start.line, 8);
    }

    #[test]
    fn public_api_returns_statistics() {
        let options = fixture_options("jscpd/fixtures/clike/file2.c");

        let result = detect_clones_and_statistics(&options).expect("detect with statistics");

        assert_eq!(result.clones.len(), 1);
        assert_eq!(result.statistics.total.clones, 1);
        assert_eq!(result.statistics.total.sources, 1);
    }

    #[test]
    fn public_api_statistic_alias_matches_upstream_name() {
        let options = fixture_options("jscpd/fixtures/clike/file2.c");

        let result = detect_clones_and_statistic(&options).expect("detect with statistic alias");

        assert_eq!(result.clones.len(), 1);
        assert_eq!(result.statistics.total.clones, 1);
    }

    #[test]
    fn public_api_exposes_default_options() {
        let options = get_default_options();

        assert_eq!(options.min_lines, 5);
        assert_eq!(options.min_tokens, 50);
        assert_eq!(options.max_lines, 1000);
        assert_eq!(options.max_size_bytes, 100 * 1024);
        assert_eq!(options.reporters, vec!["console"]);
        assert!(options.cache);
        assert!(options.gitignore);
    }

    #[test]
    fn public_api_parses_options_from_args() {
        let options = get_options_from_args([
            "jscpd",
            "fixtures",
            "--format",
            "javascript,typescript",
            "--reporters",
            "json",
            "--min-tokens",
            "7",
            "--min-lines",
            "2",
            "--max-size",
            "1mb",
            "--noTips",
        ])
        .expect("parse options from argv");

        let expected_formats = vec!["javascript".to_string(), "typescript".to_string()];
        assert_eq!(options.paths, vec![PathBuf::from("fixtures")]);
        assert_eq!(
            options.format_order.as_deref(),
            Some(expected_formats.as_slice())
        );
        assert_eq!(options.reporters, vec!["json"]);
        assert_eq!(options.min_tokens, 7);
        assert_eq!(options.min_lines, 2);
        assert_eq!(options.max_size_bytes, 1024 * 1024);
        assert!(options.no_tips);
    }

    #[test]
    fn public_api_arg_parser_preserves_runtime_option_errors() {
        let error = get_options_from_args(["jscpd", "--mode", "zzz", "."]).unwrap_err();

        assert_eq!(error.to_string(), "Mode zzz does not supported yet.");
    }

    #[test]
    fn public_api_exposes_supported_formats() {
        let formats = get_supported_formats();

        assert_eq!(formats.len(), 223);
        assert!(formats.contains(&"javascript"));
        assert!(formats.contains(&"typescript"));
        assert!(formats.contains(&"rust"));
    }

    #[test]
    fn public_api_resolves_format_by_file() {
        assert_eq!(
            get_format_by_file("src/index.mts").as_deref(),
            Some("typescript")
        );
        assert_eq!(
            get_format_by_file("src/component.vue").as_deref(),
            Some("vue")
        );
    }

    #[test]
    fn public_api_resolves_format_by_custom_mappings() {
        let formats_exts = FormatMappings::from_pairs(vec![("custom", vec!["foo"])]);
        let formats_names = FormatMappings::from_pairs(vec![("makefile", vec!["Buildfile"])]);

        assert_eq!(
            get_format_by_file_with_mappings("demo.foo", &formats_exts, &formats_names).as_deref(),
            Some("custom")
        );
        assert_eq!(
            get_format_by_file_with_mappings("Buildfile", &formats_exts, &formats_names).as_deref(),
            Some("makefile")
        );
        assert_eq!(
            get_format_by_file_with_mappings("src/index.ts", &formats_exts, &formats_names),
            None
        );
    }

    #[test]
    fn public_api_detects_from_in_memory_sources() {
        let options = Options {
            reporters: Vec::new(),
            silent: true,
            no_tips: true,
            min_tokens: 5,
            min_lines: 2,
            ..Options::default()
        };
        let content = "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n";
        let files = vec![
            SourceFile {
                source_id: "snippet.js".to_string(),
                format: "javascript".to_string(),
                content: content.to_string(),
            },
            SourceFile {
                source_id: "src/match.js".to_string(),
                format: "javascript".to_string(),
                content: content.to_string(),
            },
        ];

        let result = detect_source_files(files, &options);

        assert_eq!(result.clones.len(), 1);
        assert_eq!(result.statistics.total.sources, 2);
    }

    #[test]
    fn public_api_exposes_streaming_detector() {
        let options = Options {
            reporters: Vec::new(),
            silent: true,
            no_tips: true,
            min_tokens: 5,
            min_lines: 2,
            ..Options::default()
        };
        let content = "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n";
        let mut detector = Detector::new(options);

        assert!(
            detector
                .detect("first.js", content, "javascript")
                .is_empty()
        );
        let clones = detector.detect("second.js", content, "javascript");

        assert_eq!(clones.len(), 1);
        assert_eq!(detector.sources().len(), 2);
        assert!(
            clones[0].duplication_a.source_id == "second.js"
                || clones[0].duplication_b.source_id == "second.js"
        );
    }

    #[test]
    fn public_api_exposes_statistic_collector() {
        let options = Options {
            reporters: Vec::new(),
            silent: true,
            no_tips: true,
            min_tokens: 5,
            min_lines: 2,
            ..Options::default()
        };
        let content = "const alpha = 1;\nconst beta = 2;\nconst gamma = alpha + beta;\n";
        let result = detect_source_files(
            vec![
                SourceFile {
                    source_id: "first.js".to_string(),
                    format: "javascript".to_string(),
                    content: content.to_string(),
                },
                SourceFile {
                    source_id: "second.js".to_string(),
                    format: "javascript".to_string(),
                    content: content.to_string(),
                },
            ],
            &options,
        );
        let mut statistic = Statistic::new();

        statistic.match_source("first.js", "javascript", 3, 42);
        statistic.match_source("second.js", "javascript", 3, 42);
        statistic.clone_found(&result.clones[0]);

        let stats = statistic.get_statistic();
        assert_eq!(stats.total.sources, 2);
        assert_eq!(stats.total.clones, 1);
        assert_eq!(stats.formats["javascript"].sources["first.js"].sources, 1);
    }

    #[test]
    fn public_api_exposes_memory_store() {
        let mut store = MemoryStore::new();

        store.namespace("javascript");
        assert_eq!(*store.set("hash", 7usize), 7);
        assert_eq!(*store.get("hash").expect("stored value"), 7);
        store.namespace("typescript");
        let error = store.get("hash").unwrap_err();

        assert_eq!(error.namespace(), "typescript");
        assert_eq!(error.key(), "hash");
        assert_eq!(store.len(), 1);
        store.close();
        assert!(store.is_empty());
    }
}
