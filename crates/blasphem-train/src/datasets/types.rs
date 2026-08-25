use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use blasphem::{EvalLabel, Language};

use crate::evidence::Sha256Digest;
use crate::source_manifest::SourceRecord;

use super::PreparedCounts;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DatasetId {
    #[serde(rename = "hurtlex")]
    HurtLex,
    #[serde(rename = "textdetox")]
    TextDetox,
    #[serde(rename = "ibrohim-budi")]
    IbrohimBudi,
    #[serde(rename = "told-br")]
    ToldBr,
    #[serde(rename = "offenseval-tr")]
    OffensEvalTr,
    #[serde(rename = "vihos")]
    ViHos,
    #[serde(rename = "k-mhas")]
    KMHas,
    #[serde(rename = "germeval-2018")]
    GermEval2018,
    #[serde(rename = "community")]
    Community,
}

impl fmt::Display for DatasetId {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::HurtLex => "hurtlex",
            Self::TextDetox => "textdetox",
            Self::IbrohimBudi => "ibrohim-budi",
            Self::ToldBr => "told-br",
            Self::OffensEvalTr => "offenseval-tr",
            Self::ViHos => "vihos",
            Self::KMHas => "k-mhas",
            Self::GermEval2018 => "germeval-2018",
            Self::Community => "community",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSplit {
    Unsplit,
    Train,
    Development,
    Validation,
    Test,
}

impl fmt::Display for SourceSplit {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Unsplit => "unsplit",
            Self::Train => "train",
            Self::Development => "development",
            Self::Validation => "validation",
            Self::Test => "test",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit {
    Development,
    Validation,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPolicy {
    Hash70_15_15,
    TurkishOfficialTest,
    PreserveOfficial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageStatus {
    Resolved,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InclusionStatus {
    Included,
    Excluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    AmbiguousLabel,
    AuditOnly,
    Duplicate,
    EmptyText,
    LabelConflict,
    SealedBaselineDuplicate,
    UnsupportedLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowDisposition {
    Candidate(EvalLabel),
    Excluded(ExclusionReason),
}

pub struct SourceInput<'a> {
    pub source_file_id: &'a str,
    pub source_split: SourceSplit,
    pub reader: &'a mut dyn std::io::Read,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedRow {
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub source_id: String,
    pub source_language_code: String,
    pub detector_language: Option<Language>,
    pub detector_language_code: Option<String>,
    pub source_label: String,
    pub text: String,
    pub source_split: SourceSplit,
    pub disposition: RowDisposition,
}

pub trait DatasetAdapter {
    fn dataset_id(&self) -> DatasetId;
    fn label_conversion_version(&self) -> &'static str;
    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError>;
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("cannot parse CSV data: {0}")]
    Csv(#[from] csv::Error),
    #[error("cannot parse JSON data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot read source data: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing required column: {0}")]
    MissingColumn(&'static str),
    #[error("invalid binary label for source row: {0}")]
    InvalidBinaryLabel(String),
    #[error("missing joined label for source row: {0}")]
    MissingJoinedLabel(String),
    #[error("unused joined label for source row: {0}")]
    UnusedJoinedLabel(String),
    #[error("invalid harmful span for source row: {0}")]
    InvalidSpan(String),
    #[error("invalid Korean label set for source row: {0}")]
    InvalidKoreanLabel(String),
    #[error("invalid source row: {0}")]
    InvalidSource(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRow {
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub source_id: String,
    pub immutable_source_url: String,
    pub archive_member: Option<String>,
    pub revision: Option<String>,
    pub file_path: String,
    pub file_sha256: Sha256Digest,
    pub acquired_at_unix_seconds: u64,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
    pub source_language_code: String,
    pub detector_language_code: Option<String>,
    pub source_label: String,
    pub detector_label: Option<EvalLabel>,
    pub label_conversion_version: String,
    pub split_version: String,
    pub normalization_version: String,
    pub canonical_group_id: Option<String>,
    pub representative_source_id: Option<String>,
    pub source_split: SourceSplit,
    pub detector_split: Option<DatasetSplit>,
    pub inclusion_status: InclusionStatus,
    pub exclusion_reason: Option<ExclusionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedFileIdentity {
    pub relative_path: String,
    pub sha256: Sha256Digest,
    pub rows: usize,
    pub clean_rows: usize,
    pub toxic_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedManifest {
    pub schema_version: String,
    pub sources: Vec<SourceRecord>,
    pub language_sources: BTreeMap<String, Vec<String>>,
    pub language_counts: BTreeMap<String, PreparedCounts>,
    pub source_rows: usize,
    pub source_label_counts: BTreeMap<String, usize>,
    pub detector_label_counts: BTreeMap<String, usize>,
    pub source_split_counts: BTreeMap<String, usize>,
    pub detector_split_counts: BTreeMap<String, usize>,
    pub inclusion_status_counts: BTreeMap<String, usize>,
    pub exclusion_reason_counts: BTreeMap<String, usize>,
    pub prepared_files: BTreeMap<String, PreparedFileIdentity>,
}

impl ProvenanceRow {
    pub fn validate(&self) -> Result<(), ProvenanceValidationError> {
        match self.inclusion_status {
            InclusionStatus::Included => {
                if self.detector_label.is_none() {
                    return Err(ProvenanceValidationError::MissingDetectorLabel);
                }
                if self.detector_split.is_none() {
                    return Err(ProvenanceValidationError::MissingDetectorSplit);
                }
                if self.representative_source_id.is_none() {
                    return Err(ProvenanceValidationError::MissingRepresentativeSourceId);
                }
            }
            InclusionStatus::Excluded if self.exclusion_reason.is_none() => {
                return Err(ProvenanceValidationError::MissingExclusionReason);
            }
            InclusionStatus::Excluded => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ProvenanceValidationError {
    #[error("included provenance row has no detector label")]
    MissingDetectorLabel,
    #[error("included provenance row has no detector split")]
    MissingDetectorSplit,
    #[error("included provenance row has no representative source identifier")]
    MissingRepresentativeSourceId,
    #[error("excluded provenance row has no exclusion reason")]
    MissingExclusionReason,
}

#[must_use]
pub fn source_id(
    dataset: DatasetId,
    revision_or_hash: &str,
    split: SourceSplit,
    native_id: &str,
) -> String {
    format!("{dataset}@{revision_or_hash}/{split}/{native_id}")
}
