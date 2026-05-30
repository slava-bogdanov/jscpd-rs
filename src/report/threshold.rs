use std::error::Error;
use std::fmt;

use anyhow::Result;

use crate::cli::Options;
use crate::detector::DetectionResult;

pub(super) fn write(result: &DetectionResult, options: &Options) -> Result<()> {
    let Some(threshold) = options.threshold else {
        return Ok(());
    };
    if threshold < result.statistics.total.percentage {
        return Err(ThresholdExceeded::new(format!(
            "ERROR: jscpd found too many duplicates ({}%) over threshold ({}%)",
            result.statistics.total.percentage, threshold,
        ))
        .into());
    }
    Ok(())
}

#[derive(Debug)]
pub struct ThresholdExceeded {
    message: String,
}

impl ThresholdExceeded {
    fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ThresholdExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ThresholdExceeded {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::test_support::make_test_result_with_clone;
    use crate::report::write_reports;

    #[test]
    fn threshold_reporter_uses_strictly_greater_percentage_like_upstream() {
        let mut result = make_test_result_with_clone("src/a.js", "src/b.js");
        result.statistics.total.percentage = 25.0;

        let equal = Options {
            threshold: Some(25.0),
            reporters: vec!["threshold".to_string()],
            ..Options::default()
        };
        assert!(write_reports(&result, &equal).is_ok());

        let below = Options {
            threshold: Some(24.9),
            reporters: vec!["threshold".to_string()],
            ..Options::default()
        };
        let error = write_reports(&result, &below).unwrap_err();
        assert!(error.downcast_ref::<ThresholdExceeded>().is_some());
        assert_eq!(
            error.to_string(),
            "ERROR: jscpd found too many duplicates (25%) over threshold (24.9%)"
        );
    }
}
