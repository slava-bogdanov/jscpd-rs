use anyhow::Result;

use crate::cli::Options;
use crate::detector::DetectionResult;

mod ai;
mod badge;
mod console;
mod console_common;
mod console_full;
mod csv;
mod html;
mod json;
mod markdown;
mod sarif;
mod silent;
mod source;
mod summary;
#[cfg(test)]
mod test_support;
mod threshold;
mod xcode;
mod xml;

pub use threshold::ThresholdExceeded;

pub fn write_reports(result: &DetectionResult, options: &Options) -> Result<()> {
    warn_unknown_reporters(options);

    for reporter in &options.reporters {
        match reporter.as_str() {
            "console" if !options.silent => console::write(result, options),
            "consoleFull" => console_full::write(result, options),
            "ai" => ai::write(result, options),
            "json" => json::write(result, options)?,
            "csv" => csv::write(result, options)?,
            "badge" => badge::write(result, options)?,
            "html" => html::write(result, options)?,
            "markdown" => markdown::write(result, options)?,
            "xml" => xml::write(result, options)?,
            "sarif" => sarif::write(result, options)?,
            "xcode" => xcode::write(result, options),
            "silent" => silent::write(result),
            "threshold" => threshold::write(result, options)?,
            _ => {}
        }
    }
    Ok(())
}

pub fn write_progress(result: &DetectionResult, options: &Options) {
    if !should_write_progress(options) {
        return;
    }
    print!("{}", progress_output(result, options));
}

fn should_write_progress(options: &Options) -> bool {
    !options.silent && !options.reporters.iter().any(|reporter| reporter == "ai")
}

fn progress_output(result: &DetectionResult, options: &Options) -> String {
    let mut output = String::new();
    for clone in &result.clones {
        output.push_str(&console_common::clone_header(clone, options));
        output.push('\n');
    }
    output
}

fn warn_unknown_reporters(options: &Options) {
    for warning in unknown_reporter_warnings(options) {
        println!("{warning}");
    }
}

fn is_builtin_reporter(reporter: &str) -> bool {
    matches!(
        reporter,
        "ai" | "xml"
            | "json"
            | "csv"
            | "markdown"
            | "consoleFull"
            | "html"
            | "console"
            | "silent"
            | "threshold"
            | "xcode"
            | "sarif"
            | "badge"
    )
}

fn unknown_reporter_warnings(options: &Options) -> Vec<String> {
    options
        .reporters
        .iter()
        .filter(|reporter| !is_builtin_reporter(reporter))
        .map(|reporter| {
            format!(
                "warning: {reporter} not installed (install packages named @jscpd/{reporter}-reporter or jscpd-{reporter}-reporter)"
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_builtin_reporters() {
        for reporter in [
            "ai",
            "xml",
            "json",
            "csv",
            "markdown",
            "consoleFull",
            "html",
            "console",
            "silent",
            "threshold",
            "xcode",
            "sarif",
            "badge",
        ] {
            assert!(is_builtin_reporter(reporter), "{reporter}");
        }
        assert!(!is_builtin_reporter("badgezz"));
    }

    #[test]
    fn warns_for_unknown_reporters_like_upstream() {
        let options = Options {
            reporters: vec![
                "json".to_string(),
                "badgezz".to_string(),
                "console".to_string(),
            ],
            silent: true,
            ..Options::default()
        };

        assert_eq!(
            unknown_reporter_warnings(&options),
            vec![
                "warning: badgezz not installed (install packages named @jscpd/badgezz-reporter or jscpd-badgezz-reporter)"
            ]
        );
    }

    #[test]
    fn progress_prints_clone_headers_when_not_silent_or_ai() {
        let result = test_support::make_test_result_with_clone("src/a.js", "src/b.js");
        let options = Options::default();

        let output = progress_output(&result, &options);

        assert!(output.contains("Clone found (javascript):"));
        assert!(output.contains("src/a.js"));
    }

    #[test]
    fn progress_is_suppressed_for_silent_and_ai_reports_like_upstream() {
        let silent = Options {
            silent: true,
            ..Options::default()
        };
        let ai = Options {
            reporters: vec!["ai".to_string()],
            ..Options::default()
        };

        assert!(!should_write_progress(&silent));
        assert!(!should_write_progress(&ai));
    }
}
