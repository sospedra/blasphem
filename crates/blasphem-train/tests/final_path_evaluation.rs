use std::{collections::BTreeMap, fs};

use blasphem::{ConfusionMatrix, EvalLabel, Language, NudgeDetector};
use blasphem_train::prepared_input::load_prepared_validation;
use blasphem_train::{
    ControlKind, EventType, EvidenceKind,
    calibration::GateResult,
    datasets::{DatasetSplit, PreparedRow},
    evidence::Sha256Digest,
    evidence::{
        CanonicalEvidenceError, canonical_json_bytes, parse_canonical_json, sha256_digest,
        write_canonical_json,
    },
    load_panel,
    verification::{
        BehaviorCaseResult, BehaviorEvidence, CliSmokeCaseResult, CliSmokeEvidence, CliSmokeSuite,
        EvaluationEvidence, EvidenceStatus, LanguageBehaviorResult, LanguageCliSmokeResult,
        LanguageEvaluation, VerificationError, VerificationMetrics, cli_smoke_cases,
        evaluate_behavior, evaluate_cli_smoke, evaluate_language_validation, evaluate_validation,
        load_evidence_inputs, validate_behavior_provenance, validate_class_counts,
    },
};
use serde::{Deserialize, Serialize};
use tempfile::tempdir;

fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("defined metric");
    let difference = (actual - expected).abs();
    assert!(difference <= 1.0e-12, "actual={actual} expected={expected}");
}

#[test]
fn metrics_and_projected_precision_use_the_documented_formulas() {
    let matrix = ConfusionMatrix {
        true_positive: 90,
        false_positive: 3,
        true_negative: 97,
        false_negative: 10,
    };

    let metrics = VerificationMetrics::from_matrix(matrix);

    assert_close(metrics.false_warning_rate, 3.0 / 100.0);
    assert_close(metrics.precision, 90.0 / 93.0);
    assert_close(metrics.recall, 90.0 / 100.0);
    assert_close(metrics.specificity, 97.0 / 100.0);
    assert_close(metrics.f1, 180.0 / 193.0);
    assert_close(
        metrics.projected_precision_1_percent,
        (0.01 * 0.90) / (0.01 * 0.90 + 0.99 * 0.03),
    );
    assert_close(
        metrics.projected_precision_5_percent,
        (0.05 * 0.90) / (0.05 * 0.90 + 0.95 * 0.03),
    );
}

#[test]
fn no_positive_prediction_keeps_prediction_metrics_undefined() {
    let metrics = VerificationMetrics::from_matrix(ConfusionMatrix {
        true_positive: 0,
        false_positive: 0,
        true_negative: 300,
        false_negative: 300,
    });

    assert_eq!(metrics.precision, None);
    assert_eq!(metrics.f1, None);
    assert_eq!(metrics.projected_precision_1_percent, None);
    assert_eq!(metrics.projected_precision_5_percent, None);
    assert_close(metrics.recall, 0.0);
    assert_close(metrics.false_warning_rate, 0.0);
}

#[test]
fn evaluation_accepts_exactly_300_rows_per_new_language_class() {
    let rows = fixture_rows(Language::En, 300, 300);

    validate_class_counts(Language::En, DatasetSplit::Validation, &rows)
        .expect("the exact class floor");
}

#[test]
fn validation_loader_reads_one_declared_validation_split() {
    let source = project_root().join("data/prepared-v1");
    let directory = tempdir().expect("temporary directory");
    fs::create_dir_all(directory.path().join("EN")).expect("English directory");
    fs::copy(
        source.join("manifest.json"),
        directory.path().join("manifest.json"),
    )
    .expect("prepared manifest");
    fs::copy(
        source.join("EN/validation.tsv"),
        directory.path().join("EN/validation.tsv"),
    )
    .expect("English validation split");

    let input = load_prepared_validation(directory.path(), Language::En)
        .expect("English validation input without development or test files");

    assert_eq!(input.language, Language::En);
    assert_eq!(input.validation.len(), input.counts.validation);
    assert!(
        input
            .validation
            .iter()
            .all(|row| row.detector_language == Language::En)
    );
}

#[test]
fn language_evaluation_calls_the_public_nudge_path_for_each_row() {
    let detector = detector(Language::En);
    let mut rows = (0..300)
        .map(|index| PreparedRow {
            detector_language: Language::En,
            label: EvalLabel::Clean,
            source_id: format!("clean-{index}"),
            text: "Thank you for your help".to_owned(),
        })
        .collect::<Vec<_>>();
    rows.extend((0..300).map(|index| PreparedRow {
        detector_language: Language::En,
        label: EvalLabel::Toxic,
        source_id: format!("toxic-{index}"),
        text: "I will kill you".to_owned(),
    }));

    let result = evaluate_language_validation(&detector, &rows).expect("final-path evaluation");

    assert_eq!(
        result.matrix,
        ConfusionMatrix {
            true_positive: 300,
            true_negative: 300,
            false_positive: 0,
            false_negative: 0,
        }
    );
    assert!(result.gates.expect("new-language gates").passed());
}

#[test]
fn language_evaluation_rejects_a_row_for_another_language() {
    let detector = detector(Language::En);
    let mut rows = fixture_rows(Language::En, 300, 300);
    rows[0].detector_language = Language::Fr;

    let error = evaluate_language_validation(&detector, &rows).expect_err("wrong-language row");

    assert!(matches!(
        error,
        VerificationError::RowLanguageMismatch {
            expected: Language::En,
            actual: Language::Fr,
            ..
        }
    ));
}

#[test]
fn validation_evidence_matches_each_canonical_manifest_matrix() {
    let project = project_root();
    let model_path = project.join("resources/models/multilingual-v2/manifest.json");
    let inputs = load_evidence_inputs(&project.join("data/prepared-v1"), &model_path)
        .expect("canonical inputs");

    let evidence = evaluate_validation(
        &project.join("data/prepared-v1"),
        &model_path,
        &project.join("data/raw-v1/hurtlex"),
    )
    .expect("final-path validation");

    for entry in inputs
        .model_manifest
        .entries
        .iter()
        .filter(|entry| entry.language != Language::Es)
    {
        let actual = &evidence.languages[entry.language.code()];
        assert_eq!(actual.matrix, entry.validation, "{} matrix", entry.language);
        assert!(
            actual.gates.expect("new-language gate").passed(),
            "{} gates",
            entry.language,
        );
    }
}

#[test]
fn behavior_evidence_scores_all_360_cases_through_the_public_path() {
    let project = project_root();

    let evidence = evaluate_behavior(
        &project.join("tests/fixtures/behavior"),
        &project.join("data/prepared-v1"),
        &project.join("resources/models/multilingual-v2/manifest.json"),
        &project.join("data/raw-v1/hurtlex"),
    )
    .expect("final-path behavior evidence");

    let failures = evidence
        .languages
        .values()
        .flat_map(|language| language.cases.iter())
        .filter(|case| !case.passed)
        .map(|case| {
            format!(
                "{} expected={} actual={} text={:?}",
                case.case_id, case.expected_nudge, case.actual_nudge, case.text,
            )
        })
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(evidence.languages.len(), 15);
    assert_eq!(
        evidence
            .languages
            .values()
            .map(|language| language.cases.len())
            .sum::<usize>(),
        360,
    );
}

#[test]
fn native_smoke_evidence_scores_all_60_cases_through_the_public_path() {
    let project = project_root();

    let evidence = evaluate_cli_smoke(
        &project.join("resources/models/multilingual-v2/manifest.json"),
        &project.join("data/raw-v1/hurtlex"),
    )
    .expect("native smoke evidence");

    let failures = evidence
        .languages
        .values()
        .flat_map(|language| language.cases.iter())
        .filter(|case| !case.passed)
        .map(|case| case.case_id.as_str())
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "failed cases: {failures:?}");
    assert_eq!(
        evidence
            .languages
            .values()
            .map(|language| language.cases.len())
            .sum::<usize>(),
        60,
    );
}

#[test]
fn published_pretest_reports_are_canonical_and_pass() {
    let reports = project_root().join("reports");
    let validation = parse_canonical_json::<EvaluationEvidence>(
        &fs::read(reports.join("multilingual-validation.json")).expect("validation report"),
    )
    .expect("canonical validation report");
    let behavior = parse_canonical_json::<BehaviorEvidence>(
        &fs::read(reports.join("multilingual-behavior.json")).expect("behavior report"),
    )
    .expect("canonical behavior report");
    let smoke = parse_canonical_json::<CliSmokeEvidence>(
        &fs::read(reports.join("multilingual-cli-smoke.json")).expect("smoke report"),
    )
    .expect("canonical smoke report");

    assert!(validation.passed());
    assert!(behavior.passed());
    assert!(smoke.passed());
    assert_eq!(
        validation.evidence_status,
        EvidenceStatus::CalibrationEvidence
    );
    assert_eq!(
        behavior.evidence_status,
        EvidenceStatus::BehaviorContractEvidence,
    );
    assert_eq!(
        smoke.evidence_status,
        EvidenceStatus::NativeCliSmokeEvidence,
    );
    assert_eq!(
        validation.model_manifest_sha256,
        behavior.model_manifest_sha256
    );
    assert_eq!(
        validation.model_manifest_sha256,
        smoke.model_manifest_sha256
    );
    assert_eq!(
        validation.prepared_manifest_sha256,
        behavior.prepared_manifest_sha256,
    );
}

#[test]
fn evaluation_rejects_small_new_language_splits() {
    let rows = fixture_rows(Language::En, 299, 300);

    let error = validate_class_counts(Language::En, DatasetSplit::Validation, &rows)
        .expect_err("small clean class");

    assert!(matches!(
        error,
        VerificationError::InsufficientClassRows {
            language: Language::En,
            split: DatasetSplit::Validation,
            clean_rows: 299,
            toxic_rows: 300,
        }
    ));
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalFixture {
    schema_version: u16,
    languages: BTreeMap<String, NestedFixture>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NestedFixture {
    zeta: u8,
    alpha: u8,
}

#[test]
fn canonical_json_sorts_nested_members_and_btree_map_keys() {
    let left = CanonicalFixture {
        schema_version: 1,
        languages: BTreeMap::from([
            ("ZH".to_owned(), NestedFixture { zeta: 2, alpha: 1 }),
            ("EN".to_owned(), NestedFixture { zeta: 4, alpha: 3 }),
        ]),
    };
    let right = CanonicalFixture {
        schema_version: 1,
        languages: BTreeMap::from([
            ("EN".to_owned(), NestedFixture { zeta: 4, alpha: 3 }),
            ("ZH".to_owned(), NestedFixture { zeta: 2, alpha: 1 }),
        ]),
    };

    let left_bytes = canonical_json_bytes(&left).expect("canonical JSON");
    let right_bytes = canonical_json_bytes(&right).expect("canonical JSON");

    assert_eq!(left_bytes, right_bytes);
    assert_eq!(
        left_bytes,
        br#"{"languages":{"EN":{"alpha":3,"zeta":4},"ZH":{"alpha":1,"zeta":2}},"schema_version":1}"#,
    );
}

#[test]
fn canonical_reader_rejects_noncanonical_and_duplicate_members() {
    for bytes in [
        br#"{"languages":{},"schema_version":1} "#.as_slice(),
        br#"{"schema_version":1,"languages":{}}"#.as_slice(),
        br#"{"languages":{},"schema_version":1,"schema_version":1}"#.as_slice(),
        br#"{"languages":{"EN":{"zeta":2,"alpha":1}},"schema_version":1}"#.as_slice(),
        br#"{"languages":{"EN":{"alpha":1,"alpha":2,"zeta":3}},"schema_version":1}"#.as_slice(),
    ] {
        let error = parse_canonical_json::<CanonicalFixture>(bytes)
            .expect_err("noncanonical evidence must fail");
        assert!(matches!(
            error,
            CanonicalEvidenceError::Json(_) | CanonicalEvidenceError::NonCanonical
        ));
    }
}

#[test]
fn sha256_digest_rejects_non_lowercase_or_wrong_length_values() {
    for value in [
        "a".repeat(63),
        "A".repeat(64),
        "g".repeat(64),
        "a".repeat(65),
    ] {
        assert!(Sha256Digest::try_from(value).is_err());
    }
}

#[test]
fn evidence_digest_hashes_the_exact_input_bytes() {
    assert_eq!(
        sha256_digest(b"abc").as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
}

#[test]
fn evidence_inputs_hash_exact_typed_manifest_bytes() {
    let project = project_root();
    let model_path = project.join("resources/models/multilingual-v2/manifest.json");
    let prepared_path = project.join("data/prepared-v1/manifest.json");

    let inputs = load_evidence_inputs(&project.join("data/prepared-v1"), &model_path)
        .expect("verified evidence inputs");

    assert_eq!(inputs.model_manifest.entries.len(), 15);
    assert_eq!(
        inputs.model_manifest_sha256,
        sha256_digest(&fs::read(model_path).expect("model manifest bytes")),
    );
    assert_eq!(
        inputs.prepared_manifest_sha256,
        sha256_digest(&fs::read(prepared_path).expect("prepared manifest bytes")),
    );
}

#[test]
fn canonical_writer_adds_no_trailing_newline() {
    let directory = tempdir().expect("temporary directory");
    let path = directory.path().join("nested/evidence.json");
    let fixture = CanonicalFixture {
        schema_version: 1,
        languages: BTreeMap::new(),
    };

    write_canonical_json(&path, &fixture).expect("write canonical evidence");

    assert_eq!(
        fs::read(path).expect("read evidence"),
        br#"{"languages":{},"schema_version":1}"#,
    );
}

#[test]
fn validation_evidence_has_all_new_languages_and_an_informational_pool() {
    let evaluations = Language::ALL
        .into_iter()
        .map(|language| {
            LanguageEvaluation::from_matrix(
                language,
                DatasetSplit::Validation,
                ConfusionMatrix {
                    true_positive: 270,
                    false_positive: 10,
                    true_negative: 390,
                    false_negative: 30,
                },
            )
        })
        .collect();

    let evidence = EvaluationEvidence::validation(digest('a'), digest('b'), evaluations)
        .expect("complete validation evidence");

    assert_eq!(
        evidence.evidence_status,
        EvidenceStatus::CalibrationEvidence
    );
    assert_eq!(evidence.languages.len(), 15);
    assert!(evidence.languages.contains_key("ES"));
    assert_eq!(evidence.pooled_matrix.true_positive, 4_050);
    assert_eq!(evidence.pooled_matrix.false_positive, 150);
    assert_eq!(evidence.pooled_matrix.true_negative, 5_850);
    assert_eq!(evidence.pooled_matrix.false_negative, 450);
    assert!(evidence.languages.values().all(|result| result.gates
        == Some(GateResult {
            false_warning_passed: true,
            precision_passed: true,
            has_true_positive: true,
        })));
}

#[test]
fn validation_evidence_rejects_a_missing_language() {
    let evaluations = Language::ALL
        .into_iter()
        .filter(|language| !matches!(language, Language::Es | Language::Ko))
        .map(|language| {
            LanguageEvaluation::from_matrix(
                language,
                DatasetSplit::Validation,
                ConfusionMatrix::default(),
            )
        })
        .collect();

    let error = EvaluationEvidence::validation(digest('a'), digest('b'), evaluations)
        .expect_err("missing Korean evidence");

    assert!(matches!(error, VerificationError::EvaluationLanguageSet));
}

#[test]
fn validation_evidence_rejects_changed_metrics_or_gates() {
    let mut changed_gates = passing_validation_evaluations();
    changed_gates[0].gates = None;
    assert!(
        EvaluationEvidence::validation(digest('a'), digest('b'), changed_gates).is_err(),
        "a new language must have the exact derived gates",
    );

    let mut changed_metrics = passing_validation_evaluations();
    changed_metrics[0].metrics.precision = Some(f64::NAN);
    assert!(
        EvaluationEvidence::validation(digest('a'), digest('b'), changed_metrics).is_err(),
        "evidence must reject a changed or non-finite metric",
    );
}

#[test]
fn undefined_metrics_serialize_as_json_null() {
    let evaluation = LanguageEvaluation::from_matrix(
        Language::En,
        DatasetSplit::Validation,
        ConfusionMatrix {
            true_negative: 300,
            false_negative: 300,
            ..ConfusionMatrix::default()
        },
    );

    let value = serde_json::to_value(evaluation).expect("evaluation JSON");

    assert!(value["metrics"]["precision"].is_null());
    assert!(value["metrics"]["f1"].is_null());
    assert!(value["metrics"]["projected_precision_1_percent"].is_null());
    assert!(value["metrics"]["projected_precision_5_percent"].is_null());
}

#[test]
fn behavior_evidence_keeps_declared_case_provenance() {
    let case = BehaviorCaseResult {
        case_id: "en-t01".to_owned(),
        text: "I will kill you".to_owned(),
        event_type: EventType::Threat,
        pair_id: "en-threat".to_owned(),
        control_kind: ControlKind::None,
        evidence_kind: EvidenceKind::Authored,
        evidence_ref: "authored-en-001".to_owned(),
        expected_nudge: true,
        actual_nudge: true,
        passed: true,
    };
    let evidence = BehaviorEvidence {
        schema_version: 1,
        evidence_status: EvidenceStatus::BehaviorContractEvidence,
        model_manifest_sha256: digest('a'),
        prepared_manifest_sha256: digest('b'),
        languages: BTreeMap::from([(
            "EN".to_owned(),
            LanguageBehaviorResult {
                language: Language::En,
                passed: true,
                cases: vec![case],
            },
        )]),
    };

    let bytes = canonical_json_bytes(&evidence).expect("canonical behavior evidence");
    let parsed = parse_canonical_json::<BehaviorEvidence>(&bytes).expect("typed evidence");

    assert_eq!(parsed, evidence);
}

#[test]
fn native_smoke_evidence_keeps_public_boolean_and_score_fields() {
    let case = CliSmokeCaseResult {
        case_id: "supplied-en-toxic".to_owned(),
        suite: CliSmokeSuite::Supplied,
        language: Language::En,
        text: "I will kill you".to_owned(),
        expected_nudge: true,
        ok: false,
        score: 95,
        threshold: 50,
        should_nudge: true,
        passed: true,
    };
    let evidence = CliSmokeEvidence {
        schema_version: 1,
        evidence_status: EvidenceStatus::NativeCliSmokeEvidence,
        model_manifest_sha256: digest('a'),
        languages: BTreeMap::from([(
            "EN".to_owned(),
            LanguageCliSmokeResult {
                language: Language::En,
                passed: true,
                cases: vec![case],
            },
        )]),
    };

    let bytes = canonical_json_bytes(&evidence).expect("canonical smoke evidence");
    let parsed = parse_canonical_json::<CliSmokeEvidence>(&bytes).expect("typed evidence");

    assert_eq!(parsed, evidence);
}

#[test]
fn dataset_behavior_refs_are_final_audit_only_provenance_rows() {
    let fixture_root = project_root().join("tests/fixtures/behavior");
    let panels = Language::ALL
        .into_iter()
        .map(|language| {
            let rows = load_panel(&fixture_root, language).expect("behavior panel");
            (language, rows)
        })
        .collect::<BTreeMap<_, _>>();

    validate_behavior_provenance(
        &project_root().join("data/prepared-v1/provenance.tsv"),
        &panels,
    )
    .expect("final audit-only provenance");
}

#[test]
fn native_smoke_inputs_cover_two_pairs_per_language() {
    let cases = cli_smoke_cases();
    let identifiers = cases
        .iter()
        .map(|case| case.case_id)
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(cases.len(), 60);
    assert_eq!(identifiers.len(), cases.len());
    for language in Language::ALL {
        let language_cases = cases
            .iter()
            .filter(|case| case.language == language)
            .collect::<Vec<_>>();
        assert_eq!(language_cases.len(), 4, "{} case count", language.code());
        for suite in [CliSmokeSuite::Supplied, CliSmokeSuite::Context] {
            let suite_cases = language_cases
                .iter()
                .filter(|case| case.suite == suite)
                .collect::<Vec<_>>();
            assert_eq!(suite_cases.len(), 2, "{} {suite:?}", language.code());
            assert_eq!(
                suite_cases
                    .iter()
                    .filter(|case| case.expected_nudge)
                    .count(),
                1,
                "{} {suite:?} toxic count",
                language.code(),
            );
        }
    }
}

#[test]
fn behavior_evidence_requires_15_languages_and_360_cases() {
    let results = Language::ALL
        .into_iter()
        .map(|language| {
            let cases = (0..24)
                .map(|index| BehaviorCaseResult {
                    case_id: format!("{}-{index:02}", language.code().to_ascii_lowercase()),
                    text: format!("fixture {index}"),
                    event_type: EventType::None,
                    pair_id: "none".to_owned(),
                    control_kind: ControlKind::Context,
                    evidence_kind: EvidenceKind::Authored,
                    evidence_ref: format!("authored-{}-{index:02}", language.code()),
                    expected_nudge: false,
                    actual_nudge: false,
                    passed: true,
                })
                .collect();
            LanguageBehaviorResult {
                language,
                passed: true,
                cases,
            }
        })
        .collect();

    let evidence = BehaviorEvidence::new(digest('a'), digest('b'), results)
        .expect("complete behavior evidence");

    assert_eq!(evidence.languages.len(), 15);
    assert_eq!(
        evidence
            .languages
            .values()
            .map(|language| language.cases.len())
            .sum::<usize>(),
        360,
    );
}

#[test]
fn native_smoke_evidence_requires_15_languages_and_60_cases() {
    let results = Language::ALL
        .into_iter()
        .map(|language| {
            let cases = cli_smoke_cases()
                .iter()
                .filter(|case| case.language == language)
                .map(|case| CliSmokeCaseResult {
                    case_id: case.case_id.to_owned(),
                    suite: case.suite,
                    language,
                    text: case.text.to_owned(),
                    expected_nudge: case.expected_nudge,
                    ok: !case.expected_nudge,
                    score: if case.expected_nudge { 50 } else { 0 },
                    threshold: 50,
                    should_nudge: case.expected_nudge,
                    passed: true,
                })
                .collect();
            LanguageCliSmokeResult {
                language,
                passed: true,
                cases,
            }
        })
        .collect();

    let evidence =
        CliSmokeEvidence::new(digest('a'), results).expect("complete native smoke evidence");

    assert_eq!(evidence.languages.len(), 15);
    assert_eq!(
        evidence
            .languages
            .values()
            .map(|language| language.cases.len())
            .sum::<usize>(),
        60,
    );
}

#[test]
fn evidence_constructor_rejects_a_false_language_summary() {
    let result = LanguageCliSmokeResult {
        language: Language::En,
        passed: true,
        cases: vec![CliSmokeCaseResult {
            case_id: "bad-summary".to_owned(),
            suite: CliSmokeSuite::Supplied,
            language: Language::En,
            text: "fixture".to_owned(),
            expected_nudge: true,
            ok: true,
            score: 0,
            threshold: 50,
            should_nudge: false,
            passed: false,
        }],
    };

    let error =
        CliSmokeEvidence::new(digest('a'), vec![result]).expect_err("summary does not match cases");

    assert!(matches!(error, VerificationError::EvidenceSummaryMismatch));
}

fn fixture_rows(language: Language, clean: usize, toxic: usize) -> Vec<PreparedRow> {
    let clean_rows = (0..clean).map(|index| PreparedRow {
        detector_language: language,
        label: EvalLabel::Clean,
        source_id: format!("clean-{index}"),
        text: format!("clean fixture {index}"),
    });
    let toxic_rows = (0..toxic).map(|index| PreparedRow {
        detector_language: language,
        label: EvalLabel::Toxic,
        source_id: format!("toxic-{index}"),
        text: format!("toxic fixture {index}"),
    });
    clean_rows.chain(toxic_rows).collect()
}

fn passing_validation_evaluations() -> Vec<LanguageEvaluation> {
    Language::ALL
        .into_iter()
        .filter(|language| *language != Language::Es)
        .map(|language| {
            LanguageEvaluation::from_matrix(
                language,
                DatasetSplit::Validation,
                ConfusionMatrix {
                    true_positive: 270,
                    false_positive: 10,
                    true_negative: 390,
                    false_negative: 30,
                },
            )
        })
        .collect()
}

fn digest(character: char) -> Sha256Digest {
    character
        .to_string()
        .repeat(64)
        .try_into()
        .expect("valid fixture digest")
}

fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("project root")
}

fn detector(language: Language) -> NudgeDetector {
    let path = project_root()
        .join("data/raw-v1/hurtlex")
        .join(language.storage_code())
        .join("1.2")
        .join(format!("hurtlex_{}.tsv", language.storage_code()));
    let bytes = fs::read(path).expect("HurtLex bytes");
    NudgeDetector::from_hurtlex_bytes(language, Some(&bytes)).expect("fixed-language detector")
}
