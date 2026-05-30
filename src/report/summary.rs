use crate::detector::{DetectionResult, StatisticRow};

pub(super) fn statistic_to_summary_row(format: &str, statistic: &StatisticRow) -> [String; 7] {
    [
        format.to_string(),
        statistic.sources.to_string(),
        statistic.lines.to_string(),
        statistic.tokens.to_string(),
        statistic.clones.to_string(),
        format!("{} ({}%)", statistic.duplicated_lines, statistic.percentage),
        format!(
            "{} ({}%)",
            statistic.duplicated_tokens, statistic.percentage_tokens
        ),
    ]
}

pub(super) fn silent_summary(result: &DetectionResult) -> String {
    format!(
        "Duplications detection: Found {} exact clones with {}({}%) duplicated lines in {} ({} formats) files.",
        result.clones.len(),
        result.statistics.total.duplicated_lines,
        result.statistics.total.percentage,
        result.statistics.total.sources,
        result.statistics.formats.len(),
    )
}
