use super::model::{CloneMatch, StatisticRow, Statistics};

#[derive(Clone, Debug, Default)]
pub struct Statistic {
    statistics: Statistics,
}

impl Statistic {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_statistic(&self) -> &Statistics {
        &self.statistics
    }

    pub fn into_statistics(self) -> Statistics {
        self.statistics
    }

    pub fn match_source(
        &mut self,
        source_id: impl AsRef<str>,
        format_name: impl AsRef<str>,
        lines: usize,
        tokens: usize,
    ) {
        update_source_statistics(
            &mut self.statistics,
            source_id.as_ref(),
            format_name.as_ref(),
            lines,
            tokens,
        );
        finalize_percentages(&mut self.statistics);
    }

    pub fn clone_found(&mut self, clone: &CloneMatch) {
        update_clone_statistics(&mut self.statistics, clone);
        finalize_percentages(&mut self.statistics);
    }
}

pub fn clone_lines(clone: &CloneMatch) -> usize {
    clone
        .duplication_a
        .end
        .line
        .saturating_sub(clone.duplication_a.start.line)
        + 1
}

pub(super) fn clone_stat_lines(clone: &CloneMatch) -> usize {
    clone
        .duplication_a
        .end
        .line
        .saturating_sub(clone.duplication_a.start.line)
}

fn clone_stat_tokens(clone: &CloneMatch) -> usize {
    clone
        .duplication_a
        .end
        .position
        .saturating_sub(clone.duplication_a.start.position)
}

pub(super) fn update_source_statistics(
    statistics: &mut Statistics,
    source_id: &str,
    format_name: &str,
    lines: usize,
    tokens: usize,
) {
    statistics.total.sources += 1;
    statistics.total.lines += lines;
    statistics.total.tokens += tokens;

    let format = statistics
        .formats
        .entry(format_name.to_string())
        .or_default();
    format.total.sources += 1;
    format.total.lines += lines;
    format.total.tokens += tokens;

    let source = format.sources.entry(source_id.to_string()).or_default();
    source.sources = 1;
    source.lines += lines;
    source.tokens += tokens;
}

pub(super) fn update_clone_statistics(statistics: &mut Statistics, clone: &CloneMatch) {
    let lines = clone_stat_lines(clone);
    let tokens = clone_stat_tokens(clone);
    statistics.total.clones += 1;
    statistics.total.duplicated_lines += lines;
    statistics.total.duplicated_tokens += tokens;

    let format = statistics.formats.entry(clone.format.clone()).or_default();
    format.total.clones += 1;
    format.total.duplicated_lines += lines;
    format.total.duplicated_tokens += tokens;

    for source_id in [
        &clone.duplication_a.source_id,
        &clone.duplication_b.source_id,
    ] {
        let source = format.sources.entry(source_id.clone()).or_default();
        source.clones += 1;
        source.duplicated_lines += lines;
        source.duplicated_tokens += tokens;
    }
}

pub(super) fn finalize_percentages(statistics: &mut Statistics) {
    update_row_percentages(&mut statistics.total);
    for format in statistics.formats.values_mut() {
        update_row_percentages(&mut format.total);
        for source in format.sources.values_mut() {
            update_row_percentages(source);
        }
    }
}

fn update_row_percentages(row: &mut StatisticRow) {
    row.percentage = percentage(row.lines, row.duplicated_lines);
    row.percentage_tokens = percentage(row.tokens, row.duplicated_tokens);
}

fn percentage(total: usize, duplicated: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        ((duplicated as f64 * 10000.0) / total as f64).round() / 100.0
    }
}
