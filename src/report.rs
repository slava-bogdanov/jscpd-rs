use anyhow::Result;

use crate::cli::Options;
use crate::detector::DetectionResult;

mod ai;
mod badge;
mod console;
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

pub fn write_reports(result: &DetectionResult, options: &Options) -> Result<()> {
    if should_write_report("console", options) && !options.silent {
        console::write(result);
    }
    if should_write_report("consoleFull", options) {
        console_full::write(result, options);
    }
    if should_write_report("ai", options) {
        ai::write(result, options);
    }
    if should_write_report("json", options) {
        json::write(result, options)?;
    }
    if should_write_report("csv", options) {
        csv::write(result, options)?;
    }
    if should_write_report("badge", options) {
        badge::write(result, options)?;
    }
    if should_write_report("html", options) {
        html::write(result, options)?;
    }
    if should_write_report("markdown", options) {
        markdown::write(result, options)?;
    }
    if should_write_report("xml", options) {
        xml::write(result, options)?;
    }
    if should_write_report("sarif", options) {
        sarif::write(result, options)?;
    }
    if should_write_report("xcode", options) {
        xcode::write(result, options);
    }
    if should_write_report("silent", options) {
        silent::write(result);
    }
    if should_write_report("threshold", options) {
        threshold::write(result, options)?;
    }
    Ok(())
}

fn should_write_report(name: &str, options: &Options) -> bool {
    options.reporters.iter().any(|reporter| reporter == name)
}
