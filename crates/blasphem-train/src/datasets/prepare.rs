use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use blasphem::{EvalLabel, Language, normalize_text};

use super::{
    DatasetSplit, ExclusionReason, ImportedRow, InclusionStatus, ProvenanceRow, RowDisposition,
    SourceSplit, SplitPolicy,
};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const UNKNOWN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRow {
    pub detector_language: Language,
    pub label: EvalLabel,
    pub source_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedCounts {
    pub development: usize,
    pub validation: usize,
    pub test: usize,
    pub duplicates: usize,
    pub conflicts: usize,
    pub excluded: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedLanguage {
    pub language: Language,
    pub development: Vec<PreparedRow>,
    pub validation: Vec<PreparedRow>,
    pub test: Vec<PreparedRow>,
    pub provenance: Vec<ProvenanceRow>,
    pub counts: PreparedCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationPolicy {
    pub language: Language,
    pub split_policy: SplitPolicy,
    pub split_version: &'static str,
    pub normalization_version: &'static str,
    pub audit_only_source_ids: BTreeSet<String>,
}

#[derive(Debug, Error)]
pub enum PreparationError {
    #[error("duplicate imported source identifier: {0}")]
    DuplicateSourceId(String),
    #[error("audit-only source identifier is unknown: {0}")]
    UnknownAuditOnlySourceId(String),
    #[error("audit-only source identifier is not a development row: {0}")]
    InvalidAuditOnlySourceId(String),
    #[error("candidate source {source_id} has detector language {actual:?}; expected {expected:?}")]
    DetectorLanguageMismatch {
        source_id: String,
        actual: Option<Language>,
        expected: Language,
    },
    #[error("source {source_id} has unsupported split {source_split}")]
    UnsupportedSourceSplit {
        source_id: String,
        source_split: SourceSplit,
    },
}

struct RowState {
    row: ImportedRow,
    label: Option<EvalLabel>,
    normalized: Option<String>,
    split: Option<DatasetSplit>,
    canonical_group_id: Option<String>,
    representative_source_id: Option<String>,
    exclusion_reason: Option<ExclusionReason>,
}

impl RowState {
    fn excluded(row: ImportedRow, exclusion_reason: ExclusionReason) -> Self {
        Self {
            label: match row.disposition {
                RowDisposition::Candidate(label) => Some(label),
                RowDisposition::Excluded(_) => None,
            },
            row,
            normalized: None,
            split: None,
            canonical_group_id: None,
            representative_source_id: None,
            exclusion_reason: Some(exclusion_reason),
        }
    }

    fn candidate(
        row: ImportedRow,
        label: EvalLabel,
        normalized: String,
        split: DatasetSplit,
    ) -> Self {
        Self {
            row,
            label: Some(label),
            normalized: Some(normalized),
            split: Some(split),
            canonical_group_id: None,
            representative_source_id: None,
            exclusion_reason: None,
        }
    }
}

#[must_use]
pub fn split_hash(language: Language, normalized: &str) -> u64 {
    language
        .storage_code()
        .bytes()
        .chain(std::iter::once(0))
        .chain(normalized.bytes())
        .fold(FNV_OFFSET, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        })
}

#[must_use]
pub fn split_for_key(language: Language, normalized: &str) -> DatasetSplit {
    match split_hash(language, normalized) % 100 {
        0..=69 => DatasetSplit::Development,
        70..=84 => DatasetSplit::Validation,
        _ => DatasetSplit::Test,
    }
}

pub fn prepare_language(
    rows: Vec<ImportedRow>,
    policy: &PreparationPolicy,
) -> Result<PreparedLanguage, PreparationError> {
    validate_source_ids(&rows)?;
    validate_candidate_languages(&rows, policy)?;
    validate_audit_only_source_ids(&rows, policy)?;

    let mut states = Vec::with_capacity(rows.len());
    let mut groups = BTreeMap::<String, Vec<usize>>::new();
    for row in rows {
        if policy.audit_only_source_ids.contains(&row.source_id) {
            states.push(RowState::excluded(row, ExclusionReason::AuditOnly));
            continue;
        }
        match row.disposition {
            RowDisposition::Excluded(reason) => states.push(RowState::excluded(row, reason)),
            RowDisposition::Candidate(label) => {
                let normalized = split_normalized_text(&row.text);
                if normalized.is_empty() {
                    states.push(RowState::excluded(row, ExclusionReason::EmptyText));
                    continue;
                }

                let split = split_for_source(&row, policy, &normalized)?;
                let group_key = format!("{}\0{normalized}", policy.language.code());
                let index = states.len();
                states.push(RowState::candidate(row, label, normalized, split));
                groups.entry(group_key).or_default().push(index);
            }
        }
    }

    for group in groups.values() {
        classify_group(group, &mut states, policy.language);
    }

    let mut prepared = PreparedLanguage {
        language: policy.language,
        development: Vec::new(),
        validation: Vec::new(),
        test: Vec::new(),
        provenance: Vec::with_capacity(states.len()),
        counts: PreparedCounts::default(),
    };
    for state in states {
        if let Some(reason) = state.exclusion_reason {
            prepared.counts.excluded += 1;
            if reason == ExclusionReason::Duplicate {
                prepared.counts.duplicates += 1;
            }
            if reason == ExclusionReason::LabelConflict {
                prepared.counts.conflicts += 1;
            }
        } else {
            let row = PreparedRow {
                detector_language: policy.language,
                label: state.label.expect("candidate rows have a label"),
                source_id: state.row.source_id.clone(),
                text: state.row.text.clone(),
            };
            match state.split.expect("included rows have a split") {
                DatasetSplit::Development => {
                    prepared.counts.development += 1;
                    prepared.development.push(row);
                }
                DatasetSplit::Validation => {
                    prepared.counts.validation += 1;
                    prepared.validation.push(row);
                }
                DatasetSplit::Test => {
                    prepared.counts.test += 1;
                    prepared.test.push(row);
                }
            }
        }
        prepared.provenance.push(provenance_row(state, policy));
    }

    prepared
        .development
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    prepared
        .validation
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    prepared
        .test
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    prepared
        .provenance
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    for row in &prepared.provenance {
        row.validate()
            .expect("preparation creates valid provenance");
    }
    Ok(prepared)
}

fn validate_source_ids(rows: &[ImportedRow]) -> Result<(), PreparationError> {
    let mut source_ids = BTreeSet::new();
    for row in rows {
        if !source_ids.insert(&row.source_id) {
            return Err(PreparationError::DuplicateSourceId(row.source_id.clone()));
        }
    }
    Ok(())
}

fn validate_audit_only_source_ids(
    rows: &[ImportedRow],
    policy: &PreparationPolicy,
) -> Result<(), PreparationError> {
    let rows_by_source_id = rows
        .iter()
        .map(|row| (row.source_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    for source_id in &policy.audit_only_source_ids {
        let Some(row) = rows_by_source_id.get(source_id.as_str()) else {
            return Err(PreparationError::UnknownAuditOnlySourceId(
                source_id.clone(),
            ));
        };
        let normalized = split_normalized_text(&row.text);
        let split = split_for_source(row, policy, &normalized)?;
        if split != DatasetSplit::Development {
            return Err(PreparationError::InvalidAuditOnlySourceId(
                source_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_candidate_languages(
    rows: &[ImportedRow],
    policy: &PreparationPolicy,
) -> Result<(), PreparationError> {
    for row in rows {
        if matches!(row.disposition, RowDisposition::Candidate(_))
            && row.detector_language != Some(policy.language)
        {
            return Err(PreparationError::DetectorLanguageMismatch {
                source_id: row.source_id.clone(),
                actual: row.detector_language,
                expected: policy.language,
            });
        }
    }
    Ok(())
}

fn split_normalized_text(text: &str) -> String {
    let normalized = normalize_text(text);
    if normalized.is_empty() {
        text.trim().to_owned()
    } else {
        normalized
    }
}

fn split_for_source(
    row: &ImportedRow,
    policy: &PreparationPolicy,
    normalized: &str,
) -> Result<DatasetSplit, PreparationError> {
    let unsupported = || PreparationError::UnsupportedSourceSplit {
        source_id: row.source_id.clone(),
        source_split: row.source_split,
    };
    match policy.split_policy {
        SplitPolicy::Hash70_15_15 if row.source_split == SourceSplit::Unsplit => {
            Ok(split_for_key(policy.language, normalized))
        }
        SplitPolicy::Hash70_15_15 => Err(unsupported()),
        SplitPolicy::TurkishOfficialTest if row.source_split == SourceSplit::Test => {
            Ok(DatasetSplit::Test)
        }
        SplitPolicy::TurkishOfficialTest if row.source_split == SourceSplit::Train => {
            match split_hash(policy.language, normalized) % 100 {
                0..=84 => Ok(DatasetSplit::Development),
                _ => Ok(DatasetSplit::Validation),
            }
        }
        SplitPolicy::TurkishOfficialTest => Err(unsupported()),
        SplitPolicy::PreserveOfficial => match row.source_split {
            SourceSplit::Train => Ok(DatasetSplit::Development),
            SourceSplit::Development | SourceSplit::Validation => Ok(DatasetSplit::Validation),
            SourceSplit::Test => Ok(DatasetSplit::Test),
            SourceSplit::Unsplit => Err(unsupported()),
        },
    }
}

fn classify_group(group: &[usize], states: &mut [RowState], language: Language) {
    let has_clean = group
        .iter()
        .any(|index| states[*index].label == Some(EvalLabel::Clean));
    let has_toxic = group
        .iter()
        .any(|index| states[*index].label == Some(EvalLabel::Toxic));
    let normalized = states[group[0]]
        .normalized
        .as_deref()
        .expect("grouped row has normalized text");
    let canonical_group_id = format!("{:016x}", split_hash(language, normalized));

    if has_clean && has_toxic {
        for index in group {
            let state = &mut states[*index];
            state.canonical_group_id = Some(canonical_group_id.clone());
            state.exclusion_reason = Some(ExclusionReason::LabelConflict);
        }
        return;
    }

    let representative = *group
        .iter()
        .min_by(|left, right| {
            split_priority(states[**right].split.expect("grouped row has a split"))
                .cmp(&split_priority(
                    states[**left].split.expect("grouped row has a split"),
                ))
                .then_with(|| {
                    states[**left]
                        .row
                        .source_id
                        .cmp(&states[**right].row.source_id)
                })
        })
        .expect("group has one row");
    let representative_source_id = states[representative].row.source_id.clone();
    let representative_split = states[representative]
        .split
        .expect("grouped row has a split");

    for index in group {
        let state = &mut states[*index];
        state.canonical_group_id = Some(canonical_group_id.clone());
        state.representative_source_id = Some(representative_source_id.clone());
        state.split = Some(representative_split);
        if *index != representative {
            state.exclusion_reason = Some(ExclusionReason::Duplicate);
        }
    }
}

const fn split_priority(split: DatasetSplit) -> u8 {
    match split {
        DatasetSplit::Development => 0,
        DatasetSplit::Validation => 1,
        DatasetSplit::Test => 2,
    }
}

fn provenance_row(state: RowState, policy: &PreparationPolicy) -> ProvenanceRow {
    let included = state.exclusion_reason.is_none();
    ProvenanceRow {
        dataset: state.row.dataset,
        source_file_id: state.row.source_file_id,
        source_id: state.row.source_id,
        immutable_source_url: String::new(),
        archive_member: None,
        revision: None,
        file_path: String::new(),
        file_sha256: UNKNOWN_SHA256
            .to_owned()
            .try_into()
            .expect("zero SHA-256 digest is valid"),
        acquired_at_unix_seconds: 0,
        license_id: String::new(),
        license_url: String::new(),
        citation: String::new(),
        upstream_lineage: Vec::new(),
        lineage_status: super::LineageStatus::Unresolved,
        source_language_code: state.row.source_language_code,
        detector_language_code: state.row.detector_language_code,
        source_label: state.row.source_label,
        detector_label: state.label,
        label_conversion_version: String::new(),
        split_version: policy.split_version.to_owned(),
        normalization_version: policy.normalization_version.to_owned(),
        canonical_group_id: state.canonical_group_id,
        representative_source_id: state.representative_source_id,
        source_split: state.row.source_split,
        detector_split: state.split,
        inclusion_status: if included {
            InclusionStatus::Included
        } else {
            InclusionStatus::Excluded
        },
        exclusion_reason: state.exclusion_reason,
    }
}
