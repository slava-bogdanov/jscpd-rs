use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};
use serde::Serialize;

use super::source::source_location;
use crate::cli::Options;
use crate::detector::DetectionResult;

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-sarif.json");
    let sarif = SarifReport::from_detection(result, options);
    let json = serde_json::to_string(&sarif)?;
    fs::write(&path, json).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("SARIF report saved to {}", path.display());
    Ok(())
}

#[derive(Serialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
    artifacts: Vec<SarifArtifact>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifDriver {
    name: &'static str,
    rules: Vec<SarifRule>,
    version: &'static str,
    information_uri: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRule {
    id: &'static str,
    short_description: SarifMessage,
    help_uri: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifResult {
    level: &'static str,
    message: SarifMessage,
    rule_id: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    locations: Vec<SarifLocation>,
    rule_index: usize,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifLocation {
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifPhysicalLocation {
    artifact_location: SarifArtifactLocation,
    region: SarifRegion,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifRegion {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SarifArtifact {
    source_language: String,
    location: SarifArtifactLocation,
}

impl SarifReport {
    fn from_detection(result: &DetectionResult, options: &Options) -> Self {
        const URL: &str = "https://github.com/kucherenko/jscpd/";

        let mut artifacts = Vec::new();
        let mut artifact_indices = HashMap::new();
        let mut results = Vec::new();

        for clone in &result.clones {
            let uri = clone.duplication_a.source_id.clone();
            let artifact_index = *artifact_indices.entry(uri.clone()).or_insert_with(|| {
                let index = artifacts.len();
                artifacts.push(SarifArtifact {
                    source_language: sarif_source_language(&clone.format),
                    location: SarifArtifactLocation {
                        uri: uri.clone(),
                        index: None,
                    },
                });
                index
            });

            results.push(SarifResult {
                level: "warning",
                message: SarifMessage {
                    text: format!(
                        "Clone detected in {}, - {}[{}] and {}[{}]",
                        clone.format,
                        clone.duplication_a.source_id,
                        source_location(&clone.duplication_a.start, &clone.duplication_a.end),
                        clone.duplication_b.source_id,
                        source_location(&clone.duplication_b.start, &clone.duplication_b.end),
                    ),
                },
                rule_id: "duplication",
                locations: vec![SarifLocation {
                    physical_location: SarifPhysicalLocation {
                        artifact_location: SarifArtifactLocation {
                            uri,
                            index: Some(artifact_index),
                        },
                        region: SarifRegion {
                            start_line: clone.duplication_a.start.line,
                            start_column: clone.duplication_a.start.column,
                            end_line: clone.duplication_a.end.line,
                            end_column: clone.duplication_a.end.column,
                        },
                    },
                }],
                rule_index: 0,
            });
        }

        if result.statistics.total.percentage >= options.threshold.unwrap_or(100.0) {
            results.push(SarifResult {
                level: "error",
                message: SarifMessage {
                    text: format!(
                        "The duplication level ({}%) is bigger than threshold ({}%)",
                        result.statistics.total.percentage,
                        options
                            .threshold
                            .map(|threshold| threshold.to_string())
                            .unwrap_or_else(|| "undefined".to_string()),
                    ),
                },
                rule_id: "duplications-threshold",
                locations: Vec::new(),
                rule_index: 1,
            });
        }

        Self {
            schema: "http://json.schemastore.org/sarif-2.1.0.json",
            version: "2.1.0",
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "jscpd",
                        rules: vec![
                            SarifRule {
                                id: "duplication",
                                short_description: SarifMessage {
                                    text: "Found code duplication".to_string(),
                                },
                                help_uri: URL,
                            },
                            SarifRule {
                                id: "duplications-threshold",
                                short_description: SarifMessage {
                                    text: "Level of duplication is too high".to_string(),
                                },
                                help_uri: URL,
                            },
                        ],
                        version: "4.2.4",
                        information_uri: URL,
                    },
                },
                results,
                artifacts,
            }],
        }
    }
}

fn sarif_source_language(format: &str) -> String {
    match format {
        "javascript" => "JavaScript".to_string(),
        "typescript" => "TypeScript".to_string(),
        "jsx" => "JSX".to_string(),
        "tsx" => "TSX".to_string(),
        "css" => "CSS".to_string(),
        "html" | "markup" => "HTML".to_string(),
        "json" => "JSON".to_string(),
        "markdown" => "Markdown".to_string(),
        "rust" => "Rust".to_string(),
        "python" => "Python".to_string(),
        "ruby" => "Ruby".to_string(),
        "go" => "Go".to_string(),
        "java" => "Java".to_string(),
        "csharp" => "C#".to_string(),
        "cpp" => "C++".to_string(),
        "c" => "C".to_string(),
        other => {
            let mut chars = other.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_uppercase(), chars.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;
    use crate::report::write_reports;

    #[test]
    fn sarif_report_matches_upstream_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let report = SarifReport::from_detection(&result, &Options::default());
        let json = serde_json::to_value(report).unwrap();

        assert_eq!(
            json["$schema"],
            "http://json.schemastore.org/sarif-2.1.0.json"
        );
        assert_eq!(json["version"], "2.1.0");
        assert_eq!(json["runs"][0]["tool"]["driver"]["name"], "jscpd");
        assert_eq!(
            json["runs"][0]["tool"]["driver"]["rules"][0]["id"],
            "duplication"
        );
        assert_eq!(
            json["runs"][0]["results"][0]["message"]["text"],
            "Clone detected in javascript, - src/a.js[2:3 - 5:1] and src/b.js[8:1 - 11:1]"
        );
        assert_eq!(
            json["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["index"],
            0
        );
        assert_eq!(
            json["runs"][0]["artifacts"][0]["sourceLanguage"],
            "JavaScript"
        );
    }

    #[test]
    fn sarif_report_includes_threshold_result_like_upstream() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.statistics.total.percentage = 25.0;
        let options = Options {
            threshold: Some(10.0),
            ..Options::default()
        };
        let report = SarifReport::from_detection(&result, &options);
        let json = serde_json::to_value(report).unwrap();

        assert_eq!(json["runs"][0]["results"][1]["level"], "error");
        assert_eq!(
            json["runs"][0]["results"][1]["message"]["text"],
            "The duplication level (25%) is bigger than threshold (10%)"
        );
        assert!(json["runs"][0]["results"][1]["locations"].is_null());
    }

    #[test]
    fn write_reports_writes_sarif_report() {
        let output = crate::report::test_support::temp_output("sarif-report");
        let options = Options {
            output: output.clone(),
            reporters: vec!["sarif".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let sarif = std::fs::read_to_string(output.join("jscpd-sarif.json")).unwrap();
        let _ = std::fs::remove_dir_all(output);
        let json: serde_json::Value = serde_json::from_str(&sarif).unwrap();

        assert_eq!(json["version"], "2.1.0");
        assert_eq!(json["runs"][0]["results"][0]["ruleId"], "duplication");
    }
}
