use std::path::PathBuf;

use blasphem::{Language, NudgeDetector, ReplyTarget, RuleId, SparseModel};

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

#[test]
fn grawlix_masks_matched_spans() {
    let detector = detector(Language::En);
    let result = detector.analyze("you are a stupid loser", ReplyTarget::Person);
    let spans = blasphem::masked_spans(&result);
    let masked = blasphem::apply_grawlix(&result.original_text, &spans);

    assert!(!spans.is_empty(), "expected at least one matched span");
    assert!(masked.starts_with("you are a "), "got {masked}");
    assert!(!masked.contains("stupid"), "got {masked}");
    assert_eq!(masked.len(), result.original_text.len());
}

#[test]
fn grawlix_leaves_clean_text_untouched() {
    let detector = detector(Language::En);
    let result = detector.analyze("good morning everyone", ReplyTarget::Person);
    let masked = blasphem::apply_grawlix(&result.original_text, &blasphem::masked_spans(&result));

    assert_eq!(masked, "good morning everyone");
}

#[test]
fn grawlix_preserves_multibyte_tails() {
    let detector = detector(Language::Es);
    let result = detector.analyze("eres un idiota señor", ReplyTarget::Person);
    let masked = blasphem::apply_grawlix(&result.original_text, &blasphem::masked_spans(&result));

    assert!(masked.ends_with("señor"), "got {masked}");
}

#[test]
fn judge_reports_unsafe_with_a_normalized_score() {
    let judge = blasphem::Judge::new(blasphem::JudgeOptions {
        locales: vec![Language::En, Language::Es],
        ..blasphem::JudgeOptions::default()
    })
    .expect("judge builds");
    let verdict = judge.judge("you are a stupid loser");

    // These exact values appear in README.md and in the package README.
    assert!(!verdict.safe);
    assert_eq!(verdict.score, 0.64);
    assert_eq!(verdict.locale, Some(Language::En));
    assert_eq!(verdict.grawlix, None);
}

#[test]
fn judge_returns_grawlix_only_when_requested() {
    let judge = blasphem::Judge::new(blasphem::JudgeOptions {
        locales: vec![Language::En],
        grawlix: true,
        ..blasphem::JudgeOptions::default()
    })
    .expect("judge builds");
    let masked = judge.judge("you are a stupid loser").grawlix;

    // This exact string appears in README.md and in the package README.
    assert_eq!(masked.as_deref(), Some("you are a @#$%&! loser"));
}

#[test]
fn judge_fails_open_when_the_detected_locale_is_not_loaded() {
    let judge = blasphem::Judge::new(blasphem::JudgeOptions {
        locales: vec![Language::Ko],
        ..blasphem::JudgeOptions::default()
    })
    .expect("judge builds");
    let verdict = judge.judge("you are a stupid loser");

    assert!(verdict.safe);
    assert_eq!(verdict.score, 0.0);
    assert_eq!(verdict.locale, None);
}

#[test]
fn judge_without_detection_scores_every_loaded_locale() {
    let judge = blasphem::Judge::new(blasphem::JudgeOptions {
        locales: vec![Language::En, Language::Es],
        detect_language: false,
        ..blasphem::JudgeOptions::default()
    })
    .expect("judge builds");
    let verdict = judge.judge("eres un idiota");

    assert_eq!(verdict.locale, Some(Language::Es));
    assert!(!verdict.safe);
}
