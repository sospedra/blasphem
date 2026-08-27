use blasphem::Language;
use blasphem_wasm::DetectorCore;

#[cfg(feature = "language-detection")]
const AUTO_CASES: &[(Language, &str, u8)] = &[
    (Language::En, "I never should've bought that.", 42),
    (Language::Zh, "我想要确定什么都没有发生在汤姆身上。", 28),
    (
        Language::Es,
        "Was ist das? A ship. Todo bien en la costa.",
        17,
    ),
    (Language::Ar, "هل تحب الكتب؟", 11),
    (
        Language::Ms,
        "Dia memberitahu saya yang dia benar-benar letih.",
        38,
    ),
    (Language::Pt, "Não vou chegar em casa até segunda.", 15),
    (Language::Fr, "Bonjour le monde", 22),
    (Language::Hi, "वह मेरे पिताजी की माँ है। वह मेरी दादी है।", 38),
    (Language::Ru, "Они были здесь.", 46),
    (Language::Ja, "私は２日間忙しくありません。", 0),
    (Language::De, "Was ist das?", 0),
    (Language::Tr, "Çok büyük bir musibet.", 0),
    (Language::Vi, "Đây là 1 lời nói đùa cợt", 13),
    (Language::Ko, "물이 별로 없다.", 18),
    (Language::It, "La incontrerai domani sera.", 1),
];

#[test]
fn explicit_core_initializes_every_language_and_returns_route_fields() {
    for language in Language::ALL {
        let detector = DetectorCore::new(language.code()).expect("embedded detector");
        let result = detector.check("A neutral message");

        assert_eq!(detector.language(), language.code());
        assert!(result.score <= 100, "{}", language.code());
        assert_eq!(result.threshold, 50, "{}", language.code());
        assert_eq!(result.ok, !result.should_nudge, "{}", language.code());
        assert_eq!(
            result.should_nudge,
            result.score >= result.threshold,
            "{}",
            language.code(),
        );
        assert!(result.evaluated, "{}", language.code());
        assert_eq!(result.resolved_language, language.code());
        assert!(result.language_reliable, "{}", language.code());
        assert_eq!(result.language_score, None, "{}", language.code());
    }
}

#[test]
fn explicit_ms_and_id_alias_return_the_same_canonical_result() {
    let ms = DetectorCore::new("MS").expect("MS detector");
    let id = DetectorCore::new("ID").expect("ID alias detector");

    assert_eq!(ms.language(), "MS");
    assert_eq!(id.language(), "MS");
    assert_eq!(
        ms.check("Dia memberitahu saya yang dia benar-benar letih."),
        id.check("Dia memberitahu saya yang dia benar-benar letih."),
    );
}

#[cfg(feature = "language-detection")]
#[test]
fn automatic_core_resolves_all_fifteen_languages_and_matches_explicit_toxicity() {
    let automatic = DetectorCore::new("AUTO").expect("automatic detector");

    assert_eq!(automatic.language(), "AUTO");
    for &(language, text, expected_score) in AUTO_CASES {
        let automatic_result = automatic.check(text);
        let explicit_result = DetectorCore::new(language.code())
            .expect("explicit detector")
            .check(text);

        assert_eq!(
            automatic_result.resolved_language,
            language.code(),
            "{text}"
        );
        assert!(automatic_result.evaluated, "{text}");
        assert!(automatic_result.language_reliable, "{text}");
        assert!(
            automatic_result
                .language_score
                .is_some_and(|score| score > 0.0),
            "{text}",
        );
        assert_eq!(automatic_result.score, expected_score, "{text}");
        assert_eq!(automatic_result.ok, explicit_result.ok, "{text}");
        assert_eq!(automatic_result.score, explicit_result.score, "{text}");
        assert_eq!(
            automatic_result.threshold, explicit_result.threshold,
            "{text}"
        );
        assert_eq!(
            automatic_result.should_nudge, explicit_result.should_nudge,
            "{text}",
        );
    }
}

#[cfg(feature = "language-detection")]
#[test]
fn automatic_core_fails_open_without_evaluating_unreliable_input() {
    let detector = DetectorCore::new("AUTO").expect("automatic detector");

    for text in ["", "!@#$%^&*()", "😀🚀🧪❤️", "Hello"] {
        let result = detector.check(text);

        assert!(result.ok, "{text:?}");
        assert_eq!(result.score, 0, "{text:?}");
        assert_eq!(result.threshold, 50, "{text:?}");
        assert!(!result.should_nudge, "{text:?}");
        assert!(!result.evaluated, "{text:?}");
        assert_eq!(result.resolved_language, "unknown", "{text:?}");
        assert!(!result.language_reliable, "{text:?}");
        assert_eq!(result.language_score, None, "{text:?}");
    }
}

#[cfg(not(feature = "language-detection"))]
#[test]
fn automatic_core_requires_the_optional_language_detection_feature() {
    let error = DetectorCore::new("AUTO").expect_err("AUTO must require its feature");

    assert_eq!(error, "AUTO requires the language-detection feature");
}

#[test]
fn core_rejects_unknown_language_values() {
    for language in ["", "EN-US", "XX"] {
        assert!(
            DetectorCore::new(language).is_err(),
            "{language:?} must fail"
        );
    }
}

#[test]
fn core_judge_scores_english_and_masks_on_request() {
    let core = blasphem_wasm::JudgeCore::new(&["en".to_owned(), "es".to_owned()], true, true)
        .expect("judge core builds");
    let verdict = core.judge("you are a stupid loser");

    assert!(!verdict.safe);
    assert!(verdict.score > 0.0 && verdict.score <= 1.0);
    assert_eq!(verdict.locale.as_deref(), Some("en"));
    assert!(
        verdict
            .grawlix
            .is_some_and(|masked| !masked.contains("stupid"))
    );
}

#[test]
fn core_judge_omits_grawlix_when_not_requested() {
    let core =
        blasphem_wasm::JudgeCore::new(&["en".to_owned()], true, false).expect("judge core builds");

    assert_eq!(core.judge("you are a stupid loser").grawlix, None);
}

#[test]
fn core_judge_rejects_an_unknown_locale() {
    let error = blasphem_wasm::JudgeCore::new(&["xx".to_owned()], true, false)
        .expect_err("unknown locale fails");

    assert!(error.contains("xx"), "got {error}");
}
