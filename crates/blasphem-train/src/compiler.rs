use std::{
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{
        Once,
        atomic::{AtomicU64, Ordering},
    },
};

use blasphem::{
    ConfusionMatrix, EvalLabel, FeatureError, FeatureProfile, Language, NormalizationProfile,
    ReplyTarget, RuleChannel, SparseInput, SparseModel, SparseModelError, canonical_rule_identity,
    encode_sparse, extract_feature_bins, lexicon_marked_text, uses_lexicon_features,
};
use liblinear::{LibLinearModel, SolverType, util::TrainingInput};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    atomic_publish::{AtomicPublishError, atomic_publish_noreplace},
    behavior_panel::load_panel,
    calibration::{CalibrationError, CalibrationResult, CalibrationRow, calibrate_at_or_above},
    corpus::{PreparedLanguageInput, load_corpus_language},
    datasets::{DatasetId, PreparedRow},
    evidence::Sha256Digest,
    model_manifest::{
        DatasetInput, MODEL_MANIFEST_SCHEMA_VERSION, ManifestInputs, ModelManifest,
        ModelManifestEntry, ModelSetError, artifact_relative_path, build_manifest_entry,
        parse_model_manifest, rule_pack_version, validate_model_set,
    },
    source_manifest::{FrozenSourceLock, SourceRecord, parse_frozen_source_lock},
};

const BIN_COUNT: usize = 65_536;
const WEIGHT_SCALE: u16 = 256;
const MIN_DOCUMENT_FREQUENCY: u32 = 2;
const FALSE_WARNING_LIMIT_BASIS_POINTS: u16 = 300;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static SILENCE_LIBLINEAR: Once = Once::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCompileOptions {
    pub corpus_root: PathBuf,
    pub source_lock: PathBuf,
    pub lexicon_root: PathBuf,
    pub behavior_root: Option<PathBuf>,
    pub output: PathBuf,
    pub manifest_output: PathBuf,
}

struct CompiledModel {
    artifact: Vec<u8>,
    entry: ModelManifestEntry,
}

pub fn compile_model_set(options: &BatchCompileOptions) -> Result<ModelManifest, ModelSetError> {
    if options.manifest_output.starts_with(&options.output) {
        return Err(ModelSetError::InvalidOutputPath(
            options.manifest_output.clone(),
        ));
    }
    reject_existing_output(&options.output)?;
    reject_existing_output(&options.manifest_output)?;
    let lock = read_source_lock(&options.source_lock)?;
    let mut models = Vec::with_capacity(Language::ALL.len());
    for language in Language::ALL {
        models.push(compile_corpus_language(options, &lock, language)?);
    }
    let manifest = ModelManifest {
        schema_version: MODEL_MANIFEST_SCHEMA_VERSION,
        entries: models.iter().map(|model| model.entry.clone()).collect(),
    };
    publish_model_set(options, &models, &manifest)
}

fn reject_existing_output(output: &Path) -> Result<(), ModelSetError> {
    match fs::symlink_metadata(output) {
        Ok(_) => Err(ModelSetError::PublicationDestinationExists(
            output.to_owned(),
        )),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ModelSetError::StagingIo {
            path: output.to_owned(),
            source,
        }),
    }
}

fn read_source_lock(path: &Path) -> Result<FrozenSourceLock, ModelSetError> {
    let file = File::open(path).map_err(|source| ModelSetError::CorpusIo {
        path: path.to_owned(),
        source,
    })?;
    parse_frozen_source_lock(file).map_err(|error| ModelSetError::SourceLock {
        path: path.to_owned(),
        reason: error.to_string(),
    })
}

fn compile_corpus_language(
    options: &BatchCompileOptions,
    lock: &FrozenSourceLock,
    language: Language,
) -> Result<CompiledModel, ModelSetError> {
    let PreparedLanguageInput {
        development,
        validation,
        counts,
        sources,
        ..
    } = load_corpus_language(&options.corpus_root, language, lock).map_err(|error| {
        ModelSetError::Corpus {
            language,
            reason: error.to_string(),
        }
    })?;
    let lexicon_sources = sources
        .iter()
        .filter(|source| source.dataset == DatasetId::Lexicon)
        .collect::<Vec<_>>();
    if lexicon_sources.len() != 1 {
        return Err(ModelSetError::LexiconSourceCount {
            language,
            actual: lexicon_sources.len(),
        });
    }
    let lexicon_source = lexicon_sources[0];
    let lexicon_bytes = read_lexicon(options, language, lexicon_source)?;
    let rule_channel = RuleChannel::from_lexicon_bytes(language, Some(&lexicon_bytes))
        .map_err(|source| ModelSetError::RuleChannel { language, source })?;
    let behavior_rows = options
        .behavior_root
        .as_ref()
        .map(|root| load_panel(root, language))
        .transpose()
        .map_err(|source| ModelSetError::BehaviorPanel { language, source })?
        .unwrap_or_default();
    let clean_controls = behavior_rows
        .into_iter()
        .filter(|row| !row.expected_nudge)
        .map(|row| row.text)
        .collect::<Vec<_>>();
    let clean_control_sha256 =
        (!clean_controls.is_empty()).then(|| clean_control_identity(language, &clean_controls));
    let clean_control_rows = clean_controls.len();
    let compiled = compile_language(&CompileRequest {
        language,
        development,
        validation,
        rule_channel,
        clean_controls,
    })
    .map_err(|source| ModelSetError::CompileLanguage { language, source })?;
    let dataset_inputs = sources
        .iter()
        .filter(|source| source.dataset != DatasetId::Lexicon)
        .map(|source| DatasetInput {
            dataset: source.dataset,
            source_file_id: source.source_file_id.clone(),
            revision: source.revision.clone(),
            file_sha256: source.file_sha256.clone(),
        })
        .collect();
    let entry = build_manifest_entry(
        &compiled,
        ManifestInputs {
            dataset_inputs,
            prepared_counts: counts,
            rule_pack_version: rule_pack_version(language),
            rule_pack_sha256: sha256_digest(&canonical_rule_identity(language)),
            lexicon_sha256: Some(lexicon_source.file_sha256.clone()),
            clean_control_rows,
            clean_control_sha256,
        },
    )?;
    Ok(CompiledModel {
        artifact: compiled.artifact,
        entry,
    })
}

fn read_lexicon(
    options: &BatchCompileOptions,
    language: Language,
    source_record: &SourceRecord,
) -> Result<Vec<u8>, ModelSetError> {
    let declared = Path::new(&source_record.file_path);
    let relative = declared.strip_prefix("resources/lexicon").map_err(|_| {
        ModelSetError::UnsafeLexiconPath {
            language,
            path: source_record.file_path.clone(),
        }
    })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ModelSetError::UnsafeLexiconPath {
            language,
            path: source_record.file_path.clone(),
        });
    }
    let path = options.lexicon_root.join(relative);
    let bytes = fs::read(&path).map_err(|source| ModelSetError::LexiconIo {
        language,
        path,
        source,
    })?;
    if sha256_digest(&bytes) != source_record.file_sha256 {
        return Err(ModelSetError::LexiconDigestMismatch(language));
    }
    Ok(bytes)
}

fn publish_model_set(
    options: &BatchCompileOptions,
    models: &[CompiledModel],
    manifest: &ModelManifest,
) -> Result<ModelManifest, ModelSetError> {
    let staging = create_staging_directory(&options.output)?;
    let guard = StagingGuard(staging.clone());
    for model in models {
        write_staged_file(
            &staging.join(artifact_relative_path(model.entry.language)),
            &model.artifact,
        )?;
    }
    let mut manifest_bytes =
        serde_json::to_vec_pretty(manifest).map_err(ModelSetError::ManifestSerialization)?;
    manifest_bytes.push(b'\n');
    let (staged_manifest, mut manifest_file) = create_staging_file(&options.manifest_output)?;
    let manifest_guard = StagingGuard(staged_manifest.clone());
    manifest_file
        .write_all(&manifest_bytes)
        .and_then(|()| manifest_file.flush())
        .and_then(|()| manifest_file.sync_all())
        .map_err(|source| ModelSetError::StagingIo {
            path: staged_manifest.clone(),
            source,
        })?;
    let parsed = parse_model_manifest(
        fs::read(&staged_manifest)
            .map_err(|source| ModelSetError::StagingIo {
                path: staged_manifest.clone(),
                source,
            })?
            .as_slice(),
    )?;
    validate_model_set(&staging, &parsed)?;
    if let Err(error) = atomic_publish_noreplace(&staging, &options.output) {
        if matches!(&error, AtomicPublishError::ParentSync(_)) {
            fs::remove_dir_all(&options.output).map_err(|source| {
                ModelSetError::AtomicPublicationIo {
                    operation: "model publication rollback",
                    source,
                }
            })?;
        }
        return Err(map_atomic_error(error, &options.output));
    }
    drop(guard);
    if let Err(error) = atomic_publish_noreplace(&staged_manifest, &options.manifest_output) {
        let mut rollback_error = None;
        if matches!(&error, AtomicPublishError::ParentSync(_)) {
            if let Err(source) = fs::remove_file(&options.manifest_output) {
                rollback_error = Some(source);
            }
        }
        if let Err(source) = fs::remove_dir_all(&options.output) {
            rollback_error.get_or_insert(source);
        }
        if let Some(source) = rollback_error {
            return Err(ModelSetError::AtomicPublicationIo {
                operation: "publication rollback",
                source,
            });
        }
        return Err(map_atomic_error(error, &options.manifest_output));
    }
    drop(manifest_guard);
    Ok(parsed)
}

fn create_staging_directory(output: &Path) -> Result<PathBuf, ModelSetError> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .ok_or_else(|| ModelSetError::InvalidOutputPath(output.to_owned()))?
        .to_string_lossy();
    for _ in 0..100 {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let staging = parent.join(format!(".{name}.staging-{}-{sequence}", std::process::id()));
        match fs::create_dir(&staging) {
            Ok(()) => return Ok(staging),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(ModelSetError::StagingIo {
                    path: staging,
                    source,
                });
            }
        }
    }
    Err(ModelSetError::StagingIo {
        path: parent.to_owned(),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "cannot allocate a unique staging directory",
        ),
    })
}

fn create_staging_file(output: &Path) -> Result<(PathBuf, File), ModelSetError> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| ModelSetError::StagingIo {
        path: parent.to_owned(),
        source,
    })?;
    let name = output
        .file_name()
        .ok_or_else(|| ModelSetError::InvalidOutputPath(output.to_owned()))?
        .to_string_lossy();
    for _ in 0..100 {
        let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let staging = parent.join(format!(".{name}.staging-{}-{sequence}", std::process::id()));
        match File::options().write(true).create_new(true).open(&staging) {
            Ok(file) => return Ok((staging, file)),
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(ModelSetError::StagingIo {
                    path: staging,
                    source,
                });
            }
        }
    }
    Err(ModelSetError::StagingIo {
        path: parent.to_owned(),
        source: std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "cannot allocate a unique staging path",
        ),
    })
}

fn write_staged_file(path: &Path, bytes: &[u8]) -> Result<(), ModelSetError> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| ModelSetError::StagingIo {
            path: path.to_owned(),
            source,
        })?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all())
        .map_err(|source| ModelSetError::StagingIo {
            path: path.to_owned(),
            source,
        })
}

fn map_atomic_error(error: AtomicPublishError, output: &Path) -> ModelSetError {
    match error {
        AtomicPublishError::DestinationExists => {
            ModelSetError::PublicationDestinationExists(output.to_owned())
        }
        AtomicPublishError::Unsupported => ModelSetError::AtomicPublicationUnsupported,
        AtomicPublishError::Rename(source) => ModelSetError::AtomicPublicationIo {
            operation: "rename",
            source,
        },
        AtomicPublishError::StagingSync(source) => ModelSetError::AtomicPublicationIo {
            operation: "staging sync",
            source,
        },
        AtomicPublishError::ParentSync(source) => ModelSetError::AtomicPublicationIo {
            operation: "parent sync",
            source,
        },
        AtomicPublishError::Cleanup(source) => ModelSetError::AtomicPublicationIo {
            operation: "staging cleanup",
            source,
        },
    }
}

struct StagingGuard(PathBuf);

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.0.is_dir() {
            let _ = fs::remove_dir_all(&self.0);
        } else if self.0.exists() {
            let _ = fs::remove_file(&self.0);
        }
    }
}

fn sha256_digest(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("SHA-256 output is a valid digest")
}

fn clean_control_identity(language: Language, controls: &[String]) -> Sha256Digest {
    let mut hash = Sha256::new();
    hash.update(b"TOXCLEAN1");
    hash.update(language.code().as_bytes());
    hash.update(
        u32::try_from(controls.len())
            .expect("clean-control count fits in u32")
            .to_le_bytes(),
    );
    for control in controls {
        hash.update(
            u32::try_from(control.len())
                .expect("clean-control length fits in u32")
                .to_le_bytes(),
        );
        hash.update(control.as_bytes());
    }
    let digest = hash.finalize();
    format!("{digest:x}")
        .try_into()
        .expect("SHA-256 output is a valid digest")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainedWeights {
    pub bias: i32,
    pub weights: Box<[i16]>,
}

#[derive(Clone, Copy)]
struct TrainingRows<'a> {
    profile: FeatureProfile,
    normalization: NormalizationProfile,
    development: &'a [PreparedRow],
}

/// Selects the learner while preserving the language's feature and artifact profiles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Learner {
    LogOdds,
    Logistic(LogisticOptions),
    NaiveBayesLogistic { cost: f64, interpolation: f64 },
}

/// Development-only settings for the fixed-bin logistic learner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogisticOptions {
    pub cost: f64,
    pub class_weighted: bool,
    pub minimum_document_frequency: u32,
}

struct PreparedFeatures {
    labels: Vec<f64>,
    features: Vec<Vec<(u32, f64)>>,
    clean_documents: u32,
    toxic_documents: u32,
}

struct NaiveBayesRatios {
    values: Vec<f64>,
    active_bins: Vec<usize>,
}

fn default_learner(language: Language) -> Learner {
    match language {
        Language::Es => Learner::NaiveBayesLogistic {
            cost: 1.0,
            interpolation: 1.0,
        },
        Language::Tr => Learner::Logistic(LogisticOptions {
            cost: 1.0,
            class_weighted: true,
            minimum_document_frequency: 1,
        }),
        Language::Ko => Learner::Logistic(LogisticOptions {
            cost: 0.15,
            class_weighted: true,
            minimum_document_frequency: 1,
        }),
        _ => Learner::LogOdds,
    }
}

pub struct CompileRequest {
    pub language: Language,
    pub development: Vec<PreparedRow>,
    pub validation: Vec<PreparedRow>,
    pub rule_channel: RuleChannel,
    pub clean_controls: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledLanguage {
    pub artifact: Vec<u8>,
    pub calibration: CalibrationResult,
    pub score_scale: u32,
    pub validation_predictions: Vec<bool>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CompileError {
    #[error("the {split} split is empty")]
    EmptySplit { split: &'static str },
    #[error("the clean control {source_id} for {} triggers the rule channel", language.code())]
    CleanControlRuleNudge {
        language: Language,
        source_id: String,
    },
    #[error(
        "the {split} row {source_id} has language {}; expected {}",
        actual.code(),
        expected.code()
    )]
    LanguageMismatch {
        expected: Language,
        actual: Language,
        split: &'static str,
        source_id: String,
    },
    #[error("the {split} split for {} has no {label} rows", language.code())]
    MissingClass {
        language: Language,
        split: &'static str,
        label: &'static str,
    },
    #[error(
        "the profiles {feature:?} and {normalization:?} do not match language {}",
        language.code()
    )]
    ProfileMismatch {
        language: Language,
        feature: FeatureProfile,
        normalization: NormalizationProfile,
    },
    #[error("cannot extract features for {split} row {source_id}: {source}")]
    FeatureExtraction {
        split: &'static str,
        source_id: String,
        #[source]
        source: FeatureError,
    },
    #[error("the logistic cost must be finite and greater than zero")]
    InvalidLogisticCost,
    #[error("the Naive Bayes interpolation must be finite and between zero and one")]
    InvalidInterpolation,
    #[error("cannot train the logistic model: {0}")]
    LinearTraining(String),
    #[error("the validation score set is empty")]
    EmptyScoreScaleInput,
    #[error(
        "the rule channel has language {}; expected {}",
        actual.code(),
        expected.code()
    )]
    RuleChannelLanguageMismatch {
        expected: Language,
        actual: Language,
    },
    #[error(transparent)]
    Calibration(#[from] CalibrationError),
    #[error("cannot encode or parse the sparse artifact: {0}")]
    Artifact(#[from] SparseModelError),
    #[error("the encoded artifact changes the validation prediction for {source_id}")]
    ArtifactPredictionMismatch { source_id: String },
    #[error("the encoded artifact changes the validation matrix for {}", .0.code())]
    ArtifactMatrixMismatch(Language),
}

pub fn train_weights(
    profile: FeatureProfile,
    normalization: NormalizationProfile,
    development: &[PreparedRow],
) -> Result<TrainedWeights, CompileError> {
    let learner = development.first().map_or(Learner::LogOdds, |row| {
        default_learner(row.detector_language)
    });
    train_weights_with_learner(
        TrainingRows {
            profile,
            normalization,
            development,
        },
        learner,
    )
}

fn train_weights_with_learner(
    rows: TrainingRows<'_>,
    learner: Learner,
) -> Result<TrainedWeights, CompileError> {
    let TrainingRows {
        profile,
        normalization,
        development,
    } = rows;
    let Some(first) = development.first() else {
        return Err(CompileError::EmptySplit {
            split: "development",
        });
    };
    let language = first.detector_language;
    let (expected_profile, expected_normalization, _) = language.profiles();
    if (profile, normalization) != (expected_profile, expected_normalization) {
        return Err(CompileError::ProfileMismatch {
            language,
            feature: profile,
            normalization,
        });
    }
    validate_compile_split("development", language, development)?;
    match learner {
        Learner::LogOdds => train_log_odds(rows),
        Learner::Logistic(options) => train_logistic(rows, options),
        Learner::NaiveBayesLogistic {
            cost,
            interpolation,
        } => train_naive_bayes_logistic(rows, cost, interpolation),
    }
}

fn train_log_odds(rows: TrainingRows<'_>) -> Result<TrainedWeights, CompileError> {
    let TrainingRows {
        profile,
        normalization,
        development,
    } = rows;
    let mut clean = vec![0_u32; BIN_COUNT];
    let mut toxic = vec![0_u32; BIN_COUNT];
    let mut clean_documents = 0_u32;
    let mut toxic_documents = 0_u32;
    for row in development {
        let (counts, documents) = match row.label {
            EvalLabel::Clean => (&mut clean, &mut clean_documents),
            EvalLabel::Toxic => (&mut toxic, &mut toxic_documents),
        };
        *documents = documents.saturating_add(1);
        let bins = extract_feature_bins(profile, normalization, &row.text).map_err(|source| {
            CompileError::FeatureExtraction {
                split: "development",
                source_id: row.source_id.clone(),
                source,
            }
        })?;
        for bin in bins {
            counts[bin] = counts[bin].saturating_add(1);
        }
    }

    Ok(quantize_log_odds(
        &clean,
        &toxic,
        clean_documents,
        toxic_documents,
    ))
}

fn train_logistic(
    rows: TrainingRows<'_>,
    options: LogisticOptions,
) -> Result<TrainedWeights, CompileError> {
    validate_logistic_cost(options.cost)?;
    let prepared = prepare_linear_features(rows, options.minimum_document_frequency)?;
    let model = fit_logistic(prepared, options)?;
    quantize_linear_model(&model)
}

fn validate_logistic_cost(cost: f64) -> Result<(), CompileError> {
    if !cost.is_finite() || cost <= 0.0 {
        return Err(CompileError::InvalidLogisticCost);
    }
    Ok(())
}

fn prepare_linear_features(
    rows: TrainingRows<'_>,
    minimum_document_frequency: u32,
) -> Result<PreparedFeatures, CompileError> {
    let TrainingRows {
        profile,
        normalization,
        development,
    } = rows;
    let mut labels = Vec::with_capacity(development.len());
    let mut features = Vec::with_capacity(development.len());
    let mut clean_documents = 0_u32;
    let mut toxic_documents = 0_u32;
    for row in development {
        let label = match row.label {
            EvalLabel::Clean => {
                clean_documents = clean_documents.saturating_add(1);
                -1.0
            }
            EvalLabel::Toxic => {
                toxic_documents = toxic_documents.saturating_add(1);
                1.0
            }
        };
        let bins = extract_feature_bins(profile, normalization, &row.text).map_err(|source| {
            CompileError::FeatureExtraction {
                split: "development",
                source_id: row.source_id.clone(),
                source,
            }
        })?;
        labels.push(label);
        features.push(
            bins.into_iter()
                .map(|bin| {
                    (
                        u32::try_from(bin + 1).expect("the feature index fits in u32"),
                        1.0,
                    )
                })
                .collect::<Vec<_>>(),
        );
    }
    filter_rare_features(&mut features, minimum_document_frequency);
    Ok(PreparedFeatures {
        labels,
        features,
        clean_documents,
        toxic_documents,
    })
}

fn fit_logistic(
    prepared: PreparedFeatures,
    options: LogisticOptions,
) -> Result<impl LibLinearModel, CompileError> {
    let PreparedFeatures {
        labels,
        mut features,
        clean_documents,
        toxic_documents,
    } = prepared;
    ensure_linear_dimension(&mut features);
    let input = TrainingInput::from_sparse_features(labels, features)
        .map_err(|error| CompileError::LinearTraining(error.to_string()))?;
    let mut builder = liblinear::Builder::new();
    builder.problem().input_data(input).bias(1.0);
    builder
        .parameters()
        .solver_type(SolverType::L2R_LR)
        .stopping_criterion(0.0001)
        .constraints_violation_cost(options.cost);
    if options.class_weighted {
        let total_documents = f64::from(clean_documents) + f64::from(toxic_documents);
        builder
            .parameters()
            .cost_penalty_labels(vec![-1, 1])
            .cost_penalty_weights(vec![
                total_documents / (2.0 * f64::from(clean_documents)),
                total_documents / (2.0 * f64::from(toxic_documents)),
            ]);
    }
    SILENCE_LIBLINEAR.call_once(|| liblinear::toggle_liblinear_stdout_output(false));
    builder
        .build_model()
        .map_err(|error| CompileError::LinearTraining(error.to_string()))
}

fn train_naive_bayes_logistic(
    rows: TrainingRows<'_>,
    cost: f64,
    interpolation: f64,
) -> Result<TrainedWeights, CompileError> {
    validate_logistic_cost(cost)?;
    if !interpolation.is_finite() || !(0.0..=1.0).contains(&interpolation) {
        return Err(CompileError::InvalidInterpolation);
    }
    let mut prepared = prepare_linear_features(rows, MIN_DOCUMENT_FREQUENCY)?;
    let ratios = naive_bayes_ratios(&prepared)?;
    for document in &mut prepared.features {
        for (index, value) in document {
            *value = ratios.values[*index as usize - 1];
        }
    }
    let model = fit_logistic(
        prepared,
        LogisticOptions {
            cost,
            class_weighted: false,
            minimum_document_frequency: MIN_DOCUMENT_FREQUENCY,
        },
    )?;
    quantize_reweighted_model(&model, &ratios, interpolation)
}

/// Binarized log-count ratios with alpha=1, over the retained development vocabulary.
/// Wang and Manning (2012), equation 2: https://aclanthology.org/P12-2018/
fn naive_bayes_ratios(prepared: &PreparedFeatures) -> Result<NaiveBayesRatios, CompileError> {
    let (clean, toxic) = feature_class_counts(prepared);
    let active_bins = (0..BIN_COUNT)
        .filter(|&bin| clean[bin].saturating_add(toxic[bin]) >= MIN_DOCUMENT_FREQUENCY)
        .collect::<Vec<_>>();
    if active_bins.is_empty() {
        return Err(CompileError::LinearTraining(
            "no feature survives the minimum document frequency".to_owned(),
        ));
    }
    let clean_sum = active_bins
        .iter()
        .map(|&bin| 1.0 + f64::from(clean[bin]))
        .sum::<f64>();
    let toxic_sum = active_bins
        .iter()
        .map(|&bin| 1.0 + f64::from(toxic[bin]))
        .sum::<f64>();
    let mut values = vec![0.0; BIN_COUNT];
    for &bin in &active_bins {
        let positive = (1.0 + f64::from(toxic[bin])) / toxic_sum;
        let negative = (1.0 + f64::from(clean[bin])) / clean_sum;
        values[bin] = (positive / negative).ln();
    }
    Ok(NaiveBayesRatios {
        values,
        active_bins,
    })
}

fn feature_class_counts(prepared: &PreparedFeatures) -> (Vec<u32>, Vec<u32>) {
    let mut clean = vec![0_u32; BIN_COUNT];
    let mut toxic = vec![0_u32; BIN_COUNT];
    for (label, document) in prepared.labels.iter().zip(&prepared.features) {
        let counts = if *label > 0.0 { &mut toxic } else { &mut clean };
        for &(index, _) in document {
            let bin = index as usize - 1;
            counts[bin] = counts[bin].saturating_add(1);
        }
    }
    (clean, toxic)
}

fn filter_rare_features(features: &mut [Vec<(u32, f64)>], minimum_document_frequency: u32) {
    if minimum_document_frequency <= 1 {
        return;
    }
    let mut frequencies = vec![0_u32; BIN_COUNT + 1];
    for document in features.iter() {
        for &(index, _) in document {
            frequencies[index as usize] += 1;
        }
    }
    for document in features {
        document.retain(|&(index, _)| frequencies[index as usize] >= minimum_document_frequency);
    }
}

fn ensure_linear_dimension(features: &mut [Vec<(u32, f64)>]) {
    let maximum_index = u32::try_from(BIN_COUNT).expect("the bin count fits in u32");
    if let Some(first) = features.first_mut() {
        if first.last().map(|entry| entry.0) != Some(maximum_index) {
            first.push((maximum_index, 0.0));
        }
    }
}

fn quantize_linear_model(model: &impl LibLinearModel) -> Result<TrainedWeights, CompileError> {
    let direction = model_orientation(model)?;
    let weights = (1..=BIN_COUNT)
        .map(|index| {
            let feature_index = i32::try_from(index).expect("the feature index fits in i32");
            quantize_i16(direction * model.feature_coefficient(feature_index, 0))
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok(TrainedWeights {
        bias: quantize_i32(direction * model.label_bias(0)),
        weights,
    })
}

fn model_orientation(model: &impl LibLinearModel) -> Result<f64, CompileError> {
    if model.num_features() != BIN_COUNT {
        return Err(CompileError::LinearTraining(format!(
            "expected {BIN_COUNT} features, got {}",
            model.num_features()
        )));
    }
    match model.labels().as_slice() {
        [1, -1] => Ok(1.0),
        [-1, 1] => Ok(-1.0),
        labels => Err(CompileError::LinearTraining(format!(
            "unexpected labels {labels:?}"
        ))),
    }
}

/// Folds equation 4's interpolated coefficients into binary-feature runtime weights.
fn quantize_reweighted_model(
    model: &impl LibLinearModel,
    ratios: &NaiveBayesRatios,
    interpolation: f64,
) -> Result<TrainedWeights, CompileError> {
    let direction = model_orientation(model)?;
    let coefficient = |bin: usize| {
        let index = i32::try_from(bin + 1).expect("the feature index fits in i32");
        direction * model.feature_coefficient(index, 0)
    };
    let mean_magnitude = ratios
        .active_bins
        .iter()
        .map(|&bin| coefficient(bin).abs())
        .sum::<f64>()
        / ratios.active_bins.len() as f64;
    let mut weights = vec![0_i16; BIN_COUNT];
    for &bin in &ratios.active_bins {
        let blended = (1.0 - interpolation) * mean_magnitude + interpolation * coefficient(bin);
        weights[bin] = quantize_i16(blended * ratios.values[bin]);
    }
    Ok(TrainedWeights {
        bias: quantize_i32(direction * model.label_bias(0)),
        weights: weights.into_boxed_slice(),
    })
}

pub fn compile_language(request: &CompileRequest) -> Result<CompiledLanguage, CompileError> {
    compile_language_with_learner(request, default_learner(request.language))
}

/// Compiles a learner experiment with the language's existing runtime profiles and gates.
///
/// # Errors
///
/// Returns an error for invalid inputs, invalid logistic cost, or failed calibration gates.
pub fn compile_language_with_learner(
    request: &CompileRequest,
    learner: Learner,
) -> Result<CompiledLanguage, CompileError> {
    if request.rule_channel.language() != request.language {
        return Err(CompileError::RuleChannelLanguageMismatch {
            expected: request.language,
            actual: request.rule_channel.language(),
        });
    }
    validate_compile_split("development", request.language, &request.development)?;
    validate_compile_split("validation", request.language, &request.validation)?;

    let (feature_profile, normalization_profile, feature_schema) = request.language.profiles();
    let development = model_rows(request, &request.development);
    let validation_model = model_rows(request, &request.validation);
    let clean_controls_model = request
        .clean_controls
        .iter()
        .map(|text| model_text(request, text))
        .collect::<Vec<_>>();
    let trained = train_weights_with_learner(
        TrainingRows {
            profile: feature_profile,
            normalization: normalization_profile,
            development: &development,
        },
        learner,
    )?;
    let minimum_boundary = clean_control_boundary(
        request.language,
        &trained,
        feature_profile,
        normalization_profile,
        &request.rule_channel,
        &request.clean_controls,
        &clean_controls_model,
    )?;
    let mut calibration_rows = Vec::with_capacity(request.validation.len());
    let mut raw_scores = Vec::with_capacity(request.validation.len());
    let mut rule_results = Vec::with_capacity(request.validation.len());
    for (row, model_row) in request.validation.iter().zip(&validation_model) {
        let raw_score = trained_raw_score(
            &trained,
            feature_profile,
            normalization_profile,
            model_row,
            "validation",
        )?;
        let rule_outcome = request
            .rule_channel
            .analyze(&row.text, ReplyTarget::Unknown);
        calibration_rows.push(CalibrationRow {
            label: row.label,
            sparse_raw_score: raw_score,
            rule_should_nudge: rule_outcome.should_nudge,
            suppress_sparse: rule_outcome.suppresses_sparse_channel(),
        });
        raw_scores.push(raw_score);
        rule_results.push((
            rule_outcome.should_nudge,
            rule_outcome.suppresses_sparse_channel(),
        ));
    }

    let calibration = calibrate_at_or_above(request.language, &calibration_rows, minimum_boundary)?;
    let score_scale = validation_score_scale(&raw_scores, calibration.boundary)?;
    let artifact = encode_sparse(&SparseInput {
        language: request.language,
        feature_profile,
        normalization_profile,
        feature_schema,
        bias: trained.bias,
        decision_boundary: calibration.boundary,
        score_scale,
        max_false_warning_basis_points: FALSE_WARNING_LIMIT_BASIS_POINTS,
        weights: &trained.weights,
    })?;
    let parsed = SparseModel::from_bytes(&artifact)?;
    let calibrated_predictions = calibration_rows
        .iter()
        .map(|row| {
            row.rule_should_nudge
                || (!row.suppress_sparse && row.sparse_raw_score >= calibration.boundary)
        })
        .collect::<Vec<_>>();
    let validation_predictions = validation_model
        .iter()
        .zip(&rule_results)
        .map(|(row, (rule_should_nudge, suppress_sparse))| {
            *rule_should_nudge
                || (!suppress_sparse && parsed.raw_score(&row.text) >= parsed.raw_boundary())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        calibrated_predictions.len(),
        request.validation.len(),
        "calibration must produce one prediction per validation row"
    );
    assert_eq!(
        validation_predictions.len(),
        request.validation.len(),
        "artifact parsing must produce one prediction per validation row"
    );
    for ((row, expected), actual) in request
        .validation
        .iter()
        .zip(&calibrated_predictions)
        .zip(&validation_predictions)
    {
        if actual != expected {
            return Err(CompileError::ArtifactPredictionMismatch {
                source_id: row.source_id.clone(),
            });
        }
    }
    if prediction_matrix(&request.validation, &validation_predictions) != calibration.matrix {
        return Err(CompileError::ArtifactMatrixMismatch(request.language));
    }

    Ok(CompiledLanguage {
        artifact,
        calibration,
        score_scale,
        validation_predictions,
    })
}

fn clean_control_boundary(
    language: Language,
    trained: &TrainedWeights,
    feature_profile: FeatureProfile,
    normalization_profile: NormalizationProfile,
    rule_channel: &RuleChannel,
    clean_controls: &[String],
    clean_controls_model: &[String],
) -> Result<i32, CompileError> {
    let mut minimum_boundary = i32::MIN;
    for (index, (text, model_text)) in clean_controls.iter().zip(clean_controls_model).enumerate() {
        let source_id = format!("clean-control/{index}");
        let outcome = rule_channel.analyze(text, ReplyTarget::Unknown);
        if outcome.should_nudge {
            return Err(CompileError::CleanControlRuleNudge {
                language,
                source_id,
            });
        }
        if outcome.suppresses_sparse_channel() {
            continue;
        }
        let row = PreparedRow {
            detector_language: language,
            label: EvalLabel::Clean,
            source_id,
            text: model_text.clone(),
        };
        let score = trained_raw_score(
            trained,
            feature_profile,
            normalization_profile,
            &row,
            "clean-control",
        )?;
        minimum_boundary = minimum_boundary.max(score.saturating_add(1));
    }
    Ok(minimum_boundary)
}

/// The text the sparse model scores: the row text, plus lexicon markers for languages that use them.
fn model_text(request: &CompileRequest, text: &str) -> String {
    if !uses_lexicon_features(request.language) {
        return text.to_owned();
    }
    match request.rule_channel.lexicon() {
        Some(lexicon) => lexicon_marked_text(text, &lexicon.check(text).matches),
        None => text.to_owned(),
    }
}

fn model_rows(request: &CompileRequest, rows: &[PreparedRow]) -> Vec<PreparedRow> {
    rows.iter()
        .map(|row| PreparedRow {
            detector_language: row.detector_language,
            label: row.label,
            source_id: row.source_id.clone(),
            text: model_text(request, &row.text),
        })
        .collect()
}

fn validate_compile_split(
    split: &'static str,
    language: Language,
    rows: &[PreparedRow],
) -> Result<(), CompileError> {
    if rows.is_empty() {
        return Err(CompileError::EmptySplit { split });
    }
    let mut has_clean = false;
    let mut has_toxic = false;
    for row in rows {
        if row.detector_language != language {
            return Err(CompileError::LanguageMismatch {
                expected: language,
                actual: row.detector_language,
                split,
                source_id: row.source_id.clone(),
            });
        }
        match row.label {
            EvalLabel::Clean => has_clean = true,
            EvalLabel::Toxic => has_toxic = true,
        }
    }
    if !has_clean {
        return Err(CompileError::MissingClass {
            language,
            split,
            label: "clean",
        });
    }
    if !has_toxic {
        return Err(CompileError::MissingClass {
            language,
            split,
            label: "toxic",
        });
    }
    Ok(())
}

fn trained_raw_score(
    trained: &TrainedWeights,
    feature_profile: FeatureProfile,
    normalization_profile: NormalizationProfile,
    row: &PreparedRow,
    split: &'static str,
) -> Result<i32, CompileError> {
    let bins = extract_feature_bins(feature_profile, normalization_profile, &row.text).map_err(
        |source| CompileError::FeatureExtraction {
            split,
            source_id: row.source_id.clone(),
            source,
        },
    )?;
    let raw = bins.iter().fold(i64::from(trained.bias), |sum, bin| {
        sum + i64::from(trained.weights[*bin])
    });
    Ok(
        i32::try_from(raw.clamp(i64::from(i32::MIN), i64::from(i32::MAX)))
            .expect("clamped raw score fits in i32"),
    )
}

fn prediction_matrix(rows: &[PreparedRow], predictions: &[bool]) -> ConfusionMatrix {
    let mut matrix = ConfusionMatrix::default();
    for (row, predicted) in rows.iter().zip(predictions) {
        match (row.label, *predicted) {
            (EvalLabel::Toxic, true) => matrix.true_positive += 1,
            (EvalLabel::Clean, false) => matrix.true_negative += 1,
            (EvalLabel::Clean, true) => matrix.false_positive += 1,
            (EvalLabel::Toxic, false) => matrix.false_negative += 1,
        }
    }
    matrix
}

pub fn validation_score_scale(raw_scores: &[i32], boundary: i32) -> Result<u32, CompileError> {
    if raw_scores.is_empty() {
        return Err(CompileError::EmptyScoreScaleInput);
    }
    let mut values = raw_scores.to_vec();
    values.sort_unstable();
    let lower = values[(values.len() - 1) / 10];
    let upper = values[(values.len() - 1) * 9 / 10];
    let lower_distance = (i64::from(boundary) - i64::from(lower)).unsigned_abs();
    let upper_distance = (i64::from(upper) - i64::from(boundary)).unsigned_abs();
    Ok(lower_distance
        .max(upper_distance)
        .clamp(1, u64::from(u32::MAX)) as u32)
}

fn quantize_log_odds(
    clean: &[u32],
    toxic: &[u32],
    clean_documents: u32,
    toxic_documents: u32,
) -> TrainedWeights {
    let mut bias = (f64::from(toxic_documents) / f64::from(clean_documents)).ln();
    let mut weights = vec![0_i16; BIN_COUNT];
    for (bin, (&clean_count, &toxic_count)) in clean.iter().zip(toxic).enumerate() {
        if clean_count.saturating_add(toxic_count) < MIN_DOCUMENT_FREQUENCY {
            continue;
        }
        let clean_probability = (f64::from(clean_count) + 1.0) / (f64::from(clean_documents) + 2.0);
        let toxic_probability = (f64::from(toxic_count) + 1.0) / (f64::from(toxic_documents) + 2.0);
        bias += (1.0 - toxic_probability).ln() - (1.0 - clean_probability).ln();
        weights[bin] = quantize_i16(logit(toxic_probability) - logit(clean_probability));
    }
    TrainedWeights {
        bias: quantize_i32(bias),
        weights: weights.into_boxed_slice(),
    }
}

fn logit(probability: f64) -> f64 {
    (probability / (1.0 - probability)).ln()
}

fn quantize_i16(value: f64) -> i16 {
    (value * f64::from(WEIGHT_SCALE))
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}

fn quantize_i32(value: f64) -> i32 {
    (value * f64::from(WEIGHT_SCALE))
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

#[cfg(test)]
mod tests {
    use super::{quantize_i16, quantize_i32};

    #[test]
    fn weight_quantization_clamps_to_i16() {
        assert_eq!(quantize_i16(f64::INFINITY), i16::MAX);
        assert_eq!(quantize_i16(f64::NEG_INFINITY), i16::MIN);
    }

    #[test]
    fn bias_quantization_clamps_to_i32() {
        assert_eq!(quantize_i32(f64::INFINITY), i32::MAX);
        assert_eq!(quantize_i32(f64::NEG_INFINITY), i32::MIN);
    }
}
