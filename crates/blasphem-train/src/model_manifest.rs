use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use blasphem::{
    ConfusionMatrix, FeatureProfile, FeatureSchema, Language, Metrics, NormalizationProfile,
    RuleChannelError, SparseModel, SparseModelError, arabic_hindi_rules, canonical_rule_identity,
    cjk_rules, word_rules,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    calibration::{GateResult, gates},
    compiler::{CompileError, CompiledLanguage},
    datasets::{DatasetId, PreparedCounts},
    evidence::Sha256Digest,
};

pub const MODEL_MANIFEST_SCHEMA_VERSION: u16 = 2;
const SPANISH_HURTLEX_SHA256: &str =
    "7ac642a30c91308b8fd2bfcf75c827238999b776aae502dddf8c3dbb20cde7cc";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelManifestEntry {
    pub language: Language,
    pub artifact_relative_path: String,
    pub dataset_inputs: Vec<DatasetInput>,
    pub feature_profile: FeatureProfile,
    pub normalization_profile: NormalizationProfile,
    pub feature_schema: FeatureSchema,
    pub rule_pack_version: u16,
    pub rule_pack_sha256: Sha256Digest,
    pub hurtlex_sha256: Option<Sha256Digest>,
    pub clean_control_rows: usize,
    pub clean_control_sha256: Option<Sha256Digest>,
    pub development_rows: usize,
    pub validation_rows: usize,
    pub test_rows: usize,
    pub duplicate_rows: usize,
    pub conflict_rows: usize,
    pub excluded_rows: usize,
    pub boundary: i32,
    pub score_scale: u32,
    pub false_warning_limit_basis_points: u16,
    pub validation: ConfusionMatrix,
    pub validation_metrics: Metrics,
    pub validation_gates: Option<GateResult>,
    pub artifact_bytes: usize,
    pub artifact_sha256: Sha256Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatasetInput {
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub revision: Option<String>,
    pub file_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInputs {
    pub dataset_inputs: Vec<DatasetInput>,
    pub prepared_counts: PreparedCounts,
    pub rule_pack_version: u16,
    pub rule_pack_sha256: Sha256Digest,
    pub hurtlex_sha256: Option<Sha256Digest>,
    pub clean_control_rows: usize,
    pub clean_control_sha256: Option<Sha256Digest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    pub schema_version: u16,
    pub entries: Vec<ModelManifestEntry>,
}

#[derive(Debug, Error)]
pub enum ModelSetError {
    #[error("cannot load the {} behavior panel: {source}", language.code())]
    BehaviorPanel {
        language: Language,
        #[source]
        source: crate::behavior_panel::BehaviorPanelError,
    },
    #[error("cannot read {path}: {source}")]
    CorpusIo {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse the source lock {path}: {reason}")]
    SourceLock {
        path: std::path::PathBuf,
        reason: String,
    },
    #[error("cannot load the {} corpus: {reason}", language.code())]
    Corpus { language: Language, reason: String },
    #[error("cannot parse model manifest JSON: {0}")]
    ModelManifestJson(#[from] serde_json::Error),
    #[error("invalid model manifest schema: expected {expected}, got {actual}")]
    InvalidModelManifestSchema { expected: u16, actual: u16 },
    #[error("invalid clean-control metadata for {}", .0.code())]
    CleanControlMetadataMismatch(Language),
    #[error("cannot parse prepared manifest JSON: {0}")]
    PreparedManifestJson(serde_json::Error),
    #[error("invalid prepared manifest schema: {actual}")]
    InvalidPreparedManifestSchema { actual: String },
    #[error("prepared manifest has the wrong {field} language key set")]
    PreparedLanguageKeySet { field: &'static str },
    #[error("prepared manifest has the wrong prepared-file key set")]
    PreparedFileKeySet,
    #[error("prepared file identity key {key} declares path {declared}")]
    PreparedIdentityPathMismatch { key: String, declared: String },
    #[error("prepared manifest repeats source record {0}")]
    DuplicateSourceRecord(String),
    #[error("prepared manifest repeats source {source_id} for {}", language.code())]
    DuplicateLanguageSourceId {
        language: Language,
        source_id: String,
    },
    #[error("prepared manifest references unknown source {source_id} for {}", language.code())]
    UnknownLanguageSource {
        language: Language,
        source_id: String,
    },
    #[error(
        "prepared source {source_id} has language {}; expected {}",
        actual.code(),
        expected.code()
    )]
    WrongLanguageSource {
        expected: Language,
        actual: Language,
        source_id: String,
    },
    #[error("prepared manifest has no language entry for {}", .0.code())]
    MissingPreparedLanguage(Language),
    #[error("prepared manifest has no file identity for {0}")]
    MissingPreparedIdentity(String),
    #[error(
        "prepared {split} count mismatch for {}: declared {declared}, file identity {file_rows}",
        language.code()
    )]
    PreparedSplitCountMismatch {
        language: Language,
        split: &'static str,
        declared: usize,
        file_rows: usize,
    },
    #[error("cannot read prepared file {}: {source}", path.display())]
    PreparedFileIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("prepared file digest mismatch for {path}: expected {expected}, got {actual}")]
    PreparedDigestMismatch {
        path: String,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("prepared {split} header mismatch for {}: {actual:?}", language.code())]
    PreparedHeaderMismatch {
        language: Language,
        split: &'static str,
        actual: Vec<String>,
    },
    #[error("cannot parse prepared TSV {path}: {source}")]
    PreparedCsv {
        path: String,
        #[source]
        source: csv::Error,
    },
    #[error("prepared file {path} contains invalid language {value}")]
    InvalidPreparedLanguage { path: String, value: String },
    #[error("prepared file {path} contains invalid label {value}")]
    InvalidPreparedLabel { path: String, value: String },
    #[error(
        "prepared {split} row {source_id} has language {}; expected {}",
        actual.code(),
        expected.code()
    )]
    PreparedRowLanguageMismatch {
        expected: Language,
        actual: Language,
        split: &'static str,
        source_id: String,
    },
    #[error("prepared file {path} has rows={rows}, clean={clean_rows}, toxic={toxic_rows}")]
    PreparedFileCountMismatch {
        path: String,
        rows: usize,
        clean_rows: usize,
        toxic_rows: usize,
    },
    #[error("prepared development and validation repeat source identifier {0}")]
    DuplicatePreparedSourceId(String),
    #[error(
        "compiled validation predictions for {} have length {actual}; expected {expected}",
        language.code()
    )]
    ValidationPredictionCountMismatch {
        language: Language,
        expected: usize,
        actual: usize,
    },
    #[error("compiled validation gates fail for {}", .0.code())]
    ValidationGateFailure(Language),
    #[error("model manifest entries do not follow the runtime language order")]
    ManifestEntryOrder,
    #[error("model manifest repeats language {}", .0.code())]
    DuplicateManifestEntry(Language),
    #[error("model manifest uses an invalid artifact path for {}", .0.code())]
    InvalidArtifactPath(Language),
    #[error("model manifest misses artifact for {}", .0.code())]
    MissingArtifact(Language),
    #[error("cannot read artifact for {}: {source}", language.code())]
    ArtifactIo {
        language: Language,
        #[source]
        source: std::io::Error,
    },
    #[error("artifact size mismatch for {}", .0.code())]
    ArtifactSizeMismatch(Language),
    #[error("artifact digest mismatch for {}", .0.code())]
    ArtifactDigestMismatch(Language),
    #[error("cannot parse artifact for {}: {source}", language.code())]
    ArtifactParse {
        language: Language,
        #[source]
        source: SparseModelError,
    },
    #[error("artifact metadata mismatch for {}", .0.code())]
    ArtifactMetadataMismatch(Language),
    #[error("validation metrics mismatch for {}", .0.code())]
    ValidationMetricsMismatch(Language),
    #[error("validation gates mismatch for {}", .0.code())]
    ValidationGatesMismatch(Language),
    #[error("rule-pack metadata mismatch for {}", .0.code())]
    RulePackMetadataMismatch(Language),
    #[error("model manifest misses a HurtLex digest for {}", .0.code())]
    MissingHurtlexDigest(Language),
    #[error("dataset inputs do not use canonical order for {}", .0.code())]
    DatasetInputOrder(Language),
    #[error("dataset inputs contain HurtLex for {}", .0.code())]
    HurtlexDatasetInput(Language),
    #[error(
        "validation row count mismatch for {}: declared {declared}, matrix total {actual}",
        language.code()
    )]
    ValidationRowCountMismatch {
        language: Language,
        declared: usize,
        actual: u64,
    },
    #[error("validation matrix total overflows for {}", .0.code())]
    ValidationMatrixTotalOverflow(Language),
    #[error("{} has {actual} HurtLex source records; expected one", language.code())]
    HurtlexSourceCount { language: Language, actual: usize },
    #[error("unsafe HurtLex path for {}: {path}", language.code())]
    UnsafeHurtlexPath { language: Language, path: String },
    #[error("cannot read HurtLex data for {} at {}: {source}", language.code(), path.display())]
    HurtlexIo {
        language: Language,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("HurtLex digest mismatch for {}", .0.code())]
    HurtlexDigestMismatch(Language),
    #[error("cannot build the rule channel for {}: {source}", language.code())]
    RuleChannel {
        language: Language,
        #[source]
        source: RuleChannelError,
    },
    #[error("cannot compile the sparse model for {}: {source}", language.code())]
    CompileLanguage {
        language: Language,
        #[source]
        source: CompileError,
    },
    #[error("the model-set output already exists: {}", .0.display())]
    PublicationDestinationExists(PathBuf),
    #[error("invalid model-set output path: {}", .0.display())]
    InvalidOutputPath(PathBuf),
    #[error("cannot write staged file {}: {source}", path.display())]
    StagingIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot serialize the model manifest: {0}")]
    ManifestSerialization(serde_json::Error),
    #[error("this target cannot publish the model set without replacement")]
    AtomicPublicationUnsupported,
    #[error("the atomic model-set {operation} failed: {source}")]
    AtomicPublicationIo {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
}

pub fn parse_model_manifest(reader: impl Read) -> Result<ModelManifest, ModelSetError> {
    let manifest: ModelManifest = serde_json::from_reader(reader)?;
    if manifest.schema_version != MODEL_MANIFEST_SCHEMA_VERSION {
        return Err(ModelSetError::InvalidModelManifestSchema {
            expected: MODEL_MANIFEST_SCHEMA_VERSION,
            actual: manifest.schema_version,
        });
    }
    Ok(manifest)
}

pub fn build_manifest_entry(
    compiled: &CompiledLanguage,
    mut inputs: ManifestInputs,
) -> Result<ModelManifestEntry, ModelSetError> {
    let language = compiled.calibration.language;
    if compiled.validation_predictions.len() != inputs.prepared_counts.validation {
        return Err(ModelSetError::ValidationPredictionCountMismatch {
            language,
            expected: inputs.prepared_counts.validation,
            actual: compiled.validation_predictions.len(),
        });
    }
    let validation_gates = gates(compiled.calibration.matrix);
    if !validation_gates.passed() {
        return Err(ModelSetError::ValidationGateFailure(language));
    }
    inputs.dataset_inputs.sort_by(|left, right| {
        (
            left.dataset,
            left.source_file_id.as_str(),
            left.revision.as_deref(),
            &left.file_sha256,
        )
            .cmp(&(
                right.dataset,
                right.source_file_id.as_str(),
                right.revision.as_deref(),
                &right.file_sha256,
            ))
    });
    let (feature_profile, normalization_profile, feature_schema) = language.profiles();
    let counts = inputs.prepared_counts;
    let validation = compiled.calibration.matrix;

    Ok(ModelManifestEntry {
        language,
        artifact_relative_path: artifact_relative_path(language).to_owned(),
        dataset_inputs: inputs.dataset_inputs,
        feature_profile,
        normalization_profile,
        feature_schema,
        rule_pack_version: inputs.rule_pack_version,
        rule_pack_sha256: inputs.rule_pack_sha256,
        hurtlex_sha256: inputs.hurtlex_sha256,
        clean_control_rows: inputs.clean_control_rows,
        clean_control_sha256: inputs.clean_control_sha256,
        development_rows: counts.development,
        validation_rows: counts.validation,
        test_rows: counts.test,
        duplicate_rows: counts.duplicates,
        conflict_rows: counts.conflicts,
        excluded_rows: counts.excluded,
        boundary: compiled.calibration.boundary,
        score_scale: compiled.score_scale,
        false_warning_limit_basis_points: 300,
        validation,
        validation_metrics: validation.metrics(),
        validation_gates: Some(validation_gates),
        artifact_bytes: compiled.artifact.len(),
        artifact_sha256: sha256(&compiled.artifact),
    })
}

pub fn validate_model_set(root: &Path, manifest: &ModelManifest) -> Result<(), ModelSetError> {
    if manifest.schema_version != MODEL_MANIFEST_SCHEMA_VERSION
        || manifest.entries.len() != Language::ALL.len()
    {
        return Err(ModelSetError::ManifestEntryOrder);
    }
    let mut languages = BTreeSet::new();
    for entry in &manifest.entries {
        if !languages.insert(entry.language) {
            return Err(ModelSetError::DuplicateManifestEntry(entry.language));
        }
    }
    for (entry, expected_language) in manifest.entries.iter().zip(Language::ALL) {
        if entry.language != expected_language {
            return Err(ModelSetError::ManifestEntryOrder);
        }
        if entry.artifact_relative_path != artifact_relative_path(entry.language) {
            return Err(ModelSetError::InvalidArtifactPath(entry.language));
        }
        validate_entry_metadata(entry)?;
        let path = root.join(&entry.artifact_relative_path);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(ModelSetError::MissingArtifact(entry.language));
            }
            Err(source) => {
                return Err(ModelSetError::ArtifactIo {
                    language: entry.language,
                    source,
                });
            }
        };
        if bytes.len() != entry.artifact_bytes {
            return Err(ModelSetError::ArtifactSizeMismatch(entry.language));
        }
        if sha256(&bytes) != entry.artifact_sha256 {
            return Err(ModelSetError::ArtifactDigestMismatch(entry.language));
        }
        let model =
            SparseModel::from_bytes(&bytes).map_err(|source| ModelSetError::ArtifactParse {
                language: entry.language,
                source,
            })?;
        if model.language() != entry.language
            || model.feature_profile() != entry.feature_profile
            || model.normalization_profile() != entry.normalization_profile
            || model.feature_schema() != entry.feature_schema
            || model.raw_boundary() != entry.boundary
            || model.score_scale() != entry.score_scale
            || entry.false_warning_limit_basis_points != 300
            || model.max_false_warning_basis_points() != entry.false_warning_limit_basis_points
        {
            return Err(ModelSetError::ArtifactMetadataMismatch(entry.language));
        }
    }
    Ok(())
}

fn validate_entry_metadata(entry: &ModelManifestEntry) -> Result<(), ModelSetError> {
    let expected_rule_sha256 = sha256(&canonical_rule_identity(entry.language));
    if entry.rule_pack_version != rule_pack_version(entry.language)
        || entry.rule_pack_sha256 != expected_rule_sha256
    {
        return Err(ModelSetError::RulePackMetadataMismatch(entry.language));
    }
    let hurtlex_sha256 = entry
        .hurtlex_sha256
        .as_ref()
        .ok_or(ModelSetError::MissingHurtlexDigest(entry.language))?;
    if entry.language == Language::Es && hurtlex_sha256.as_str() != SPANISH_HURTLEX_SHA256 {
        return Err(ModelSetError::HurtlexDigestMismatch(entry.language));
    }
    if (entry.clean_control_rows == 0) != entry.clean_control_sha256.is_none() {
        return Err(ModelSetError::CleanControlMetadataMismatch(entry.language));
    }
    if entry
        .dataset_inputs
        .iter()
        .any(|input| input.dataset == DatasetId::HurtLex)
    {
        return Err(ModelSetError::HurtlexDatasetInput(entry.language));
    }
    if entry
        .dataset_inputs
        .windows(2)
        .any(|pair| !dataset_input_precedes(&pair[0], &pair[1]))
    {
        return Err(ModelSetError::DatasetInputOrder(entry.language));
    }
    let matrix_rows = entry
        .validation
        .true_positive
        .checked_add(entry.validation.true_negative)
        .and_then(|total| total.checked_add(entry.validation.false_positive))
        .and_then(|total| total.checked_add(entry.validation.false_negative))
        .ok_or(ModelSetError::ValidationMatrixTotalOverflow(entry.language))?;
    if usize::try_from(matrix_rows).ok() != Some(entry.validation_rows) {
        return Err(ModelSetError::ValidationRowCountMismatch {
            language: entry.language,
            declared: entry.validation_rows,
            actual: matrix_rows,
        });
    }
    let expected_metrics = entry.validation.metrics();
    if entry.validation_metrics != expected_metrics {
        return Err(ModelSetError::ValidationMetricsMismatch(entry.language));
    }
    let expected_gates = gates(entry.validation);
    if entry.validation_gates != Some(expected_gates) || !expected_gates.passed() {
        return Err(ModelSetError::ValidationGatesMismatch(entry.language));
    }
    Ok(())
}

fn dataset_input_precedes(left: &DatasetInput, right: &DatasetInput) -> bool {
    (
        left.dataset,
        left.source_file_id.as_str(),
        left.revision.as_deref(),
        &left.file_sha256,
    ) < (
        right.dataset,
        right.source_file_id.as_str(),
        right.revision.as_deref(),
        &right.file_sha256,
    )
}

pub(crate) fn rule_pack_version(language: Language) -> u16 {
    if language == Language::Es {
        return 1;
    }
    word_rules(language)
        .or_else(|| arabic_hindi_rules(language))
        .or_else(|| cjk_rules(language))
        .expect("every non-Spanish language has a version-two rule pack")
        .version
}

#[must_use]
pub const fn artifact_relative_path(language: Language) -> &'static str {
    match language {
        Language::En => "en-sparse-v2.bin",
        Language::Zh => "zh-sparse-v2.bin",
        Language::Es => "es-chargram-v1.bin",
        Language::Ar => "ar-sparse-v2.bin",
        Language::Ms => "id-sparse-v2.bin",
        Language::Pt => "pt-sparse-v2.bin",
        Language::Fr => "fr-sparse-v2.bin",
        Language::Hi => "hi-sparse-v2.bin",
        Language::Ru => "ru-sparse-v2.bin",
        Language::Ja => "ja-sparse-v2.bin",
        Language::De => "de-sparse-v2.bin",
        Language::Tr => "tr-sparse-v2.bin",
        Language::Vi => "vi-sparse-v2.bin",
        Language::Ko => "ko-sparse-v2.bin",
        Language::It => "it-sparse-v2.bin",
    }
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("SHA-256 output is a valid digest")
}
