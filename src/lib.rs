pub mod blame;
pub mod cli;
pub mod detector;
pub mod files;
pub mod formats;
pub mod report;
pub mod server;
pub mod tokenizer;
pub mod verbose;

use anyhow::Result;

pub use detector::{CloneMatch, DetectionResult};

use crate::cli::Options;
use crate::files::SourceFile;

pub fn detect_clones(options: &Options) -> Result<Vec<CloneMatch>> {
    Ok(detect_clones_and_statistics(options)?.clones)
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
}
