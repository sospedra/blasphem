use std::path::PathBuf;

use blasphem::{Language, NudgeDetector, PackInput, PackSource, ReplyTarget, RuleId, SparseModel};

fn lexicon_bytes(language: Language) -> Vec<u8> {
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .join("resources/lexicon")
        .join(format!("{}.tsv", language.storage_code()));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn detector(language: Language) -> NudgeDetector {
    let bytes = lexicon_bytes(language);
    NudgeDetector::from_lexicon_bytes(language, Some(&bytes)).expect("runtime detector")
}

fn sparse_model(language: Language) -> SparseModel {
    let filename = if language == Language::Es {
        "es-sparse.bin".to_owned()
    } else {
        format!(
            "{}-sparse.bin",
            language.storage_code().to_ascii_lowercase()
        )
    };
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .join("resources/models")
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
fn runtime_rejects_missing_required_lexicon_data() {
    let error = NudgeDetector::from_lexicon_bytes(Language::En, None)
        .expect_err("missing Lexicon must fail");

    assert!(error.to_string().contains("EN"));
    assert!(error.to_string().contains("missing"));
}

#[test]
fn runtime_rejects_a_lexicon_digest_mismatch() {
    let mut bytes = lexicon_bytes(Language::En);
    bytes.push(b'\n');

    let error = NudgeDetector::from_lexicon_bytes(Language::En, Some(&bytes))
        .expect_err("changed Lexicon must fail");

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
fn clean_controls_do_not_pin_the_english_or_russian_boundaries() {
    for (language, text) in [
        (Language::Ru, "Ты написал thank you"),
        (Language::En, "I will kill your process"),
    ] {
        let model = sparse_model(language);
        let score = model.score(text);

        assert!(
            score < 45,
            "{} control remains pinned at {score}; raw score {}, boundary {}",
            language.code(),
            model.raw_score(text),
            model.raw_boundary(),
        );
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
    assert_eq!(verdict.score, 0.95);
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

    assert_eq!(masked.as_deref(), Some("you are a @#$%&! @#$%&"));
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

fn artifact_bytes(language: Language) -> Vec<u8> {
    let filename = if language == Language::Es {
        "es-sparse.bin".to_owned()
    } else {
        format!(
            "{}-sparse.bin",
            language.storage_code().to_ascii_lowercase()
        )
    };
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .join("resources/models")
        .join(filename);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn rule_pack_version(language: Language) -> u16 {
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .join("resources/metadata/model-manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("model manifest"))
            .expect("valid manifest json");
    let entry = manifest["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["language"] == language.code())
        .unwrap_or_else(|| panic!("{} has no manifest entry", language.code()));
    u16::try_from(
        entry["rule_pack_version"]
            .as_u64()
            .expect("rule pack version"),
    )
    .expect("u16")
}

fn pack_bytes(language: Language) -> Vec<u8> {
    let artifact = artifact_bytes(language);
    let lexicon = lexicon_bytes(language);
    blasphem::encode_pack(&PackInput {
        language,
        rule_pack_version: rule_pack_version(language),
        artifact: &artifact,
        lexicon: &lexicon,
    })
}

#[cfg(feature = "language-detection")]
fn detect_bytes(language: Language) -> Vec<u8> {
    let model = std::fs::read(
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .join("crates/blasphem-language/data/blasphem-language-15.bin"),
    )
    .expect("committed language model");
    let code = language.code().to_ascii_lowercase();
    blasphem_language::slice::write_slices(&model)
        .expect("slices")
        .into_iter()
        .find(|(slice_language, _)| slice_language.code() == code)
        .map(|(_, bytes)| bytes)
        .unwrap_or_else(|| panic!("{code} has no slice"))
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).into()
}

#[cfg(feature = "language-detection")]
#[test]
fn judge_from_packs_matches_the_embedded_judge() {
    let en_pack = pack_bytes(Language::En);
    let es_pack = pack_bytes(Language::Es);
    let en_detect = detect_bytes(Language::En);
    let es_detect = detect_bytes(Language::Es);
    let sources = [
        PackSource {
            language: Language::En,
            pack: &en_pack,
            pack_sha256: Some(sha256(&en_pack)),
            detect: Some(&en_detect),
            detect_sha256: Some(sha256(&en_detect)),
        },
        PackSource {
            language: Language::Es,
            pack: &es_pack,
            pack_sha256: Some(sha256(&es_pack)),
            detect: Some(&es_detect),
            detect_sha256: Some(sha256(&es_detect)),
        },
    ];
    let from_packs = blasphem::Judge::from_packs(&sources, true, true).expect("judge from packs");
    let embedded = blasphem::Judge::new(blasphem::JudgeOptions {
        locales: vec![Language::En, Language::Es],
        detect_language: true,
        grawlix: true,
    })
    .expect("embedded judge");

    assert_eq!(from_packs.locales(), vec![Language::En, Language::Es]);
    for text in [
        "you are a stupid loser",
        "eres un idiota",
        "good morning everyone",
        "Was ist das?",
        "",
    ] {
        assert_eq!(from_packs.judge(text), embedded.judge(text), "{text:?}");
    }
    assert_eq!(from_packs.judge("you are a stupid loser").score, 0.95);
}

#[test]
fn judge_from_packs_without_detection_scores_every_loaded_locale() {
    let en_pack = pack_bytes(Language::En);
    let es_pack = pack_bytes(Language::Es);
    let sources = [
        PackSource {
            language: Language::Es,
            pack: &es_pack,
            pack_sha256: None,
            detect: None,
            detect_sha256: None,
        },
        PackSource {
            language: Language::En,
            pack: &en_pack,
            pack_sha256: None,
            detect: None,
            detect_sha256: None,
        },
    ];
    let judge = blasphem::Judge::from_packs(&sources, false, false).expect("judge from packs");

    assert_eq!(judge.locales(), vec![Language::En, Language::Es]);
    let verdict = judge.judge("eres un idiota");
    assert_eq!(verdict.locale, Some(Language::Es));
    assert!(!verdict.safe);
}

#[test]
fn judge_from_packs_rejects_a_digest_mismatch_by_file_name() {
    let en_pack = pack_bytes(Language::En);
    let sources = [PackSource {
        language: Language::En,
        pack: &en_pack,
        pack_sha256: Some([0; 32]),
        detect: None,
        detect_sha256: None,
    }];
    let error = blasphem::Judge::from_packs(&sources, false, false).expect_err("bad digest");

    assert!(
        error
            .to_string()
            .starts_with("BLASPHEM_DIGEST_MISMATCH: en.pack expected sha256 0000"),
        "got {error}"
    );
}

#[test]
fn judge_from_packs_rejects_a_foreign_format_version() {
    let mut en_pack = pack_bytes(Language::En);
    en_pack[8..12].copy_from_slice(&2_u32.to_le_bytes());
    let sources = [PackSource {
        language: Language::En,
        pack: &en_pack,
        pack_sha256: None,
        detect: None,
        detect_sha256: None,
    }];
    let error = blasphem::Judge::from_packs(&sources, false, false).expect_err("bad version");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_FORMAT_VERSION: en.pack has format version 2, this build accepts 1"
    );
}

#[test]
fn judge_from_packs_rejects_a_pack_for_another_language() {
    let es_pack = pack_bytes(Language::Es);
    let sources = [PackSource {
        language: Language::En,
        pack: &es_pack,
        pack_sha256: None,
        detect: None,
        detect_sha256: None,
    }];
    let error = blasphem::Judge::from_packs(&sources, false, false).expect_err("wrong language");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_PACK_INVALID: en.pack declares es"
    );
}

#[cfg(feature = "language-detection")]
#[test]
fn judge_from_packs_requires_a_detect_slice_when_detection_is_on() {
    let en_pack = pack_bytes(Language::En);
    let sources = [PackSource {
        language: Language::En,
        pack: &en_pack,
        pack_sha256: None,
        detect: None,
        detect_sha256: None,
    }];
    let error = blasphem::Judge::from_packs(&sources, true, false).expect_err("missing slice");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_PACK_INVALID: en.detect is required when language detection is on"
    );
}

#[test]
fn judge_from_packs_rejects_empty_sources_and_repeated_locales() {
    let en_pack = pack_bytes(Language::En);
    let source = PackSource {
        language: Language::En,
        pack: &en_pack,
        pack_sha256: None,
        detect: None,
        detect_sha256: None,
    };

    assert_eq!(
        blasphem::Judge::from_packs(&[], false, false)
            .expect_err("no sources")
            .to_string(),
        "BLASPHEM_LOCALES_EMPTY: no locale was given"
    );
    assert_eq!(
        blasphem::Judge::from_packs(&[source, source], false, false)
            .expect_err("repeated")
            .to_string(),
        "BLASPHEM_PACK_INVALID: en.pack was given twice"
    );
}
