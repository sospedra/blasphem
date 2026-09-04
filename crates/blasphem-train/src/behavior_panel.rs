use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use blasphem::{EvalLabel, Language};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const PANEL_HEADER: [&str; 9] = [
    "case_id",
    "language",
    "expected_nudge",
    "event_type",
    "pair_id",
    "control_kind",
    "evidence_kind",
    "evidence_ref",
    "text",
];
const REGISTRY_HEADER: [&str; 2] = ["evidence_ref", "language"];
const NO_PAIR: &str = "none";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorRow {
    pub case_id: String,
    pub language: Language,
    pub expected_nudge: bool,
    pub event_type: EventType,
    pub pair_id: String,
    pub control_kind: ControlKind,
    pub evidence_kind: EvidenceKind,
    pub evidence_ref: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Threat,
    HarmWish,
    SelfHarmCommand,
    DirectedInsult,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    None,
    Negation,
    Quotation,
    Reporting,
    Counterspeech,
    ViolenceQuestion,
    Replacement,
    Context,
    Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Dataset,
    NativeReview,
    Authored,
}

#[derive(Debug, Error)]
pub enum BehaviorPanelError {
    #[error("cannot read behavior data: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot parse behavior data: {0}")]
    Csv(#[from] csv::Error),
    #[error("behavior file has an invalid header: {0}")]
    InvalidHeader(PathBuf),
    #[error("behavior fixture root has no project root: {0}")]
    InvalidFixtureRoot(PathBuf),
    #[error("duplicate behavior case identifier: {0}")]
    DuplicateCaseId(String),
    #[error("duplicate evidence reference: {0}")]
    DuplicateEvidenceReference(String),
    #[error("duplicate toxic pair identifier: {0}")]
    DuplicateToxicPair(String),
    #[error("toxic pair has no clean link: {0}")]
    MissingCleanPair(String),
    #[error("toxic pair has more than one clean link: {0}")]
    DuplicateCleanPair(String),
    #[error("duplicate development source identifier: {0}")]
    DuplicateDevelopmentSourceId(String),
    #[error("behavior case {case_id} has language {actual:?}; expected {expected:?}")]
    WrongLanguage {
        case_id: String,
        expected: Language,
        actual: Language,
    },
    #[error("behavior case has an invalid event and control pair: {case_id}")]
    InvalidRowPair { case_id: String },
    #[error("behavior case {case_id} references unknown pair {pair_id}")]
    UnknownPair { case_id: String, pair_id: String },
    #[error("behavior case {case_id} has no registered evidence: {evidence_ref}")]
    MissingEvidence {
        case_id: String,
        evidence_ref: String,
    },
    #[error(
        "behavior case {case_id} has evidence language {actual:?}; expected {expected:?}: {evidence_ref}"
    )]
    EvidenceLanguageMismatch {
        case_id: String,
        evidence_ref: String,
        expected: Language,
        actual: Language,
    },
    #[error(
        "behavior case {case_id} lacks matching development evidence for {language:?}: {source_id}"
    )]
    MissingDevelopmentEvidence {
        case_id: String,
        language: Language,
        source_id: String,
    },
    #[error(
        "behavior case {case_id} has development evidence language {actual:?}; expected {expected:?}: {source_id}"
    )]
    DevelopmentEvidenceLanguageMismatch {
        case_id: String,
        source_id: String,
        expected: Language,
        actual: Language,
    },
    #[error(
        "behavior case {case_id} expects {expected:?} development evidence but found {actual:?}: {source_id}"
    )]
    DevelopmentEvidenceLabelMismatch {
        case_id: String,
        source_id: String,
        expected: EvalLabel,
        actual: EvalLabel,
    },
    #[error("behavior case {case_id} text differs from development evidence: {source_id}")]
    DevelopmentEvidenceTextMismatch { case_id: String, source_id: String },
    #[error("event distribution for {event_type:?} has {actual} toxic rows; expected {expected}")]
    InvalidEventDistribution {
        event_type: EventType,
        expected: usize,
        actual: usize,
    },
}

#[derive(Debug, Deserialize)]
struct RegistryRow {
    evidence_ref: String,
    language: Language,
}

#[derive(Debug, Deserialize)]
struct DevelopmentRow {
    source_id: String,
    #[serde(rename = "detector_language_code")]
    detector_language: Language,
    label: EvalLabel,
    inclusion_status: String,
    exclusion_reason: String,
    text: String,
}

struct DevelopmentEvidence {
    language: Language,
    label: EvalLabel,
    text: String,
}

/// Loads and validates one language behavior panel.
///
/// # Errors
///
/// Returns an error for malformed rows, invalid pair links, or unregistered evidence.
pub fn load_panel(root: &Path, language: Language) -> Result<Vec<BehaviorRow>, BehaviorPanelError> {
    let path = root.join(format!(
        "{}.tsv",
        language.storage_code().to_ascii_lowercase()
    ));
    let mut reader = tsv_reader(&path, &PANEL_HEADER)?;
    let mut rows = Vec::new();
    let mut case_ids = BTreeSet::new();
    for result in reader.deserialize::<BehaviorRow>() {
        let row = result?;
        if !case_ids.insert(row.case_id.clone()) {
            return Err(BehaviorPanelError::DuplicateCaseId(row.case_id));
        }
        if row.language != language {
            return Err(BehaviorPanelError::WrongLanguage {
                case_id: row.case_id,
                expected: language,
                actual: row.language,
            });
        }
        validate_row_pair(&row)?;
        rows.push(row);
    }
    validate_evidence(root, language, &rows)?;
    validate_pair_links(&rows)?;
    Ok(rows)
}

/// Checks the two-per-event toxic contract for one behavior panel.
///
/// # Errors
///
/// Returns an error when any toxic event type has a count other than two.
pub fn validate_event_distribution(rows: &[BehaviorRow]) -> Result<(), BehaviorPanelError> {
    for event_type in [
        EventType::Threat,
        EventType::HarmWish,
        EventType::SelfHarmCommand,
        EventType::DirectedInsult,
    ] {
        let actual = rows
            .iter()
            .filter(|row| row.expected_nudge && row.event_type == event_type)
            .count();
        if actual != 2 {
            return Err(BehaviorPanelError::InvalidEventDistribution {
                event_type,
                expected: 2,
                actual,
            });
        }
    }
    Ok(())
}

fn tsv_reader<const N: usize>(
    path: &Path,
    expected_header: &[&str; N],
) -> Result<csv::Reader<std::fs::File>, BehaviorPanelError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(false)
        .quoting(false)
        .from_path(path)?;
    if !reader.headers()?.iter().eq(expected_header.iter().copied()) {
        return Err(BehaviorPanelError::InvalidHeader(path.to_owned()));
    }
    Ok(reader)
}

fn validate_row_pair(row: &BehaviorRow) -> Result<(), BehaviorPanelError> {
    let valid = if row.expected_nudge {
        row.event_type != EventType::None
            && row.control_kind == ControlKind::None
            && row.pair_id != NO_PAIR
    } else {
        row.event_type == EventType::None && row.control_kind != ControlKind::None
    };
    if !valid {
        return Err(BehaviorPanelError::InvalidRowPair {
            case_id: row.case_id.clone(),
        });
    }
    Ok(())
}

fn validate_pair_links(rows: &[BehaviorRow]) -> Result<(), BehaviorPanelError> {
    let mut toxic_pairs = BTreeSet::new();
    for row in rows.iter().filter(|row| row.expected_nudge) {
        if !toxic_pairs.insert(row.pair_id.as_str()) {
            return Err(BehaviorPanelError::DuplicateToxicPair(row.pair_id.clone()));
        }
    }
    let mut clean_pair_counts = BTreeMap::<&str, usize>::new();
    for row in rows
        .iter()
        .filter(|row| !row.expected_nudge && row.pair_id != NO_PAIR)
    {
        if !toxic_pairs.contains(row.pair_id.as_str()) {
            return Err(BehaviorPanelError::UnknownPair {
                case_id: row.case_id.clone(),
                pair_id: row.pair_id.clone(),
            });
        }
        *clean_pair_counts.entry(row.pair_id.as_str()).or_default() += 1;
    }
    for pair_id in toxic_pairs {
        match clean_pair_counts.get(pair_id).copied().unwrap_or(0) {
            0 => return Err(BehaviorPanelError::MissingCleanPair(pair_id.to_owned())),
            1 => {}
            _ => return Err(BehaviorPanelError::DuplicateCleanPair(pair_id.to_owned())),
        }
    }
    Ok(())
}

fn validate_evidence(
    root: &Path,
    language: Language,
    rows: &[BehaviorRow],
) -> Result<(), BehaviorPanelError> {
    let authored = load_registry(&root.join("authored-v1.tsv"))?;
    let native_review = load_registry(&root.join("native-review-v1.tsv"))?;
    let development = load_development_evidence(root)?;

    for row in rows {
        if row.evidence_ref.trim().is_empty() {
            return Err(BehaviorPanelError::MissingEvidence {
                case_id: row.case_id.clone(),
                evidence_ref: row.evidence_ref.clone(),
            });
        }
        match row.evidence_kind {
            EvidenceKind::Authored => {
                validate_registered_evidence(row, language, &authored)?;
            }
            EvidenceKind::NativeReview => {
                validate_registered_evidence(row, language, &native_review)?;
            }
            EvidenceKind::Dataset => {
                let Some(evidence) = development.get(row.evidence_ref.as_str()) else {
                    return Err(BehaviorPanelError::MissingDevelopmentEvidence {
                        case_id: row.case_id.clone(),
                        language,
                        source_id: row.evidence_ref.clone(),
                    });
                };
                if evidence.language != language {
                    return Err(BehaviorPanelError::DevelopmentEvidenceLanguageMismatch {
                        case_id: row.case_id.clone(),
                        source_id: row.evidence_ref.clone(),
                        expected: language,
                        actual: evidence.language,
                    });
                }
                let expected_label = if row.expected_nudge {
                    EvalLabel::Toxic
                } else {
                    EvalLabel::Clean
                };
                if evidence.label != expected_label {
                    return Err(BehaviorPanelError::DevelopmentEvidenceLabelMismatch {
                        case_id: row.case_id.clone(),
                        source_id: row.evidence_ref.clone(),
                        expected: expected_label,
                        actual: evidence.label,
                    });
                }
                if evidence.text != row.text {
                    return Err(BehaviorPanelError::DevelopmentEvidenceTextMismatch {
                        case_id: row.case_id.clone(),
                        source_id: row.evidence_ref.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn load_registry(path: &Path) -> Result<BTreeMap<String, Language>, BehaviorPanelError> {
    let mut reader = tsv_reader(path, &REGISTRY_HEADER)?;
    let mut rows = BTreeMap::new();
    for result in reader.deserialize::<RegistryRow>() {
        let row = result?;
        if rows
            .insert(row.evidence_ref.clone(), row.language)
            .is_some()
        {
            return Err(BehaviorPanelError::DuplicateEvidenceReference(
                row.evidence_ref,
            ));
        }
    }
    Ok(rows)
}

fn validate_registered_evidence(
    row: &BehaviorRow,
    language: Language,
    registry: &BTreeMap<String, Language>,
) -> Result<(), BehaviorPanelError> {
    let Some(actual) = registry.get(&row.evidence_ref).copied() else {
        return Err(BehaviorPanelError::MissingEvidence {
            case_id: row.case_id.clone(),
            evidence_ref: row.evidence_ref.clone(),
        });
    };
    if actual != language {
        return Err(BehaviorPanelError::EvidenceLanguageMismatch {
            case_id: row.case_id.clone(),
            evidence_ref: row.evidence_ref.clone(),
            expected: language,
            actual,
        });
    }
    Ok(())
}

/// The audit-only rows the panels cite. They are excluded from the corpus, so
/// this file is their only copy.
pub const BEHAVIOR_PROVENANCE: &str = "crates/blasphem-train/metadata/behavior-audit-v1.tsv";

fn unescape_evidence(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('t') => output.push('\t'),
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('\\') => output.push('\\'),
            other => {
                output.push('\\');
                if let Some(value) = other {
                    output.push(value);
                }
            }
        }
    }
    output
}

fn load_development_evidence(
    root: &Path,
) -> Result<BTreeMap<String, DevelopmentEvidence>, BehaviorPanelError> {
    let project_root = root
        .ancestors()
        .nth(5)
        .ok_or_else(|| BehaviorPanelError::InvalidFixtureRoot(root.to_owned()))?;
    let path = project_root.join(BEHAVIOR_PROVENANCE);
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(false)
        .from_path(path)?;
    let mut source_ids = BTreeMap::new();
    for result in reader.deserialize::<DevelopmentRow>() {
        let row = result?;
        let _ = (&row.inclusion_status, &row.exclusion_reason);
        let source_id = row.source_id;
        if source_ids
            .insert(
                source_id.clone(),
                DevelopmentEvidence {
                    language: row.detector_language,
                    label: row.label,
                    text: unescape_evidence(&row.text),
                },
            )
            .is_some()
        {
            return Err(BehaviorPanelError::DuplicateDevelopmentSourceId(source_id));
        }
    }
    Ok(source_ids)
}
