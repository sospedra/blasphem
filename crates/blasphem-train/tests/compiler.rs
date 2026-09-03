use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use blasphem::{
    EvalLabel, FeatureProfile, FeatureSchema, Language, NormalizationProfile, ReplyTarget,
    RuleChannel, SparseModel, SparseV2Input, encode_sparse_v2,
};
use blasphem_train::{
    compiler::{
        BatchCompileOptions, CompileError, CompileRequest, CompiledLanguage, compile_language,
        compile_model_set, train_weights, validation_score_scale,
    },
    corpus::load_corpus_language,
    datasets::{DatasetId, PreparedRow},
    evidence::Sha256Digest,
    model_manifest::{ModelSetError, parse_model_manifest, validate_model_set},
    source_manifest::parse_frozen_source_lock,
    source_role::SourceRole,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

#[test]
fn compile_help_exposes_only_batch_inputs() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["compile", "--help"])
        .output()
        .expect("compile help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(stdout.contains("--corpus-root"));
    assert!(stdout.contains("--source-lock"));
    assert!(stdout.contains("--hurtlex-root"));
    assert!(stdout.contains("--behavior-root"));
    assert!(stdout.contains("--output"));
    assert!(!stdout.contains("--test"));
    assert!(!stdout.contains("--development"));
    assert!(!stdout.contains("--validation"));
}

#[test]
fn compile_rejects_a_test_input_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["compile", "--test", "test.tsv"])
        .output()
        .expect("compile command");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("unexpected argument '--test'"));
}

#[test]
fn training_counts_a_repeated_feature_once_per_document() {
    let development = vec![
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/1", "tox tox tox"),
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/2", "tox"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/1", "abc"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/2", "abc"),
    ];

    let trained = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &development,
    )
    .expect("train weights");

    assert_eq!(trained.bias, 0);
    for bin in [1722, 1731, 8133, 26526, 42498, 44885, 64854] {
        assert_eq!(trained.weights[bin], 562, "bin {bin}");
    }
}

#[test]
fn training_fixture_has_a_stable_version_two_artifact() {
    let development = vec![
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/1", "tox"),
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/2", "tox"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/1", "abc"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/2", "abc"),
    ];
    let trained = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &development,
    )
    .expect("train golden fixture");
    let artifact = encode_sparse_v2(&SparseV2Input {
        language: Language::En,
        feature_profile: FeatureProfile::WordChar35V2,
        normalization_profile: NormalizationProfile::GenericV2,
        feature_schema: FeatureSchema::SparseV2,
        bias: trained.bias,
        decision_boundary: 17,
        score_scale: 90,
        max_false_warning_basis_points: 300,
        weights: &trained.weights,
    })
    .expect("encode golden fixture");

    assert_eq!(trained.bias, 0);
    assert_eq!(trained.weights[1722], 562);
    assert_eq!(
        format!("{:x}", Sha256::digest(&artifact)),
        "afa5399208b5032f3b2621deae8a4f4f341538275b21912537c414ae99cf55b0"
    );
}

#[test]
fn turkish_training_fixture_has_a_stable_balanced_artifact() {
    let development = vec![
        prepared_row(
            Language::Tr,
            EvalLabel::Clean,
            "clean/1",
            "bugün hava güzel",
        ),
        prepared_row(Language::Tr, EvalLabel::Clean, "clean/2", "yarın görüşürüz"),
        prepared_row(Language::Tr, EvalLabel::Clean, "clean/3", "kitabı okudum"),
        prepared_row(
            Language::Tr,
            EvalLabel::Clean,
            "clean/4",
            "yardımın için sağ ol",
        ),
        prepared_row(
            Language::Tr,
            EvalLabel::Clean,
            "clean/5",
            "maç akşam başlıyor",
        ),
        prepared_row(
            Language::Tr,
            EvalLabel::Clean,
            "clean/6",
            "toplantı sona erdi",
        ),
        prepared_row(Language::Tr, EvalLabel::Toxic, "toxic/1", "aptal herif"),
        prepared_row(Language::Tr, EvalLabel::Toxic, "toxic/2", "aptallar"),
    ];
    let trained = train_weights(
        FeatureProfile::TurkishChar35V3,
        NormalizationProfile::TurkishV2,
        &development,
    )
    .expect("train Turkish fixture");
    let artifact = encode_sparse_v2(&SparseV2Input {
        language: Language::Tr,
        feature_profile: FeatureProfile::TurkishChar35V3,
        normalization_profile: NormalizationProfile::TurkishV2,
        feature_schema: FeatureSchema::SparseV2,
        bias: trained.bias,
        decision_boundary: 17,
        score_scale: 90,
        max_false_warning_basis_points: 300,
        weights: &trained.weights,
    })
    .expect("encode Turkish fixture");

    assert_eq!(
        format!("{:x}", Sha256::digest(&artifact)),
        "87d8787e250a802fcace76c720c60824acf11cdfc3e2b1843ee604c33a1f37f9"
    );
}

#[test]
fn training_excludes_document_frequency_one_from_weights_and_bias() {
    let baseline = vec![
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/1", "tox"),
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/2", "tox"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/1", "abc"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/2", "abc"),
    ];
    let mut with_singleton = baseline.clone();
    with_singleton[0].text = "tox singletononly".to_owned();

    let baseline = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &baseline,
    )
    .expect("baseline weights");
    let with_singleton = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &with_singleton,
    )
    .expect("singleton weights");

    assert_eq!(with_singleton, baseline);
}

#[test]
fn training_is_independent_of_development_row_order() {
    let mut forward = vec![
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/1", "tox"),
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/2", "tox"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/1", "abc"),
        prepared_row(Language::En, EvalLabel::Clean, "clean/2", "abc"),
    ];
    let first = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &forward,
    )
    .expect("forward weights");
    forward.reverse();

    let reversed = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &forward,
    )
    .expect("reversed weights");

    assert_eq!(reversed, first);
}

#[test]
fn training_reports_the_missing_language_class() {
    let development = vec![prepared_row(
        Language::Pt,
        EvalLabel::Clean,
        "clean/1",
        "mensagem limpa",
    )];

    let error = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &development,
    )
    .expect_err("missing toxic class");

    assert!(matches!(
        error,
        CompileError::MissingClass {
            language: Language::Pt,
            split: "development",
            label: "toxic",
        }
    ));
}

#[test]
fn training_reports_a_row_language_mismatch_with_its_source() {
    let development = vec![
        prepared_row(Language::En, EvalLabel::Toxic, "toxic/1", "tox"),
        prepared_row(Language::Pt, EvalLabel::Clean, "clean/pt", "limpo"),
    ];

    let error = train_weights(
        FeatureProfile::WordChar35V2,
        NormalizationProfile::GenericV2,
        &development,
    )
    .expect_err("language mismatch");

    assert!(matches!(
        error,
        CompileError::LanguageMismatch {
            expected: Language::En,
            actual: Language::Pt,
            split: "development",
            source_id,
        } if source_id == "clean/pt"
    ));
}

#[test]
fn score_scale_uses_the_exact_asymmetric_percentiles() {
    let scores = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100];

    assert_eq!(validation_score_scale(&scores, 100), Ok(90));
}

#[test]
fn score_scale_is_one_when_every_score_equals_the_boundary() {
    assert_eq!(validation_score_scale(&[7, 7, 7], 7), Ok(1));
}

#[test]
fn score_scale_clamps_the_full_i32_range() {
    let mut scores = vec![i32::MIN; 9];
    scores.extend([i32::MAX; 2]);

    assert_eq!(validation_score_scale(&scores, i32::MIN), Ok(u32::MAX));
}

#[test]
fn score_scale_rejects_an_empty_validation_split() {
    assert_eq!(
        validation_score_scale(&[], 0),
        Err(CompileError::EmptyScoreScaleInput)
    );
}

#[test]
fn version_two_compiler_is_deterministic() {
    let first = compile_language(&fixture_request(Language::En)).expect("first compilation");
    let second = compile_language(&fixture_request(Language::En)).expect("second compilation");

    assert_eq!(first, second);
    assert_eq!(first.validation_predictions.len(), 2);
    assert_eq!(first.artifact.len(), 131_112);
}

#[test]
fn clean_controls_raise_the_sparse_boundary_above_their_raw_scores() {
    let mut request = fixture_request(Language::En);
    request.clean_controls.push("tox".to_owned());

    let compiled = compile_language(&request).expect("guarded compilation");
    let model = SparseModel::from_bytes(&compiled.artifact).expect("compiled model");

    assert!(model.raw_boundary() > model.raw_score("tox"));
}

#[test]
fn clean_controls_must_not_trigger_the_rule_channel() {
    let mut request = fixture_request(Language::En);
    request.clean_controls.push("I will kill you".to_owned());

    let error = compile_language(&request).expect_err("toxic clean control");

    assert!(matches!(
        error,
        CompileError::CleanControlRuleNudge {
            language: Language::En,
            ..
        }
    ));
}

#[test]
fn serialized_version_two_model_matches_final_validation_predictions() {
    let request = fixture_request(Language::En);
    let compiled = compile_language(&request).expect("compile fixture");
    let model = SparseModel::from_bytes(&compiled.artifact).expect("parse compiled artifact");

    assert_eq!(
        compiled.validation_predictions.len(),
        request.validation.len()
    );
    for (row, expected) in request
        .validation
        .iter()
        .zip(&compiled.validation_predictions)
    {
        let rules = request
            .rule_channel
            .analyze(&row.text, ReplyTarget::Unknown);
        let actual = rules.should_nudge
            || (!rules.suppresses_sparse_channel()
                && model.raw_score(&row.text) >= model.raw_boundary());
        assert_eq!(actual, *expected, "{}", row.source_id);
    }
    assert_eq!(model.language(), Language::En);
    assert_eq!(model.score_scale(), compiled.score_scale);
}

#[test]
fn version_two_compiler_reports_a_missing_validation_class() {
    let mut request = fixture_request(Language::En);
    request
        .validation
        .retain(|row| row.label == EvalLabel::Clean);

    let error = compile_language(&request).expect_err("missing toxic validation class");

    assert!(matches!(
        error,
        CompileError::MissingClass {
            language: Language::En,
            split: "validation",
            label: "toxic",
        }
    ));
}

#[test]
fn version_two_compiler_reports_a_validation_language_mismatch() {
    let mut request = fixture_request(Language::En);
    request.validation[1].detector_language = Language::Pt;

    let error = compile_language(&request).expect_err("validation language mismatch");

    assert!(matches!(
        error,
        CompileError::LanguageMismatch {
            expected: Language::En,
            actual: Language::Pt,
            split: "validation",
            source_id,
        } if source_id == "validation/clean"
    ));
}

#[test]
fn version_two_compiler_rejects_a_rule_channel_for_another_language() {
    let mut request = fixture_request(Language::En);
    request.rule_channel =
        RuleChannel::from_hurtlex_bytes(Language::Pt, None).expect("Portuguese rule channel");

    let error = compile_language(&request).expect_err("wrong rule channel");

    assert!(matches!(
        error,
        CompileError::RuleChannelLanguageMismatch {
            expected: Language::En,
            actual: Language::Pt,
        }
    ));
}

fn fixture_request(language: Language) -> CompileRequest {
    CompileRequest {
        language,
        development: vec![
            prepared_row(language, EvalLabel::Toxic, "development/toxic/1", "tox"),
            prepared_row(language, EvalLabel::Toxic, "development/toxic/2", "tox"),
            prepared_row(language, EvalLabel::Clean, "development/clean/1", "abc"),
            prepared_row(language, EvalLabel::Clean, "development/clean/2", "abc"),
        ],
        validation: vec![
            prepared_row(
                language,
                EvalLabel::Toxic,
                "validation/toxic",
                "I will kill you",
            ),
            prepared_row(
                language,
                EvalLabel::Clean,
                "validation/clean",
                "have a nice day",
            ),
        ],
        rule_channel: RuleChannel::from_hurtlex_bytes(language, None)
            .expect("fixture rule channel"),
        clean_controls: Vec::new(),
    }
}

fn prepared_row(language: Language, label: EvalLabel, source_id: &str, text: &str) -> PreparedRow {
    PreparedRow {
        detector_language: language,
        label,
        source_id: source_id.to_owned(),
        text: text.to_owned(),
    }
}

#[test]
fn spanish_compiles_deterministically_from_corpus_input() {
    let directory = tempdir().expect("temporary directory");
    let options = write_batch_fixture(directory.path());

    let first = compile_corpus_language_for(&options, Language::Es);
    let second = compile_corpus_language_for(&options, Language::Es);

    assert_eq!(
        first.artifact, second.artifact,
        "training must be deterministic"
    );
    assert_eq!(&first.artifact[..8], b"TOXSPRS1");
    let model = SparseModel::from_bytes(&first.artifact).expect("parses");
    assert_eq!(model.language(), Language::Es);
    assert_eq!(
        (
            model.feature_profile(),
            model.normalization_profile(),
            model.feature_schema()
        ),
        Language::Es.profiles()
    );
}

fn compile_corpus_language_for(
    options: &BatchCompileOptions,
    language: Language,
) -> CompiledLanguage {
    let lock = parse_frozen_source_lock(
        fs::File::open(&options.source_lock).expect("open the fixture source lock"),
    )
    .expect("parse the fixture source lock");
    let prepared = load_corpus_language(&options.corpus_root, language, &lock)
        .expect("load the corpus language");
    compile_language(&CompileRequest {
        language,
        development: prepared.development,
        validation: prepared.validation,
        rule_channel: RuleChannel::from_hurtlex_bytes(language, None).expect("rule channel"),
        clean_controls: Vec::new(),
    })
    .expect("compile the prepared language")
}

#[test]
fn batch_compiler_publishes_a_complete_model_set_without_test_files() {
    let directory = tempdir().expect("temporary directory");
    let options = write_batch_fixture(directory.path());

    let manifest = compile_model_set(&options).expect("compile model set");

    assert_eq!(manifest.entries.len(), Language::ALL.len());
    assert_eq!(manifest.entries[0].language, Language::En);
    assert_eq!(manifest.entries[2].language, Language::Es);
    assert!(manifest.entries.iter().all(|entry| entry.test_rows == 7));
    assert!(
        manifest
            .entries
            .iter()
            .all(|entry| entry.development_rows == 4 && entry.validation_rows == 2)
    );
    let parsed = parse_model_manifest(
        fs::File::open(options.output.join("manifest.json")).expect("published manifest"),
    )
    .expect("parse published manifest");
    assert_eq!(parsed, manifest);
    validate_model_set(&options.output, &parsed).expect("validate published model set");
    assert_eq!(
        fs::read_dir(&options.output)
            .expect("published directory")
            .count(),
        Language::ALL.len() + 1
    );
}

#[test]
fn model_set_publication_preserves_an_existing_destination() {
    let directory = tempdir().expect("temporary directory");
    let options = write_batch_fixture(directory.path());
    fs::create_dir(&options.output).expect("existing destination");
    fs::write(options.output.join("owner.bin"), [0, 1, 2, 3]).expect("owner bytes");
    fs::write(options.output.join("metadata.json"), b"{\"owner\":true}\n").expect("owner metadata");
    let before = destination_bytes(&options.output);

    let error = compile_model_set(&options).expect_err("existing destination");

    assert!(matches!(
        error,
        ModelSetError::PublicationDestinationExists(ref path) if path == &options.output
    ));
    assert_eq!(destination_bytes(&options.output), before);
}

#[test]
fn batch_compiler_rejects_a_hurtlex_digest_mismatch() {
    let directory = tempdir().expect("temporary directory");
    let options = write_batch_fixture(directory.path());
    fs::write(
        options.hurtlex_root.join("EN/1.2/hurtlex_EN.tsv"),
        b"changed",
    )
    .expect("changed HurtLex file");

    let error = compile_model_set(&options).expect_err("HurtLex digest mismatch");

    assert!(matches!(
        error,
        ModelSetError::HurtlexDigestMismatch(Language::En)
    ));
    assert!(!options.output.exists());
}

#[test]
fn batch_compiler_requires_one_hurtlex_source_per_language() {
    let directory = tempdir().expect("temporary directory");
    let options = write_batch_fixture(directory.path());
    let mut lock: Value =
        serde_json::from_slice(&fs::read(&options.source_lock).expect("fixture source lock"))
            .expect("parse fixture source lock");
    lock["sources"]
        .as_array_mut()
        .expect("source array")
        .retain(|source| source["source_file_id"] != "hurtlex-en");
    fs::write(
        &options.source_lock,
        serde_json::to_vec(&lock).expect("serialize changed lock"),
    )
    .expect("write changed lock");

    let error = compile_model_set(&options).expect_err("missing HurtLex source");

    assert!(matches!(
        error,
        ModelSetError::HurtlexSourceCount {
            language: Language::En,
            actual: 0,
        }
    ));
    assert!(!options.output.exists());
}

fn write_batch_fixture(root: &Path) -> BatchCompileOptions {
    let corpus_root = root.join("corpus");
    let hurtlex_root = root.join("hurtlex");
    let source_lock = root.join("source-lock.json");
    let output = root.join("models");
    fs::create_dir(&corpus_root).expect("corpus root");
    fs::create_dir(&hurtlex_root).expect("HurtLex root");

    let mut sources = Vec::new();
    for language in Language::ALL {
        let code = language.storage_code();
        let dataset_source_id = format!("dataset-{}", code.to_ascii_lowercase());
        let hurtlex_source_id = format!("hurtlex-{}", code.to_ascii_lowercase());
        let hurtlex_relative_path = format!("hurtlex/{code}/1.2/hurtlex_{code}.tsv");
        let hurtlex_bytes = hurtlex_fixture_bytes(language);
        let hurtlex_path = hurtlex_root.join(format!("{code}/1.2/hurtlex_{code}.tsv"));
        fs::create_dir_all(hurtlex_path.parent().expect("HurtLex parent"))
            .expect("HurtLex language directory");
        fs::write(&hurtlex_path, &hurtlex_bytes).expect("HurtLex fixture");

        sources.push(frozen_source(
            language,
            DatasetId::TextDetox,
            &dataset_source_id,
            &format!("datasets/{code}.tsv"),
            sha256_digest(format!("dataset-{code}").as_bytes()),
        ));
        sources.push(frozen_source(
            language,
            DatasetId::HurtLex,
            &hurtlex_source_id,
            &hurtlex_relative_path,
            sha256_digest(&hurtlex_bytes),
        ));

        fs::write(corpus_root.join(format!("{code}.tsv")), corpus_tsv()).expect("corpus fixture");
    }

    write_source_lock(&source_lock, &sources);

    BatchCompileOptions {
        corpus_root,
        source_lock,
        hurtlex_root,
        behavior_root: None,
        output,
    }
}

/// Four development rows, two validation rows, and seven sealed test rows.
fn corpus_tsv() -> Vec<u8> {
    let mut rows = vec![
        ("development", "clean", "cleansignal"),
        ("development", "clean", "cleansignal two"),
        ("development", "toxic", "toxsignal"),
        ("development", "toxic", "toxsignal two"),
        ("validation", "clean", "validation cleansignal"),
        ("validation", "toxic", "validation toxsignal"),
    ];
    for text in TEST_CLEAN.iter().copied() {
        rows.push(("test", "clean", text));
    }
    for text in TEST_TOXIC.iter().copied() {
        rows.push(("test", "toxic", text));
    }

    let mut lines = Vec::with_capacity(rows.len());
    for (split, label, text) in rows {
        lines.push(format!("{split}\t{label}\t{text}"));
    }
    lines.sort();
    let mut value = String::from("split\tlabel\ttext\n");
    for line in lines {
        value.push_str(&line);
        value.push('\n');
    }
    value.into_bytes()
}

const TEST_CLEAN: [&str; 4] = [
    "test cleansignal one",
    "test cleansignal two",
    "test cleansignal three",
    "test cleansignal four",
];

const TEST_TOXIC: [&str; 3] = [
    "test toxsignal one",
    "test toxsignal two",
    "test toxsignal three",
];

fn write_source_lock(path: &Path, sources: &[Value]) {
    let lock = json!({ "schema_version": "source-lock-v1", "sources": sources });
    fs::write(
        path,
        serde_json::to_vec(&lock).expect("serialize source lock"),
    )
    .expect("write source lock");
}

fn frozen_source(
    language: Language,
    dataset: DatasetId,
    source_file_id: &str,
    file_path: &str,
    file_sha256: Sha256Digest,
) -> Value {
    json!({
        "dataset": dataset,
        "detector_language": language.storage_code(),
        "source_role": SourceRole::Baseline,
        "source_file_id": source_file_id,
        "immutable_source_url": format!("https://example.invalid/{source_file_id}"),
        "archive_member": Value::Null,
        "revision": "fixture-v1",
        "file_path": file_path,
        "file_sha256": file_sha256.as_str(),
        "license_id": "CC-BY-4.0",
        "license_url": "https://example.invalid/license",
        "license_year": 2024,
        "citation": "Fixture citation",
        "upstream_lineage": ["https://example.invalid/source"],
        "lineage_status": "resolved",
    })
}

/// Spanish manifest entries pin the frozen HurtLex ES digest, so the fixture uses the real file.
fn hurtlex_fixture_bytes(language: Language) -> Vec<u8> {
    if language == Language::Es {
        return fs::read(project_root().join("data/clean-room-v1/ES.tsv"))
            .expect("Spanish HurtLex data");
    }
    b"id\tpos\tcategory\tstereotype\tlemma\tlevel\n".to_vec()
}

fn sha256_digest(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("fixture digest")
}

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn destination_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root)
        .expect("destination directory")
        .map(|entry| {
            let entry = entry.expect("destination entry");
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).expect("destination bytes");
            (name, bytes)
        })
        .collect()
}
