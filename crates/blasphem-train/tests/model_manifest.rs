use std::{fs, path::Path};

use blasphem::{
    ConfusionMatrix, FeatureProfile, FeatureSchema, Language, Metrics, NormalizationProfile,
    SparseV2Input, arabic_hindi_rules, canonical_rule_identity, cjk_rules, encode_sparse_v2,
    word_rules,
};
use blasphem_train::{
    calibration::CalibrationResult,
    calibration::gates,
    compiler::CompiledLanguage,
    datasets::{DatasetId, PreparedCounts},
    evidence::Sha256Digest,
    model_manifest::{
        DatasetInput, MODEL_MANIFEST_SCHEMA_VERSION, ManifestInputs, ModelManifest,
        ModelManifestEntry, ModelSetError, artifact_relative_path, build_manifest_entry,
        parse_model_manifest, validate_model_set,
    },
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn model_manifest_rejects_an_unknown_schema() {
    let input = br#"{"schema_version":3,"entries":[]}"#;

    let error = parse_model_manifest(input.as_slice()).expect_err("unknown schema");

    assert!(matches!(
        error,
        ModelSetError::InvalidModelManifestSchema {
            expected: MODEL_MANIFEST_SCHEMA_VERSION,
            actual: 3,
        }
    ));
}

#[test]
fn model_manifest_rejects_an_unknown_root_field() {
    let input = br#"{"schema_version":1,"entries":[],"extra":true}"#;

    let error = parse_model_manifest(input.as_slice()).expect_err("unknown field");

    assert!(matches!(error, ModelSetError::ModelManifestJson(_)));
}

#[test]
fn model_manifest_rejects_unknown_nested_metric_fields() {
    for field in ["validation", "validation_metrics", "validation_gates"] {
        let mut value = serde_json::to_value(complete_manifest_stub()).expect("serialize fixture");
        value["entries"][0][field]
            .as_object_mut()
            .expect("nested manifest object")
            .insert("extra".to_owned(), serde_json::Value::Bool(true));
        let bytes = serde_json::to_vec(&value).expect("serialize changed fixture");

        let error = parse_model_manifest(bytes.as_slice()).expect_err("unknown nested field");

        assert!(
            matches!(error, ModelSetError::ModelManifestJson(_)),
            "field={field} error={error}"
        );
    }
}

#[test]
fn validation_metrics_round_trip_without_numeric_drift() {
    let metrics = ConfusionMatrix {
        true_positive: 329,
        true_negative: 368,
        false_positive: 11,
        false_negative: 38,
    }
    .metrics();
    let bytes = serde_json::to_vec_pretty(&metrics).expect("serialize metrics");
    let parsed: Metrics = serde_json::from_slice(&bytes).expect("parse metrics");

    assert_eq!(parsed, metrics);
}

#[test]
fn artifact_paths_cover_every_language_in_runtime_order() {
    let expected = [
        "en-sparse-v2.bin",
        "zh-sparse-v2.bin",
        "es-sparse-v2.bin",
        "ar-sparse-v2.bin",
        "id-sparse-v2.bin",
        "pt-sparse-v2.bin",
        "fr-sparse-v2.bin",
        "hi-sparse-v2.bin",
        "ru-sparse-v2.bin",
        "ja-sparse-v2.bin",
        "de-sparse-v2.bin",
        "tr-sparse-v2.bin",
        "vi-sparse-v2.bin",
        "ko-sparse-v2.bin",
        "it-sparse-v2.bin",
    ];

    let actual = Language::ALL.map(artifact_relative_path);

    assert_eq!(actual, expected);
}

#[test]
fn manifest_entry_derives_artifact_and_validation_metadata() {
    let weights = vec![0_i16; 65_536];
    let artifact = encode_sparse_v2(&SparseV2Input {
        language: Language::En,
        feature_profile: FeatureProfile::WordChar35V2,
        normalization_profile: NormalizationProfile::GenericV2,
        feature_schema: FeatureSchema::SparseV2,
        bias: 0,
        decision_boundary: 5,
        score_scale: 7,
        max_false_warning_basis_points: 300,
        weights: &weights,
    })
    .expect("fixture artifact");
    let validation = ConfusionMatrix {
        true_positive: 9,
        true_negative: 300,
        false_positive: 1,
        false_negative: 0,
    };
    let compiled = CompiledLanguage {
        artifact,
        calibration: CalibrationResult {
            language: Language::En,
            boundary: 5,
            matrix: validation,
        },
        score_scale: 7,
        validation_predictions: vec![true; 310],
    };
    let inputs = ManifestInputs {
        dataset_inputs: vec![
            DatasetInput {
                dataset: DatasetId::TextDetox,
                source_file_id: "z-source".to_owned(),
                revision: None,
                file_sha256: digest(),
            },
            DatasetInput {
                dataset: DatasetId::TextDetox,
                source_file_id: "a-source".to_owned(),
                revision: Some("revision".to_owned()),
                file_sha256: digest(),
            },
        ],
        prepared_counts: PreparedCounts {
            development: 4,
            validation: 310,
            test: 20,
            duplicates: 2,
            conflicts: 3,
            excluded: 5,
        },
        rule_pack_version: 1,
        rule_pack_sha256: digest(),
        lexicon_sha256: Some(digest()),
        clean_control_rows: 16,
        clean_control_sha256: Some(digest()),
    };

    let entry = build_manifest_entry(&compiled, inputs).expect("manifest entry");

    assert_eq!(entry.language, Language::En);
    assert_eq!(entry.artifact_relative_path, "en-sparse-v2.bin");
    assert_eq!(entry.dataset_inputs[0].source_file_id, "a-source");
    assert_eq!(entry.feature_profile, FeatureProfile::WordChar35V2);
    assert_eq!(entry.boundary, 5);
    assert_eq!(entry.score_scale, 7);
    assert_eq!(entry.validation, validation);
    assert_eq!(entry.validation_metrics, validation.metrics());
    assert!(entry.validation_gates.expect("validation gates").passed());
    assert_eq!(entry.artifact_bytes, 131_112);
    assert_ne!(entry.artifact_sha256, digest());
}

fn rule_version(language: Language) -> u16 {
    if language == Language::Es {
        1
    } else {
        word_rules(language)
            .or_else(|| arabic_hindi_rules(language))
            .or_else(|| cjk_rules(language))
            .expect("language rule pack")
            .version
    }
}

#[test]
fn model_set_rejects_entries_outside_runtime_order() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries.swap(0, 1);

    let error = validate_model_set(directory.path(), &manifest).expect_err("wrong entry order");

    assert!(matches!(error, ModelSetError::ManifestEntryOrder));
}

#[test]
fn model_set_rejects_a_duplicate_entry() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[1] = manifest.entries[0].clone();

    let error = validate_model_set(directory.path(), &manifest).expect_err("duplicate entry");

    assert!(matches!(
        error,
        ModelSetError::DuplicateManifestEntry(Language::En)
    ));
}

#[test]
fn model_set_rejects_an_invalid_artifact_path() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].artifact_relative_path = "nested/en-sparse-v2.bin".to_owned();

    let error = validate_model_set(directory.path(), &manifest).expect_err("invalid path");

    assert!(matches!(
        error,
        ModelSetError::InvalidArtifactPath(Language::En)
    ));
}

#[test]
fn model_set_rejects_a_wrong_rule_pack_version() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].rule_pack_version += 1;

    let error = validate_model_set(directory.path(), &manifest).expect_err("rule pack version");

    assert!(matches!(
        error,
        ModelSetError::RulePackMetadataMismatch(Language::En)
    ));
}

#[test]
fn model_set_rejects_a_wrong_rule_pack_digest() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].rule_pack_sha256 = digest();

    let error = validate_model_set(directory.path(), &manifest).expect_err("rule pack digest");

    assert!(matches!(
        error,
        ModelSetError::RulePackMetadataMismatch(Language::En)
    ));
}

#[test]
fn model_set_requires_a_lexicon_digest() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].lexicon_sha256 = None;

    let error = validate_model_set(directory.path(), &manifest).expect_err("missing Lexicon");

    assert!(matches!(
        error,
        ModelSetError::MissingLexiconDigest(Language::En)
    ));
}

#[test]
fn model_set_rejects_a_changed_spanish_lexicon_digest() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = write_complete_model_set(directory.path());
    manifest.entries[Language::Es.index()].lexicon_sha256 = Some(digest());

    let error = validate_model_set(directory.path(), &manifest).expect_err("Spanish Lexicon");

    assert!(matches!(
        error,
        ModelSetError::LexiconDigestMismatch(Language::Es)
    ));
}

#[test]
fn model_set_rejects_noncanonical_dataset_input_order() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].dataset_inputs = vec![dataset_input("z-source"), dataset_input("a-source")];

    let error = validate_model_set(directory.path(), &manifest).expect_err("dataset input order");

    assert!(matches!(
        error,
        ModelSetError::DatasetInputOrder(Language::En)
    ));
}

#[test]
fn model_set_rejects_incomplete_clean_control_metadata() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].clean_control_sha256 = None;

    let error = validate_model_set(directory.path(), &manifest).expect_err("clean controls");

    assert!(matches!(
        error,
        ModelSetError::CleanControlMetadataMismatch(Language::En)
    ));
}

#[test]
fn model_set_rejects_a_validation_matrix_total_mismatch() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    manifest.entries[0].validation_rows = 3;

    let error = validate_model_set(directory.path(), &manifest).expect_err("validation total");

    assert!(matches!(
        error,
        ModelSetError::ValidationRowCountMismatch {
            language: Language::En,
            declared: 3,
            actual: 2,
        }
    ));
}

#[test]
fn model_set_rejects_an_overflowing_validation_matrix_total() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    let entry = &mut manifest.entries[0];
    entry.validation_rows = usize::MAX;
    entry.validation = ConfusionMatrix {
        true_positive: u64::MAX,
        true_negative: 1,
        false_positive: 0,
        false_negative: 0,
    };

    let error = validate_model_set(directory.path(), &manifest).expect_err("matrix overflow");

    assert!(matches!(
        error,
        ModelSetError::ValidationMatrixTotalOverflow(Language::En)
    ));
}

#[test]
fn model_set_rejects_a_missing_artifact() {
    let directory = tempdir().expect("temporary directory");
    let manifest = write_complete_model_set(directory.path());
    fs::remove_file(directory.path().join("en-sparse-v2.bin"))
        .expect("remove one fixture artifact");

    let error = validate_model_set(directory.path(), &manifest).expect_err("missing artifact");

    assert!(matches!(
        error,
        ModelSetError::MissingArtifact(Language::En)
    ));
}

#[test]
fn model_set_rejects_an_artifact_digest_mismatch() {
    let directory = tempdir().expect("temporary directory");
    let manifest = write_complete_model_set(directory.path());
    let path = directory.path().join("en-sparse-v2.bin");
    let mut artifact = fs::read(&path).expect("read fixture artifact");
    artifact[40] ^= 1;
    fs::write(&path, artifact).expect("mutate one artifact byte");

    let error =
        validate_model_set(directory.path(), &manifest).expect_err("artifact digest mismatch");

    assert!(matches!(
        error,
        ModelSetError::ArtifactDigestMismatch(Language::En)
    ));
}

#[test]
fn model_set_rejects_changed_header_metadata_after_digest_update() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    let artifact = fixture_artifact(Language::En, 0, 2, 300);
    let entry = &mut manifest.entries[0];
    entry.artifact_bytes = artifact.len();
    entry.artifact_sha256 = sha256(&artifact);
    fs::write(directory.path().join("en-sparse-v2.bin"), artifact).expect("fixture artifact");

    let error = validate_model_set(directory.path(), &manifest).expect_err("changed score scale");

    assert!(matches!(
        error,
        ModelSetError::ArtifactMetadataMismatch(Language::En)
    ));
}

#[test]
fn model_set_rejects_a_nonstandard_false_warning_limit() {
    let directory = tempdir().expect("temporary directory");
    let mut manifest = complete_manifest_stub();
    let artifact = fixture_artifact(Language::En, 0, 1, 301);
    let entry = &mut manifest.entries[0];
    entry.false_warning_limit_basis_points = 301;
    entry.artifact_bytes = artifact.len();
    entry.artifact_sha256 = sha256(&artifact);
    fs::write(directory.path().join("en-sparse-v2.bin"), artifact).expect("fixture artifact");

    let error = validate_model_set(directory.path(), &manifest).expect_err("invalid policy limit");

    assert!(matches!(
        error,
        ModelSetError::ArtifactMetadataMismatch(Language::En)
    ));
}

fn complete_manifest_stub() -> ModelManifest {
    ModelManifest {
        schema_version: MODEL_MANIFEST_SCHEMA_VERSION,
        entries: Language::ALL.map(manifest_entry_stub).into(),
    }
}

fn write_complete_model_set(root: &Path) -> ModelManifest {
    let mut entries = Vec::with_capacity(Language::ALL.len());
    for language in Language::ALL {
        let artifact = fixture_artifact(language, 0, 1, 300);
        fs::write(root.join(artifact_relative_path(language)), &artifact)
            .expect("write fixture artifact");
        let mut entry = manifest_entry_stub(language);
        entry.artifact_bytes = artifact.len();
        entry.artifact_sha256 = sha256(&artifact);
        entries.push(entry);
    }
    let manifest = ModelManifest {
        schema_version: MODEL_MANIFEST_SCHEMA_VERSION,
        entries,
    };
    validate_model_set(root, &manifest).expect("complete valid fixture");
    manifest
}

fn manifest_entry_stub(language: Language) -> ModelManifestEntry {
    let validation = ConfusionMatrix {
        true_positive: 1,
        true_negative: 1,
        false_positive: 0,
        false_negative: 0,
    };
    let (feature_profile, normalization_profile, feature_schema) = language.profiles();
    ModelManifestEntry {
        language,
        artifact_relative_path: artifact_relative_path(language).to_owned(),
        dataset_inputs: Vec::new(),
        feature_profile,
        normalization_profile,
        feature_schema,
        rule_pack_version: rule_version(language),
        rule_pack_sha256: sha256(&canonical_rule_identity(language)),
        lexicon_sha256: Some(lexicon_digest(language)),
        clean_control_rows: 1,
        clean_control_sha256: Some(digest()),
        development_rows: 1,
        validation_rows: 2,
        test_rows: 1,
        duplicate_rows: 0,
        conflict_rows: 0,
        excluded_rows: 0,
        boundary: 0,
        score_scale: 1,
        false_warning_limit_basis_points: 300,
        validation,
        validation_metrics: validation.metrics(),
        validation_gates: Some(gates(validation)),
        artifact_bytes: 1,
        artifact_sha256: digest(),
    }
}

/// Spanish manifest entries pin the frozen Lexicon ES digest.
fn lexicon_digest(language: Language) -> Sha256Digest {
    if language == Language::Es {
        return "7ac642a30c91308b8fd2bfcf75c827238999b776aae502dddf8c3dbb20cde7cc"
            .to_owned()
            .try_into()
            .expect("frozen Spanish Lexicon digest");
    }
    digest()
}

fn digest() -> Sha256Digest {
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        .to_owned()
        .try_into()
        .expect("fixture digest")
}

fn dataset_input(source_file_id: &str) -> DatasetInput {
    DatasetInput {
        dataset: DatasetId::TextDetox,
        source_file_id: source_file_id.to_owned(),
        revision: Some("revision".to_owned()),
        file_sha256: digest(),
    }
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("fixture digest")
}

fn fixture_artifact(
    language: Language,
    boundary: i32,
    score_scale: u32,
    false_warning_limit_basis_points: u16,
) -> Vec<u8> {
    let (feature_profile, normalization_profile, feature_schema) = language.profiles();
    encode_sparse_v2(&SparseV2Input {
        language,
        feature_profile,
        normalization_profile,
        feature_schema,
        bias: 0,
        decision_boundary: boundary,
        score_scale,
        max_false_warning_basis_points: false_warning_limit_basis_points,
        weights: &vec![0; 65_536],
    })
    .expect("fixture artifact")
}
