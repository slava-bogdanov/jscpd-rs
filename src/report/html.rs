use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::json;
use super::source::clone_fragment;
use crate::cli::Options;
use crate::detector::{CloneMatch, DetectionResult, StatisticRow};

mod assets;

const VERSION: &str = "4.2.4";

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    let destination = options.output.join("html");
    fs::create_dir_all(destination.join("styles")).with_context(|| {
        format!(
            "failed to create html styles dir `{}`",
            destination.join("styles").display()
        )
    })?;
    fs::create_dir_all(destination.join("js")).with_context(|| {
        format!(
            "failed to create html scripts dir `{}`",
            destination.join("js").display()
        )
    })?;

    let index = HtmlReport::from_detection(result).to_string();
    write_file(&destination.join("index.html"), index.as_bytes())?;
    write_file(
        &destination.join("jscpd-report.json"),
        json::to_pretty_json(result)?.as_bytes(),
    )?;
    write_file(
        &destination.join("styles").join("tailwind.css"),
        assets::TAILWIND_CSS.as_bytes(),
    )?;
    write_file(
        &destination.join("styles").join("prism.css"),
        assets::PRISM_CSS.as_bytes(),
    )?;
    write_file(
        &destination.join("js").join("prism.js"),
        assets::PRISM_JS.as_bytes(),
    )?;

    println!(
        "HTML report saved to {}",
        display_directory_with_slash(&destination)
    );
    Ok(())
}

fn write_file(path: &Path, content: &[u8]) -> Result<()> {
    fs::write(path, content).with_context(|| format!("failed to write `{}`", path.display()))
}

struct HtmlReport<'a> {
    result: &'a DetectionResult,
    formats: Vec<String>,
}

impl<'a> HtmlReport<'a> {
    fn from_detection(result: &'a DetectionResult) -> Self {
        let mut formats = result
            .statistics
            .formats
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        formats.extend(result.clones.iter().map(|clone| clone.format.clone()));
        Self {
            result,
            formats: formats.into_iter().collect(),
        }
    }
}

impl std::fmt::Display for HtmlReport<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total = &self.result.statistics.total;

        writeln!(f, "<!DOCTYPE html>")?;
        writeln!(f, r#"<html lang="en">"#)?;
        writeln!(f, "<head>")?;
        writeln!(f, r#"<meta charset="UTF-8">"#)?;
        writeln!(
            f,
            r#"<meta name="viewport" content="width=device-width, initial-scale=1.0">"#
        )?;
        writeln!(f, "<title>Copy/Paste Detector Report</title>")?;
        writeln!(f, r#"<link href="styles/tailwind.css" rel="stylesheet">"#)?;
        writeln!(f, r#"<link href="styles/prism.css" rel="stylesheet">"#)?;
        writeln!(f, "</head>")?;
        writeln!(f, "<body>")?;
        writeln!(f, "<header><div class=\"container\">")?;
        writeln!(f, "<h1>jscpd - copy/paste report</h1>")?;
        writeln!(f, "</div></header>")?;
        writeln!(f, "<main class=\"container\">")?;
        write_dashboard(f, total)?;
        write_formats(f, self)?;
        write_clones(f, self)?;
        writeln!(f, "</main>")?;
        write_footer(f)?;
        writeln!(f, r#"<script src="js/prism.js"></script>"#)?;
        write_toggle_script(f)?;
        writeln!(f, "</body>")?;
        writeln!(f, "</html>")
    }
}

fn write_dashboard(f: &mut std::fmt::Formatter<'_>, total: &StatisticRow) -> std::fmt::Result {
    writeln!(f, r#"<section id="dashboard">"#)?;
    writeln!(f, "<h2>Dashboard</h2>")?;
    writeln!(f, r#"<div class="dashboard-grid">"#)?;
    write_card(f, "blue", "Total Files", total.sources.to_string())?;
    write_card(f, "green", "Total Lines of Code", total.lines.to_string())?;
    write_card(f, "yellow", "Number of Clones", total.clones.to_string())?;
    write_card(
        f,
        "red",
        "Duplicated Lines",
        format!("{} ({:.2}%)", total.duplicated_lines, total.percentage),
    )?;
    writeln!(f, "</div>")?;
    writeln!(f, "</section>")
}

fn write_card(
    f: &mut std::fmt::Formatter<'_>,
    class_name: &str,
    title: &str,
    value: String,
) -> std::fmt::Result {
    writeln!(
        f,
        r#"<div class="card {class_name}"><h3>{}</h3><span>{}</span></div>"#,
        escape_html(title),
        escape_html(&value)
    )
}

fn write_formats(f: &mut std::fmt::Formatter<'_>, report: &HtmlReport<'_>) -> std::fmt::Result {
    writeln!(f, r#"<section id="formats">"#)?;
    writeln!(f, "<h2>Formats with Duplications</h2>")?;
    writeln!(f, "<table>")?;
    writeln!(
        f,
        "<thead><tr><th>Format</th><th>Files</th><th>Lines</th><th>Clones</th><th>Duplicated Lines</th><th>Duplicated Tokens</th></tr></thead>"
    )?;
    writeln!(f, "<tbody>")?;
    for format in &report.formats {
        let Some(statistic) = report.result.statistics.formats.get(format) else {
            continue;
        };
        let total = &statistic.total;
        writeln!(
            f,
            r##"<tr><td><a href="#{}-clones">{}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>"##,
            escape_html(format),
            escape_html(format),
            total.sources,
            total.lines,
            total.clones,
            total.duplicated_lines,
            total.duplicated_tokens,
        )?;
    }
    writeln!(f, "</tbody>")?;
    writeln!(f, "</table>")?;
    writeln!(f, "</section>")
}

fn write_clones(f: &mut std::fmt::Formatter<'_>, report: &HtmlReport<'_>) -> std::fmt::Result {
    writeln!(f, r#"<section id="txt-clones">"#)?;
    for format in &report.formats {
        writeln!(f, r#"<a name="{}-clones"></a>"#, escape_html(format))?;
        writeln!(f, "<h2>{}</h2>", escape_html(format))?;
        writeln!(f, r#"<div class="clones">"#)?;
        for (index, clone) in report
            .result
            .clones
            .iter()
            .enumerate()
            .filter(|(_, clone)| clone.format == *format)
        {
            write_clone(f, report.result, clone, index)?;
        }
        writeln!(f, "</div>")?;
    }
    writeln!(f, "</section>")
}

fn write_clone(
    f: &mut std::fmt::Formatter<'_>,
    result: &DetectionResult,
    clone: &CloneMatch,
    index: usize,
) -> std::fmt::Result {
    writeln!(f, r#"<div class="clone">"#)?;
    writeln!(
        f,
        "<p>{} (Line {}:{} - Line {}:{}), {} (Line {}:{} - Line {}:{})</p>",
        escape_html(&clone.duplication_a.source_id),
        clone.duplication_a.start.line,
        clone.duplication_a.start.column,
        clone.duplication_a.end.line,
        clone.duplication_a.end.column,
        escape_html(&clone.duplication_b.source_id),
        clone.duplication_b.start.line,
        clone.duplication_b.start.column,
        clone.duplication_b.end.line,
        clone.duplication_b.end.column,
    )?;
    writeln!(
        f,
        r#"<button id="expandBtn{index}" onclick="toggleCodeBlock('cloneGroup{index}', 'expandBtn{index}', 'collapseBtn{index}')">Show code</button>"#
    )?;
    writeln!(
        f,
        r#"<button class="hidden" id="collapseBtn{index}" onclick="toggleCodeBlock('cloneGroup{index}', 'expandBtn{index}', 'collapseBtn{index}')">Hide code</button>"#
    )?;
    writeln!(
        f,
        r#"<pre class="hidden" id="cloneGroup{index}"><code class="language-{}">{}</code></pre>"#,
        escape_html(&clone.format),
        escape_html(&clone_fragment(result, &clone.duplication_a))
    )?;
    writeln!(f, "</div>")
}

fn write_footer(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "<footer>")?;
    writeln!(
        f,
        r#"<p>Generated by <a href="https://jscpd.dev" target="_blank">jscpd</a> v{VERSION} by <a href="https://github.com/kucherenko" target="_blank">Andrey Kucherenko</a></p>"#
    )?;
    writeln!(
        f,
        r#"<p><a href="https://www.npmjs.com/package/jscpd" target="_blank">npm package</a> &middot; Since 2013 &middot; <a href="https://opencollective.com/jscpd" target="_blank">Sponsor jscpd</a></p>"#
    )?;
    writeln!(f, "</footer>")
}

fn write_toggle_script(f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "<script>")?;
    writeln!(
        f,
        "function toggleCodeBlock(codeBlockId, expandBtnId, collapseBtnId) {{"
    )?;
    writeln!(
        f,
        "  const codeBlock = document.getElementById(codeBlockId);"
    )?;
    writeln!(
        f,
        "  const expandBtn = document.getElementById(expandBtnId);"
    )?;
    writeln!(
        f,
        "  const collapseBtn = document.getElementById(collapseBtnId);"
    )?;
    writeln!(f, "  codeBlock.classList.toggle('hidden');")?;
    writeln!(f, "  expandBtn.classList.toggle('hidden');")?;
    writeln!(f, "  collapseBtn.classList.toggle('hidden');")?;
    writeln!(f, "}}")?;
    writeln!(f, "</script>")
}

fn display_directory_with_slash(path: &Path) -> String {
    let mut display = path.display().to_string();
    if !display.ends_with(std::path::MAIN_SEPARATOR) {
        display.push(std::path::MAIN_SEPARATOR);
    }
    display
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;
    use crate::report::write_reports;

    #[test]
    fn html_report_writes_upstream_layout_files() {
        let output = crate::report::test_support::temp_output("html-report");
        let options = Options {
            output: output.clone(),
            reporters: vec!["html".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let html = std::fs::read_to_string(output.join("html").join("index.html")).unwrap();
        let json = std::fs::read_to_string(output.join("html").join("jscpd-report.json")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(html.contains("<title>Copy/Paste Detector Report</title>"));
        assert!(html.contains("jscpd - copy/paste report"));
        assert!(html.contains("Formats with Duplications"));
        assert!(html.contains("Show code"));
        assert!(json.contains("\"duplicates\""));
        assert!(json.contains("\"statistics\""));
    }

    #[test]
    fn html_report_escapes_fragment_and_paths() {
        let result = make_test_result_with_clone("src/a<&>.js", "src/b.js");
        let html = HtmlReport::from_detection(&result).to_string();

        assert!(html.contains("src/a&lt;&amp;&gt;.js"));
        assert!(html.contains("alpha &lt;beta&gt; ]]&gt;"));
    }

    #[test]
    fn html_report_writes_static_assets() {
        let output = crate::report::test_support::temp_output("html-assets");
        let options = Options {
            output: output.clone(),
            reporters: vec!["html".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let tailwind = output.join("html").join("styles").join("tailwind.css");
        let prism_css = output.join("html").join("styles").join("prism.css");
        let prism_js = output.join("html").join("js").join("prism.js");
        let _ = std::fs::metadata(tailwind).unwrap();
        let _ = std::fs::metadata(prism_css).unwrap();
        let _ = std::fs::metadata(prism_js).unwrap();
        let _ = std::fs::remove_dir_all(output);
    }
}
