use std::fs;

use anyhow::{Context, Result};

use crate::cli::Options;
use crate::detector::DetectionResult;

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    fs::create_dir_all(&options.output)
        .with_context(|| format!("failed to create output dir `{}`", options.output.display()))?;
    let path = options.output.join("jscpd-badge.svg");
    let badge = BadgeReport::from_detection(result, options).to_string();
    fs::write(&path, badge).with_context(|| format!("failed to write `{}`", path.display()))?;
    println!("Badge saved to {}", path.display());
    Ok(())
}

struct BadgeReport {
    subject: String,
    status: String,
    color: &'static str,
}

impl BadgeReport {
    fn from_detection(result: &DetectionResult, options: &Options) -> Self {
        Self {
            subject: "Copy/Paste".to_string(),
            status: format!("{}%", result.statistics.total.percentage),
            color: badge_color(result, options),
        }
    }
}

impl std::fmt::Display for BadgeReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let subject_width = text_width(&self.subject);
        let status_width = text_width(&self.status);
        let subject_rect_width = subject_width + 100;
        let status_rect_width = status_width + 100;
        let total_width = subject_rect_width + status_rect_width;
        let display_width = total_width as f64 / 10.0;
        let subject_text_x = 50;
        let status_text_x = subject_rect_width + 45;
        let subject_shadow_x = subject_text_x + 10;
        let status_shadow_x = status_text_x + 10;
        let subject = escape_xml(&self.subject);
        let status = escape_xml(&self.status);

        write!(
            f,
            "<svg width=\"{display_width:.1}\" height=\"20\" viewBox=\"0 0 {total_width} 200\" xmlns=\"http://www.w3.org/2000/svg\" role=\"img\" aria-label=\"{subject}: {status}\">\n  <title>{subject}: {status}</title>\n  <linearGradient id=\"g\" x2=\"0\" y2=\"100%\">\n    <stop offset=\"0\" stop-opacity=\".1\" stop-color=\"#EEE\"/>\n    <stop offset=\"1\" stop-opacity=\".1\"/>\n  </linearGradient>\n  <mask id=\"m\"><rect width=\"{total_width}\" height=\"200\" rx=\"30\" fill=\"#FFF\"/></mask>\n  <g mask=\"url(#m)\">\n    <rect width=\"{subject_rect_width}\" height=\"200\" fill=\"#555\"/>\n    <rect width=\"{status_rect_width}\" height=\"200\" fill=\"{}\" x=\"{subject_rect_width}\"/>\n    <rect width=\"{total_width}\" height=\"200\" fill=\"url(#g)\"/>\n  </g>\n  <g aria-hidden=\"true\" fill=\"#fff\" text-anchor=\"start\" font-family=\"Verdana,DejaVu Sans,sans-serif\" font-size=\"110\">\n    <text x=\"{subject_shadow_x}\" y=\"148\" textLength=\"{subject_width}\" fill=\"#000\" opacity=\"0.25\">{subject}</text>\n    <text x=\"{subject_text_x}\" y=\"138\" textLength=\"{subject_width}\">{subject}</text>\n    <text x=\"{status_shadow_x}\" y=\"148\" textLength=\"{status_width}\" fill=\"#000\" opacity=\"0.25\">{status}</text>\n    <text x=\"{status_text_x}\" y=\"138\" textLength=\"{status_width}\">{status}</text>\n  </g>\n  \n</svg>",
            self.color
        )
    }
}

fn badge_color(result: &DetectionResult, options: &Options) -> &'static str {
    match options.threshold {
        Some(threshold) if result.statistics.total.percentage < threshold => "#3C1",
        Some(_) => "#E43",
        None => "#999",
    }
}

fn text_width(value: &str) -> usize {
    value.chars().map(char_width).sum::<usize>() + 26
}

fn char_width(value: char) -> usize {
    match value {
        'A'..='Z' => 73,
        'a'..='z' => 63,
        '0'..='9' => 61,
        '/' => 38,
        '.' => 31,
        '%' => 88,
        ':' => 28,
        ' ' => 35,
        _ => 63,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;
    use crate::report::write_reports;

    #[test]
    fn badge_color_matches_upstream_threshold_rules() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.statistics.total.percentage = 25.0;

        assert_eq!(badge_color(&result, &Options::default()), "#999");
        assert_eq!(
            badge_color(
                &result,
                &Options {
                    threshold: Some(25.1),
                    ..Options::default()
                }
            ),
            "#3C1"
        );
        assert_eq!(
            badge_color(
                &result,
                &Options {
                    threshold: Some(25.0),
                    ..Options::default()
                }
            ),
            "#E43"
        );
    }

    #[test]
    fn badge_report_matches_upstream_default_shape() {
        let result = make_test_result_with_clone("src/a.js", "src/b.js");
        let badge = BadgeReport::from_detection(&result, &Options::default()).to_string();

        assert!(badge.starts_with("<svg "));
        assert!(badge.contains(r#"role="img" aria-label="Copy/Paste: 25%""#));
        assert!(badge.contains("<title>Copy/Paste: 25%</title>"));
        assert!(badge.contains(r##"fill="#999""##));
        assert!(badge.contains(">Copy/Paste</text>"));
        assert!(badge.contains(">25%</text>"));
    }

    #[test]
    fn write_reports_writes_badge_report() {
        let output = crate::report::test_support::temp_output("badge-report");
        let options = Options {
            output: output.clone(),
            reporters: vec!["badge".to_string()],
            silent: true,
            ..Options::default()
        };
        let result = make_test_result_with_clone("src/a.js", "src/b.js");

        write_reports(&result, &options).unwrap();
        let svg = std::fs::read_to_string(output.join("jscpd-badge.svg")).unwrap();
        let _ = std::fs::remove_dir_all(output);

        assert!(svg.contains("Copy/Paste"));
        assert!(svg.contains("25%"));
    }
}
