use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use blasphem::{
    ConfusionMatrix, FeatureProfile, FeatureSchema, Language, Metrics, NormalizationProfile,
    RuleChannelError, SparseModel, SparseModelError, arabic_hindi_rules, canonical_rule_identity,
    cjk_rules, word_rules,
};

use crate::{
    calibration::{GateResult, gates},
    compiler::{CompileError, CompiledLanguage},
    datasets::{DatasetId, PreparedCounts},
    evidence::Sha256Digest,
};

pub const MODEL_MANIFEST_SCHEMA_VERSION: u16 = 2;
pub const SPANISH_LEGACY_SCHEMA_VERSION: u16 = 1;
const SPANISH_HURTLEX_SHA256: &str =
    "5adadf7886ea332e6e07de1f5abb98a71a3dacbf3bea993b21100c9b4bffd4ba";

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FrozenFileReference {
    pub relative_path: String,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpanishLegacyInput {
    pub schema_version: u16,
    pub artifact: FrozenFileReference,
    pub metadata: FrozenFileReference,
    pub source: FrozenFileReference,
    pub hurtlex: FrozenFileReference,
    pub proof_report: FrozenFileReference,
    pub behavior_panel: FrozenFileReference,
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub dataset_revision: String,
    pub source_rows: usize,
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
    pub test: ConfusionMatrix,
    pub behavior: ConfusionMatrix,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    pub schema_version: u16,
    pub entries: Vec<ModelManifestEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedSpanishLegacy {
    pub artifact: Vec<u8>,
    pub entry: ModelManifestEntry,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpanishLegacyMetadata {
    format: String,
    language: String,
    artifact: String,
    artifact_bytes: usize,
    artifact_sha256: Sha256Digest,
    dataset: String,
    dataset_revision: String,
    source_rows: usize,
    evaluation_rows: usize,
    duplicate_rows: usize,
    conflict_rows: usize,
    split_method: String,
    development_rows: usize,
    validation_rows: usize,
    test_rows: usize,
    algorithm: String,
    feature_bins: usize,
    features: Vec<String>,
    minimum_document_frequency: u32,
    weight_scale: u16,
    decision_boundary: i32,
    score_scale: u32,
    maximum_validation_false_positive_basis_points: u16,
    sparse_validation: ConfusionMatrix,
}

#[derive(Debug, Error)]
pub enum ModelSetError {
    #[error("cannot load the {} behavior panel: {source}", language.code())]
    BehaviorPanel {
        language: Language,
        #[source]
        source: crate::behavior_panel::BehaviorPanelError,
    },
    #[error("cannot parse model manifest JSON: {0}")]
    ModelManifestJson(#[from] serde_json::Error),
    #[error("invalid model manifest schema: expected {expected}, got {actual}")]
    InvalidModelManifestSchema { expected: u16, actual: u16 },
    #[error("invalid clean-control metadata for {}", .0.code())]
    CleanControlMetadataMismatch(Language),
    #[error("cannot parse Spanish legacy declaration JSON: {0}")]
    SpanishLegacyJson(serde_json::Error),
    #[error("invalid Spanish legacy schema: expected {expected}, got {actual}")]
    InvalidSpanishLegacySchema { expected: u16, actual: u16 },
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
    #[error("Spanish has no prepared version-two input")]
    SpanishPreparedInput,
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
    #[error("Spanish manifest entries must use the frozen legacy declaration")]
    SpanishCompiledEntry,
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
    #[error("invalid Spanish legacy declaration path: {}", .0.display())]
    InvalidSpanishDeclarationPath(PathBuf),
    #[error("cannot read Spanish legacy declaration {}: {source}", path.display())]
    SpanishDeclarationIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Spanish legacy declaration mismatch: {0}")]
    SpanishDeclarationMismatch(&'static str),
    #[error("unsafe frozen file path: {0}")]
    UnsafeFrozenPath(String),
    #[error("cannot read frozen file {relative_path}: {source}")]
    FrozenFileIo {
        relative_path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("frozen file digest mismatch: {0}")]
    FrozenFileDigestMismatch(String),
    #[error("cannot parse Spanish legacy metadata JSON: {0}")]
    SpanishMetadataJson(serde_json::Error),
    #[error("Spanish legacy metadata mismatch: {0}")]
    SpanishMetadataMismatch(&'static str),
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

pub fn parse_spanish_legacy_input(reader: impl Read) -> Result<SpanishLegacyInput, ModelSetError> {
    let input: SpanishLegacyInput =
        serde_json::from_reader(reader).map_err(ModelSetError::SpanishLegacyJson)?;
    if input.schema_version != SPANISH_LEGACY_SCHEMA_VERSION {
        return Err(ModelSetError::InvalidSpanishLegacySchema {
            expected: SPANISH_LEGACY_SCHEMA_VERSION,
            actual: input.schema_version,
        });
    }
    Ok(input)
}

pub fn load_spanish_legacy(
    declaration_path: &Path,
) -> Result<VerifiedSpanishLegacy, ModelSetError> {
    let canonical = fs::canonicalize(declaration_path).map_err(|source| {
        ModelSetError::SpanishDeclarationIo {
            path: declaration_path.to_owned(),
            source,
        }
    })?;
    if !canonical.ends_with("resources/models/es-legacy-input-v1.json") {
        return Err(ModelSetError::InvalidSpanishDeclarationPath(canonical));
    }
    let project_root = canonical
        .ancestors()
        .nth(3)
        .ok_or_else(|| ModelSetError::InvalidSpanishDeclarationPath(canonical.clone()))?;
    let declaration_bytes =
        fs::read(&canonical).map_err(|source| ModelSetError::SpanishDeclarationIo {
            path: canonical.clone(),
            source,
        })?;
    let input = parse_spanish_legacy_input(declaration_bytes.as_slice())?;
    validate_spanish_declaration(&input)?;

    let artifact = read_frozen_file(project_root, &input.artifact)?;
    let metadata_bytes = read_frozen_file(project_root, &input.metadata)?;
    read_frozen_file(project_root, &input.source)?;
    read_frozen_file(project_root, &input.hurtlex)?;
    read_frozen_file(project_root, &input.proof_report)?;
    read_frozen_file(project_root, &input.behavior_panel)?;
    validate_spanish_metadata(&metadata_bytes, &input)?;
    let model =
        SparseModel::from_bytes(&artifact).map_err(|source| ModelSetError::ArtifactParse {
            language: Language::Es,
            source,
        })?;
    let profiles = Language::Es.profiles();
    if model.language() != Language::Es
        || (
            model.feature_profile(),
            model.normalization_profile(),
            model.feature_schema(),
        ) != profiles
        || model.raw_boundary() != input.boundary
        || model.score_scale() != input.score_scale
        || model.max_false_warning_basis_points() != input.false_warning_limit_basis_points
    {
        return Err(ModelSetError::ArtifactMetadataMismatch(Language::Es));
    }

    let validation = input.validation;
    let entry = ModelManifestEntry {
        language: Language::Es,
        artifact_relative_path: artifact_relative_path(Language::Es).to_owned(),
        dataset_inputs: vec![DatasetInput {
            dataset: input.dataset,
            source_file_id: input.source_file_id,
            revision: Some(input.dataset_revision),
            file_sha256: input.source.sha256,
        }],
        feature_profile: profiles.0,
        normalization_profile: profiles.1,
        feature_schema: profiles.2,
        rule_pack_version: 1,
        rule_pack_sha256: sha256(&canonical_rule_identity(Language::Es)),
        hurtlex_sha256: Some(input.hurtlex.sha256),
        clean_control_rows: 0,
        clean_control_sha256: None,
        development_rows: input.development_rows,
        validation_rows: input.validation_rows,
        test_rows: input.test_rows,
        duplicate_rows: input.duplicate_rows,
        conflict_rows: input.conflict_rows,
        excluded_rows: input.excluded_rows,
        boundary: input.boundary,
        score_scale: input.score_scale,
        false_warning_limit_basis_points: input.false_warning_limit_basis_points,
        validation,
        validation_metrics: validation.metrics(),
        validation_gates: None,
        artifact_bytes: artifact.len(),
        artifact_sha256: sha256(&artifact),
    };
    Ok(VerifiedSpanishLegacy { artifact, entry })
}

fn validate_spanish_declaration(input: &SpanishLegacyInput) -> Result<(), ModelSetError> {
    let expected_files = [
        (
            &input.artifact,
            "resources/models/es-chargram-v1.bin",
            "3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36",
        ),
        (
            &input.metadata,
            "resources/models/es-chargram-v1.json",
            "b5c334f79334b20843409ef9bbebdd4fcbce9580239ae6f9f496f14bcf4ba582",
        ),
        (
            &input.source,
            "data/textdetox/es-source.tsv",
            "8e3c8078d7406e7b695ffb943e0439240ada11d6abc9d12ac313efdb6d2f1da9",
        ),
        (
            &input.hurtlex,
            "data/raw-v1/hurtlex/ES/1.2/hurtlex_ES.tsv",
            "5adadf7886ea332e6e07de1f5abb98a71a3dacbf3bea993b21100c9b4bffd4ba",
        ),
        (
            &input.proof_report,
            "docs/spanish-proof-report.md",
            "7634a66dfd43e22aac8d729ce5d06cbd1384aafe5f4ddb24c77a087039337d42",
        ),
        (
            &input.behavior_panel,
            "samples/spanish-audit.tsv",
            "8313713f8e18e5c066f6f320efb6ee340b7580cba4739fc4612e1dfe4a8a7575",
        ),
    ];
    for (reference, path, digest) in expected_files {
        if reference.relative_path != path || reference.sha256.as_str() != digest {
            return Err(ModelSetError::SpanishDeclarationMismatch("frozen file"));
        }
    }
    let expected_validation = ConfusionMatrix {
        true_positive: 159,
        true_negative: 382,
        false_positive: 18,
        false_negative: 203,
    };
    let expected_test = ConfusionMatrix {
        true_positive: 177,
        true_negative: 386,
        false_positive: 14,
        false_negative: 242,
    };
    let expected_behavior = ConfusionMatrix {
        true_positive: 39,
        true_negative: 46,
        false_positive: 0,
        false_negative: 3,
    };
    if input.dataset != DatasetId::TextDetox
        || input.source_file_id != "textdetox-es-legacy"
        || input.dataset_revision != "01907546324b0330d2d8b7669648cc18823323e5"
        || (
            input.source_rows,
            input.development_rows,
            input.validation_rows,
            input.test_rows,
        ) != (5_000, 3_418, 762, 819)
        || (
            input.duplicate_rows,
            input.conflict_rows,
            input.excluded_rows,
        ) != (1, 0, 1)
        || (
            input.boundary,
            input.score_scale,
            input.false_warning_limit_basis_points,
        ) != (10_962, 27_695, 300)
        || input.validation != expected_validation
        || input.test != expected_test
        || input.behavior != expected_behavior
    {
        return Err(ModelSetError::SpanishDeclarationMismatch("metadata"));
    }
    Ok(())
}

fn read_frozen_file(
    project_root: &Path,
    reference: &FrozenFileReference,
) -> Result<Vec<u8>, ModelSetError> {
    let path = Path::new(&reference.relative_path);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ModelSetError::UnsafeFrozenPath(
            reference.relative_path.clone(),
        ));
    }
    let bytes =
        fs::read(project_root.join(path)).map_err(|source| ModelSetError::FrozenFileIo {
            relative_path: reference.relative_path.clone(),
            source,
        })?;
    if sha256(&bytes) != reference.sha256 {
        return Err(ModelSetError::FrozenFileDigestMismatch(
            reference.relative_path.clone(),
        ));
    }
    Ok(bytes)
}

fn validate_spanish_metadata(
    bytes: &[u8],
    input: &SpanishLegacyInput,
) -> Result<(), ModelSetError> {
    let metadata: SpanishLegacyMetadata =
        serde_json::from_slice(bytes).map_err(ModelSetError::SpanishMetadataJson)?;
    let expected_sparse = ConfusionMatrix {
        true_positive: 152,
        true_negative: 388,
        false_positive: 12,
        false_negative: 210,
    };
    let expected_features = [
        "normalized word unigrams",
        "normalized word bigrams",
        "normalized character 3-grams",
        "normalized character 4-grams",
        "normalized character 5-grams",
    ];
    if metadata.format != "TOXSPRS1"
        || metadata.language != "ES"
        || metadata.artifact != "es-chargram-v1.bin"
        || metadata.artifact_bytes != 131_104
        || metadata.artifact_sha256 != input.artifact.sha256
        || metadata.dataset != "textdetox/multilingual_toxicity_dataset"
        || metadata.dataset_revision != input.dataset_revision
        || metadata.source_rows != input.source_rows
        || metadata.evaluation_rows != input.source_rows - input.excluded_rows
        || metadata.duplicate_rows != input.duplicate_rows
        || metadata.conflict_rows != input.conflict_rows
        || metadata.split_method != "FNV-1a over detector language and normalized text"
        || metadata.development_rows != input.development_rows
        || metadata.validation_rows != input.validation_rows
        || metadata.test_rows != input.test_rows
        || metadata.algorithm != "Bernoulli log-odds with fixed feature hashing"
        || metadata.feature_bins != 65_536
        || metadata
            .features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != expected_features
        || metadata.minimum_document_frequency != 2
        || metadata.weight_scale != 256
        || metadata.decision_boundary != input.boundary
        || metadata.score_scale != input.score_scale
        || metadata.maximum_validation_false_positive_basis_points
            != input.false_warning_limit_basis_points
        || metadata.sparse_validation != expected_sparse
    {
        return Err(ModelSetError::SpanishMetadataMismatch("frozen metadata"));
    }
    Ok(())
}

pub fn build_manifest_entry(
    compiled: &CompiledLanguage,
    mut inputs: ManifestInputs,
) -> Result<ModelManifestEntry, ModelSetError> {
    let language = compiled.calibration.language;
    if language == Language::Es {
        return Err(ModelSetError::SpanishCompiledEntry);
    }
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
    if entry.language == Language::Es {
        if entry.validation_gates.is_some() {
            return Err(ModelSetError::ValidationGatesMismatch(entry.language));
        }
    } else {
        let expected = gates(entry.validation);
        if entry.validation_gates != Some(expected) || !expected.passed() {
            return Err(ModelSetError::ValidationGatesMismatch(entry.language));
        }
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
