use std::{fs, path::Path};

use toxbench::{
    BrowserBuildEvidence, LanguageDetection, LanguageIdentifier, LanguageResolution,
    LanguageSource, evaluate_auto_corpus, load_browser_build_evidence, validate_pinned_corpus,
    verify_c_parity_fixture,
};
use toxcheck::Language;

#[derive(Debug)]
struct FixtureIdentifier;

impl LanguageIdentifier for FixtureIdentifier {
    fn identify(&self, text: &str) -> LanguageDetection {
        let resolution = match text {
            "correct" => LanguageResolution::Known(Language::En),
            "misrouted" => LanguageResolution::Known(Language::Es),
            "unsupported-routed" => LanguageResolution::Known(Language::En),
            "unknown" | "unsupported-unknown" => LanguageResolution::Unknown,
            value => panic!("unexpected fixture text: {value}"),
        };
        let reliable = matches!(resolution, LanguageResolution::Known(_));
        LanguageDetection {
            source: LanguageSource::Automatic,
            resolution,
            reliable,
            score: reliable.then_some(0.9),
            feature_count: Some(10),
        }
    }
}

#[test]
fn route_metrics_use_the_required_supported_and_known_route_denominators() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let text_path = temporary.path().join("text.txt");
    let labels_path = temporary.path().join("labels.txt");
    fs::write(
        &text_path,
        b"correct\nunknown\nmisrouted\nunsupported-unknown\nunsupported-routed",
    )
    .expect("text corpus");
    fs::write(&labels_path, b"en\nen\nen\nnl\nnl\n").expect("label corpus");

    let evidence = evaluate_auto_corpus(&text_path, &labels_path, &FixtureIdentifier)
        .expect("corpus evidence");

    assert_eq!(evidence.corpus.rows, 5);
    assert_eq!(evidence.corpus.supported_rows, 3);
    assert_eq!(evidence.corpus.unsupported_rows, 2);
    assert_eq!(evidence.supported.correct, 1);
    assert_eq!(evidence.supported.unknown, 1);
    assert_eq!(evidence.supported.misrouted, 1);
    assert_eq!(evidence.supported.route_accuracy.numerator, 1);
    assert_eq!(evidence.supported.route_accuracy.denominator, 3);
    assert_eq!(evidence.supported.route_accuracy.value, 1.0 / 3.0);
    assert_eq!(evidence.supported.unknown_rate.numerator, 1);
    assert_eq!(evidence.supported.unknown_rate.denominator, 3);
    assert_eq!(evidence.supported.misroute_rate.numerator, 1);
    assert_eq!(evidence.supported.misroute_rate.denominator, 3);
    assert_eq!(evidence.supported.known_route_precision.numerator, 1);
    assert_eq!(evidence.supported.known_route_precision.denominator, 2);
    assert_eq!(evidence.supported.known_route_precision.value, 0.5);
    assert_eq!(evidence.unsupported.rejected_as_unknown, 1);
    assert_eq!(evidence.unsupported.falsely_routed, 1);
    assert_eq!(evidence.unsupported.unsupported_rejection_rate.numerator, 1);
    assert_eq!(
        evidence.unsupported.unsupported_rejection_rate.denominator,
        2
    );
    assert_eq!(evidence.unsupported.unsupported_rejection_rate.value, 0.5);

    let english = evidence.languages.get("EN").expect("English evidence");
    assert_eq!(english.rows, 3);
    assert_eq!(english.correct, 1);
    assert_eq!(english.unknown, 1);
    assert_eq!(english.misrouted, 1);
}

#[test]
fn paired_corpus_accepts_a_text_file_without_a_final_newline() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let text_path = temporary.path().join("text.txt");
    let labels_path = temporary.path().join("labels.txt");
    fs::write(&text_path, b"correct").expect("text corpus");
    fs::write(&labels_path, b"en\n").expect("label corpus");

    let evidence = evaluate_auto_corpus(&text_path, &labels_path, &FixtureIdentifier)
        .expect("corpus evidence");

    assert_eq!(evidence.corpus.rows, 1);
    assert_eq!(evidence.supported.correct, 1);
}

#[test]
fn paired_corpus_rejects_unequal_file_termination() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let text_path = temporary.path().join("text.txt");
    let labels_path = temporary.path().join("labels.txt");
    fs::write(&text_path, b"correct\nunknown").expect("text corpus");
    fs::write(&labels_path, b"en\n").expect("label corpus");

    let error = evaluate_auto_corpus(&text_path, &labels_path, &FixtureIdentifier)
        .expect_err("unequal corpus must fail");

    assert!(error.to_string().contains("unequal row counts"));
}

#[test]
fn canonical_json_has_sorted_keys_and_no_final_newline() {
    let bytes = toxbench::canonical_json_bytes(&serde_json::json!({"z": 1, "a": 2}))
        .expect("canonical JSON");

    assert_eq!(bytes, br#"{"a":2,"z":1}"#);
    assert_ne!(bytes.last(), Some(&b'\n'));
}

#[test]
fn browser_size_evidence_requires_full_and_explicit_only_builds() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("browser.json");
    fs::write(
        &path,
        br#"{
          "browser_builds": {
            "explicit_only": {
              "brotli_total_bytes": 17,
              "gzip_total_bytes": 19,
              "javascript_glue": {"brotli_bytes": 7,"gzip_bytes": 8,"raw_bytes": 9,"relative_path":"explicit.js","sha256":"bb"},
              "raw_total_bytes": 21,
              "wasm": {"brotli_bytes": 10,"gzip_bytes": 11,"raw_bytes": 12,"relative_path":"explicit.wasm","sha256":"aa"}
            },
            "full": {
              "brotli_total_bytes": 37,
              "gzip_total_bytes": 39,
              "javascript_glue": {"brotli_bytes": 17,"gzip_bytes": 18,"raw_bytes": 19,"relative_path":"full.js","sha256":"dd"},
              "raw_total_bytes": 41,
              "wasm": {"brotli_bytes": 20,"gzip_bytes": 21,"raw_bytes": 22,"relative_path":"full.wasm","sha256":"cc"}
            }
          }
        }"#,
    )
    .expect("browser report");

    let evidence = load_browser_build_evidence(Path::new(&path)).expect("browser builds");

    assert_eq!(evidence.full.wasm.raw_bytes, 22);
    assert_eq!(evidence.full.raw_total_bytes, 41);
    assert_eq!(evidence.full.gzip_total_bytes, 39);
    assert_eq!(evidence.full.brotli_total_bytes, 37);
    assert_eq!(evidence.explicit_only.wasm.raw_bytes, 12);
    assert_eq!(evidence.explicit_only.raw_total_bytes, 21);
    assert_eq!(evidence.explicit_only.gzip_total_bytes, 19);
    assert_eq!(evidence.explicit_only.brotli_total_bytes, 17);

    let _: BrowserBuildEvidence = evidence;
}

#[test]
fn pinned_corpus_validation_checks_counts_digests_and_termination() {
    let corpus = toxbench::AutoCorpusEvidence {
        rows: 418_882,
        supported_rows: 147_432,
        unsupported_rows: 271_450,
        text_sha256: "8c67c444dec9216991532dee6fdcf4b84843c349fbee218cf70fc6df3d8c5786".to_owned(),
        label_sha256: "f88ed093f49c0715b75cd6a2d66ad55db936183e35278515925de31c034d8549".to_owned(),
        text_has_final_newline: false,
        labels_have_final_newline: false,
    };

    validate_pinned_corpus(&corpus).expect("pinned corpus identity");

    let mut wrong = corpus;
    wrong.supported_rows -= 1;
    assert!(validate_pinned_corpus(&wrong).is_err());
}

#[test]
fn c_parity_verification_rejects_one_changed_expected_score() {
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../eldc/tests/fixtures/c-parity-v1.jsonl");
    let first = fs::read_to_string(source)
        .expect("C parity fixture")
        .lines()
        .next()
        .expect("first parity row")
        .to_owned();
    let mut row: serde_json::Value = serde_json::from_str(&first).expect("parity row JSON");
    row["top_score"] = serde_json::json!(0.0);
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("changed-parity.jsonl");
    fs::write(
        &path,
        serde_json::to_vec(&row).expect("changed parity JSON"),
    )
    .expect("changed parity fixture");

    let error = verify_c_parity_fixture(&path).expect_err("changed score must fail");

    assert!(error.to_string().contains("top_score"));
}

#[test]
fn published_auto_evidence_requires_the_best_effort_unsupported_language_limitation() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/eldc-auto-validation.json");
    let bytes = fs::read(path).expect("published AUTO evidence");
    let evidence: toxbench::AutoValidationEvidence =
        serde_json::from_slice(&bytes).expect("AUTO evidence JSON");

    assert!(evidence.limitations.iter().any(|limitation| {
        limitation == "Unsupported-language rejection is best-effort with this 15-profile model."
    }));
}
