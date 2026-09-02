use std::path::PathBuf;

use toxcheck::{Language, NudgeDetector, ReplyTarget, RuleId, SparseModel};

fn hurtlex_bytes(language: Language) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/raw-v1/hurtlex")
        .join(language.storage_code())
        .join("1.2")
        .join(format!("hurtlex_{}.tsv", language.storage_code()));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn detector(language: Language) -> NudgeDetector {
    let bytes = hurtlex_bytes(language);
    NudgeDetector::from_hurtlex_bytes(language, Some(&bytes)).expect("runtime detector")
}

fn sparse_model(language: Language) -> SparseModel {
    let filename = if language == Language::Es {
        "es-chargram-v1.bin".to_owned()
    } else {
        format!(
            "{}-sparse-v2.bin",
            language.storage_code().to_ascii_lowercase()
        )
    };
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources/models/multilingual-v2")
        .join(filename);
    let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    SparseModel::from_bytes(&bytes).expect("valid sparse model")
}

#[test]
fn every_language_keeps_the_public_result_invariant() {
    for language in Language::ALL {
        let detector = detector(language);
        assert_eq!(detector.language(), language);
        for text in ["", "neutral message", "I will kill you"] {
            let analysis = detector.analyze(text, ReplyTarget::Unknown);
            let result = detector.check(text, ReplyTarget::Unknown);

            assert_eq!(result, analysis.nudge(), "{}", language.code());
            assert_eq!(
                result.score,
                analysis
                    .sparse_score
                    .unwrap_or(0)
                    .max(analysis.max_risk_points()),
                "{}",
                language.code(),
            );
            assert!(result.score <= 100, "{}", language.code());
            assert_eq!(result.threshold, 50, "{}", language.code());
            assert_eq!(
                result.should_nudge,
                result.score >= result.threshold,
                "{}",
                language.code(),
            );
        }
    }
}

#[test]
fn public_runtime_does_not_truncate_after_4096_bytes() {
    let detector = detector(Language::En);
    let text = format!("{}I will kill you", "neutral ".repeat(700));

    assert!(text.len() > 4_096);
    assert!(detector.check(&text, ReplyTarget::Unknown).should_nudge);
}

#[test]
fn runtime_rejects_missing_required_hurtlex_data() {
    let error = NudgeDetector::from_hurtlex_bytes(Language::En, None)
        .expect_err("missing HurtLex must fail");

    assert!(error.to_string().contains("EN"));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn runtime_rejects_a_hurtlex_digest_mismatch() {
    let mut bytes = hurtlex_bytes(Language::En);
    bytes.push(b'\n');

    let error = NudgeDetector::from_hurtlex_bytes(Language::En, Some(&bytes))
        .expect_err("changed HurtLex must fail");

    assert!(error.to_string().contains("EN"));
    assert!(error.to_string().contains("digest"));
}

#[test]
fn runtime_does_not_run_the_legacy_rule_engine_for_new_languages() {
    for (language, text) in [
        (Language::Fr, "Je vais te frapper au tennis"),
        (Language::Pt, "Vou bater em você no tênis"),
        (Language::It, "Voglio attaccare te a scacchi"),
        (Language::Tr, "Seni maçta vuracağım"),
        (Language::Vi, "Tôi sẽ đánh bạn trong trận đấu"),
    ] {
        let result = detector(language).check(text, ReplyTarget::Unknown);
        assert!(!result.should_nudge, "{}: {text}", language.code());
    }
}

#[test]
fn runtime_applies_only_declared_context_suppression_to_sparse_scores() {
    for language in Language::ALL {
        let detector = detector(language);
        let model = sparse_model(language);
        for text in [
            "\"I will kill you\"",
            "The moderator reported a threatening message",
            "I will not kill you",
        ] {
            let analysis = detector.analyze(text, ReplyTarget::Unknown);
            let has_context_suppression = analysis.evidence.iter().any(|item| {
                item.points == 0
                    && matches!(
                        item.rule_id,
                        RuleId::NegatedEvidence
                            | RuleId::QuotedEvidence
                            | RuleId::ReportedEvidence
                            | RuleId::CounterspeechEvidence
                    )
            });
            let expected = if language != Language::Es && has_context_suppression {
                model.score(text).min(49)
            } else {
                model.score(text)
            };
            assert_eq!(
                analysis.sparse_score,
                Some(expected),
                "{}: {text}",
                language.code(),
            );
        }
    }
}
