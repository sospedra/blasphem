use std::{
    fs::{self, File},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use sha2::{Digest, Sha256};
use thiserror::Error;
use blasphem::{
    ConfusionMatrix, EvalLabel, FeatureError, FeatureProfile, Language, NormalizationProfile,
    ReplyTarget, RuleChannel, SparseModel, SparseModelError, SparseV1Input, SparseV2Input,
    canonical_rule_identity, encode_sparse_v1, encode_sparse_v2, extract_feature_bins,
};

use crate::{
    atomic_publish::{AtomicPublishError, atomic_publish_noreplace},
    behavior_panel::load_panel,
    calibration::{CalibrationError, CalibrationResult, CalibrationRow, calibrate_at_or_above},
    datasets::{DatasetId, PreparedRow},
    evidence::Sha256Digest,
    model_manifest::{
        DatasetInput, MODEL_MANIFEST_SCHEMA_VERSION, ManifestInputs, ModelManifest,
        ModelManifestEntry, ModelSetError, artifact_relative_path, build_manifest_entry,
        parse_model_manifest, rule_pack_version, validate_model_set,
    },
    prepared_input::{PreparedLanguageInput, load_prepared_language},
    source_manifest::SourceRecord,
};

const BIN_COUNT: usize = 65_536;
const WEIGHT_SCALE: u16 = 256;
const MIN_DOCUMENT_FREQUENCY: u32 = 2;
const FALSE_WARNING_LIMIT_BASIS_POINTS: u16 = 300;
static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchCompileOptions {
    pub prepared_root: PathBuf,
    pub hurtlex_root: PathBuf,
    pub behavior_root: Option<PathBuf>,
    pub output: PathBuf,
}

struct CompiledModel {
    artifact: Vec<u8>,
    entry: ModelManifestEntry,
}

pub fn compile_model_set(options: &BatchCompileOptions) -> Result<ModelManifest, ModelSetError> {
    reject_existing_output(&options.output)?;
    let mut models = Vec::with_capacity(Language::ALL.len());
    for language in Language::ALL {
        models.push(compile_prepared_language(options, language)?);
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

fn compile_prepared_language(
    options: &BatchCompileOptions,
    language: Language,
) -> Result<CompiledModel, ModelSetError> {
    let PreparedLanguageInput {
        development,
        validation,
        counts,
        sources,
        ..
    } = load_prepared_language(&options.prepared_root, language)?;
    let hurtlex_sources = sources
        .iter()
        .filter(|source| source.dataset == DatasetId::HurtLex)
        .collect::<Vec<_>>();
    if hurtlex_sources.len() != 1 {
        return Err(ModelSetError::HurtlexSourceCount {
            language,
            actual: hurtlex_sources.len(),
        });
    }
    let hurtlex_source = hurtlex_sources[0];
    let hurtlex_bytes = read_hurtlex(options, language, hurtlex_source)?;
    let rule_channel = RuleChannel::from_hurtlex_bytes(language, Some(&hurtlex_bytes))
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
        .filter(|source| source.dataset != DatasetId::HurtLex)
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
            hurtlex_sha256: Some(hurtlex_source.file_sha256.clone()),
            clean_control_rows,
            clean_control_sha256,
        },
    )?;
    Ok(CompiledModel {
        artifact: compiled.artifact,
        entry,
    })
}

fn read_hurtlex(
    options: &BatchCompileOptions,
    language: Language,
    source_record: &SourceRecord,
) -> Result<Vec<u8>, ModelSetError> {
    let declared = Path::new(&source_record.file_path);
    let relative =
        declared
            .strip_prefix("hurtlex")
            .map_err(|_| ModelSetError::UnsafeHurtlexPath {
                language,
                path: source_record.file_path.clone(),
            })?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(ModelSetError::UnsafeHurtlexPath {
            language,
            path: source_record.file_path.clone(),
        });
    }
    let path = options.hurtlex_root.join(relative);
    let bytes = fs::read(&path).map_err(|source| ModelSetError::HurtlexIo {
        language,
        path,
        source,
    })?;
    if sha256_digest(&bytes) != source_record.file_sha256 {
        return Err(ModelSetError::HurtlexDigestMismatch(language));
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
    let manifest_path = staging.join("manifest.json");
    write_staged_file(&manifest_path, &manifest_bytes)?;
    let parsed = parse_model_manifest(
        fs::read(&manifest_path)
            .map_err(|source| ModelSetError::StagingIo {
                path: manifest_path,
                source,
            })?
            .as_slice(),
    )?;
    validate_model_set(&staging, &parsed)?;
    atomic_publish_noreplace(&staging, &options.output)
        .map_err(|error| map_atomic_error(error, &options.output))?;
    drop(guard);
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
        if self.0.exists() {
            let _ = fs::remove_dir_all(&self.0);
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

    let mut clean = vec![0_u32; BIN_COUNT];
    let mut toxic = vec![0_u32; BIN_COUNT];
    let mut clean_documents = 0_u32;
    let mut toxic_documents = 0_u32;
    for row in development {
        if row.detector_language != language {
            return Err(CompileError::LanguageMismatch {
                expected: language,
                actual: row.detector_language,
                split: "development",
                source_id: row.source_id.clone(),
            });
        }
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
    if clean_documents == 0 {
        return Err(CompileError::MissingClass {
            language,
            split: "development",
            label: "clean",
        });
    }
    if toxic_documents == 0 {
        return Err(CompileError::MissingClass {
            language,
            split: "development",
            label: "toxic",
        });
    }

    Ok(quantize_log_odds(
        &clean,
        &toxic,
        clean_documents,
        toxic_documents,
    ))
}

pub fn compile_language(request: &CompileRequest) -> Result<CompiledLanguage, CompileError> {
    if request.rule_channel.language() != request.language {
        return Err(CompileError::RuleChannelLanguageMismatch {
            expected: request.language,
            actual: request.rule_channel.language(),
        });
    }
    validate_compile_split("development", request.language, &request.development)?;
    validate_compile_split("validation", request.language, &request.validation)?;

    let (feature_profile, normalization_profile, feature_schema) = request.language.profiles();
    let trained = train_weights(feature_profile, normalization_profile, &request.development)?;
    let minimum_boundary = clean_control_boundary(
        request.language,
        &trained,
        feature_profile,
        normalization_profile,
        &request.rule_channel,
        &request.clean_controls,
    )?;
    let mut calibration_rows = Vec::with_capacity(request.validation.len());
    let mut raw_scores = Vec::with_capacity(request.validation.len());
    let mut rule_results = Vec::with_capacity(request.validation.len());
    for row in &request.validation {
        let raw_score = trained_raw_score(
            &trained,
            feature_profile,
            normalization_profile,
            row,
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
    let artifact = if request.language == Language::Es {
        encode_sparse_v1(&SparseV1Input {
            bias: trained.bias,
            decision_boundary: calibration.boundary,
            score_scale,
            max_false_warning_basis_points: FALSE_WARNING_LIMIT_BASIS_POINTS,
            weights: &trained.weights,
        })?
    } else {
        encode_sparse_v2(&SparseV2Input {
            language: request.language,
            feature_profile,
            normalization_profile,
            feature_schema,
            bias: trained.bias,
            decision_boundary: calibration.boundary,
            score_scale,
            max_false_warning_basis_points: FALSE_WARNING_LIMIT_BASIS_POINTS,
            weights: &trained.weights,
        })?
    };
    let parsed = SparseModel::from_bytes(&artifact)?;
    let calibrated_predictions = calibration_rows
        .iter()
        .map(|row| {
            row.rule_should_nudge
                || (!row.suppress_sparse && row.sparse_raw_score >= calibration.boundary)
        })
        .collect::<Vec<_>>();
    let validation_predictions = request
        .validation
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
) -> Result<i32, CompileError> {
    let mut minimum_boundary = i32::MIN;
    for (index, text) in clean_controls.iter().enumerate() {
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
            text: text.clone(),
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
