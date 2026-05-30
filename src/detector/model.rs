use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

use crate::tokenizer::Location;

pub type BlamedLines = BTreeMap<String, BlamedLine>;

#[derive(Clone, Debug, Serialize)]
pub struct BlamedLine {
    pub rev: String,
    pub author: String,
    pub date: String,
    pub line: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct SourceId(pub(super) usize);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct FormatId(pub(super) usize);

#[derive(Clone, Debug, Serialize)]
pub struct Fragment {
    #[serde(rename = "sourceId")]
    pub source_id: String,
    pub start: Location,
    pub end: Location,
    pub range: [usize; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blame: Option<BlamedLines>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CloneMatch {
    pub format: String,
    #[serde(rename = "duplicationA")]
    pub duplication_a: Fragment,
    #[serde(rename = "duplicationB")]
    pub duplication_b: Fragment,
    pub tokens: usize,
}

#[derive(Clone, Debug)]
pub struct SkippedClone {
    pub clone: CloneMatch,
    pub message: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StatisticRow {
    pub lines: usize,
    pub tokens: usize,
    pub sources: usize,
    pub clones: usize,
    #[serde(rename = "duplicatedLines")]
    pub duplicated_lines: usize,
    #[serde(rename = "duplicatedTokens")]
    pub duplicated_tokens: usize,
    pub percentage: f64,
    #[serde(rename = "percentageTokens")]
    pub percentage_tokens: f64,
    #[serde(rename = "newDuplicatedLines")]
    pub new_duplicated_lines: usize,
    #[serde(rename = "newClones")]
    pub new_clones: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FormatStatistic {
    pub sources: HashMap<String, StatisticRow>,
    pub total: StatisticRow,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Statistics {
    pub total: StatisticRow,
    pub formats: HashMap<String, FormatStatistic>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SourceSummary {
    pub path: String,
    pub format: String,
    pub lines: usize,
    pub tokens: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct DetectionResult {
    pub clones: Vec<CloneMatch>,
    #[serde(skip)]
    pub skipped_clones: Vec<SkippedClone>,
    pub statistics: Statistics,
    pub sources: Vec<SourceSummary>,
    #[serde(skip)]
    pub source_contents: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub(super) struct TokenSpan {
    pub(super) start: Location,
    pub(super) end: Location,
    pub(super) range: [usize; 2],
}

#[derive(Clone, Debug)]
pub(super) struct SourceMeta {
    pub(super) source_id: String,
    pub(super) format: String,
    pub(super) content: String,
    pub(super) lines: usize,
    pub(super) tokens: usize,
}

#[derive(Clone, Debug)]
pub(super) struct TokenStream {
    pub(super) source_id: SourceId,
    pub(super) format_id: FormatId,
    pub(super) hashes: Vec<u64>,
    pub(super) spans: Vec<TokenSpan>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Occurrence {
    pub(super) source_id: SourceId,
    pub(super) token_start: usize,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedSource {
    pub(super) meta: SourceMeta,
    pub(super) stream: TokenStream,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedSourceDraft {
    pub(super) meta: SourceMeta,
    pub(super) hashes: Vec<u64>,
    pub(super) spans: Vec<TokenSpan>,
}
