use std::fs;

use anyhow::{Context, Result};

use super::source::clone_fragment;
use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult};

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-report.xml");
    let xml = XmlReport::from_detection(result).to_string();
    fs::write(&path, xml).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("XML report saved to {}", path.display());
    Ok(())
}

struct XmlReport {
    duplications: Vec<XmlDuplication>,
}

struct XmlDuplication {
    lines: usize,
    first_file: XmlFile,
    second_file: XmlFile,
    fragment: String,
}

struct XmlFile {
    path: String,
    line: usize,
    fragment: String,
}

impl XmlReport {
    fn from_detection(result: &DetectionResult) -> Self {
        Self {
            duplications: result
                .clones
                .iter()
                .map(|clone| XmlDuplication::from_clone(clone, result))
                .collect(),
        }
    }
}

impl XmlDuplication {
    fn from_clone(clone: &CloneMatch, result: &DetectionResult) -> Self {
        let first_fragment = clone_fragment(result, &clone.duplication_a);
        let second_fragment = clone_fragment(result, &clone.duplication_b);

        Self {
            lines: clone
                .duplication_a
                .end
                .line
                .saturating_sub(clone.duplication_a.start.line),
            first_file: XmlFile {
                path: escape_xml(&clone.duplication_a.source_id),
                line: clone.duplication_a.start.line,
                fragment: first_fragment.clone(),
            },
            second_file: XmlFile {
                path: escape_xml(&clone.duplication_b.source_id),
                line: clone.duplication_b.start.line,
                fragment: second_fragment,
            },
            fragment: first_fragment,
        }
    }
}

impl std::fmt::Display for XmlReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"<?xml version="1.0" encoding="UTF-8" ?><pmd-cpd>"#)?;
        for duplication in &self.duplications {
            write!(
                f,
                "\n      <duplication lines=\"{}\">\n            <file path=\"{}\" line=\"{}\">\n              <codefragment><![CDATA[{}]]></codefragment>\n            </file>\n            <file path=\"{}\" line=\"{}\">\n              <codefragment><![CDATA[{}]]></codefragment>\n            </file>\n            <codefragment><![CDATA[{}]]></codefragment>\n        </duplication>\n      ",
                duplication.lines,
                duplication.first_file.path,
                duplication.first_file.line,
                cdata_fragment(&duplication.first_file.fragment),
                duplication.second_file.path,
                duplication.second_file.line,
                cdata_fragment(&duplication.second_file.fragment),
                cdata_fragment(&duplication.fragment),
            )?;
        }
        write!(f, "</pmd-cpd>")
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\'', "&apos;")
        .replace('"', "&quot;")
}

fn cdata_fragment(value: &str) -> String {
    value.replacen("]]>", "CDATA_END", 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;
    use crate::report::write_reports;

    #[test]
    fn xml_report_matches_upstream_pmd_cpd_shape() {
        let result = make_test_result_with_clone("src/a<&>.js", "src/b.js");
        let xml = XmlReport::from_detection(&result).to_string();

        assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8" ?><pmd-cpd>"#));
        assert!(xml.ends_with("</pmd-cpd>"));
        assert!(xml.contains(r#"<duplication lines="3">"#));
        assert!(xml.contains(r#"<file path="src/a&lt;&amp;&gt;.js" line="2">"#));
        assert!(xml.contains("<![CDATA[alpha <beta> CDATA_END\n]]>"));
        assert!(xml.contains(r#"<file path="src/b.js" line="8">"#));
    }

    #[test]
    fn write_reports_writes_xml_report() {
        let output = crate::report::test_support::temp_output("xml-report");
        let options = Options {
            output: output.clone(),
            reporters: vec!["xml".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let xml = std::fs::read_to_string(output.join("jscpd-report.xml")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(xml.contains("<pmd-cpd>"));
        assert!(xml.contains(r#"<file path="src/a.js" line="2">"#));
    }
}
