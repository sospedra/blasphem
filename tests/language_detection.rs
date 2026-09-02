use std::str::FromStr;

#[cfg(feature = "language-detection")]
use toxcheck::LanguageDetector;
use toxcheck::{
    Language, LanguageDetection, LanguageIdentifier, LanguageResolution, LanguageSelection,
    LanguageSource, resolve_language,
};

#[test]
fn language_selection_accepts_canonical_codes_and_the_legacy_id_alias() {
    let cases = [
        ("EN", Language::En),
        ("ZH", Language::Zh),
        ("ES", Language::Es),
        ("AR", Language::Ar),
        ("MS", Language::Ms),
        ("PT", Language::Pt),
        ("FR", Language::Fr),
        ("HI", Language::Hi),
        ("RU", Language::Ru),
        ("JA", Language::Ja),
        ("DE", Language::De),
        ("TR", Language::Tr),
        ("VI", Language::Vi),
        ("KO", Language::Ko),
        ("IT", Language::It),
    ];

    for (code, language) in cases {
        assert_eq!(
            LanguageSelection::from_str(code),
            Ok(LanguageSelection::Explicit(language))
        );
        assert_eq!(
            LanguageSelection::from_str(&code.to_ascii_lowercase()),
            Ok(LanguageSelection::Explicit(language))
        );
    }
    assert_eq!(
        LanguageSelection::from_str("ID"),
        Ok(LanguageSelection::Explicit(Language::Ms))
    );
    assert_eq!(
        LanguageSelection::from_str("id"),
        Ok(LanguageSelection::Explicit(Language::Ms))
    );
}

#[test]
fn language_selection_accepts_trimmed_mixed_case_auto() {
    assert_eq!(
        LanguageSelection::from_str("  AuTo\t"),
        Ok(LanguageSelection::Auto)
    );
    assert!(Language::from_str("AUTO").is_err());
}

#[test]
fn language_selection_rejects_invalid_codes() {
    for value in ["", " ", "AUT", "ENGLISH", "XX"] {
        assert!(LanguageSelection::from_str(value).is_err(), "{value:?}");
    }
}

#[test]
#[cfg(feature = "language-detection")]
fn automatic_detection_maps_all_fifteen_eldc_languages() {
    let detector = LanguageDetector::new().expect("embedded language model");
    let cases = [
        ("I never should've bought that.", Language::En),
        ("我想要确定什么都没有发生在汤姆身上。", Language::Zh),
        ("Was ist das? A ship. Todo bien en la costa.", Language::Es),
        ("هل تحب الكتب؟", Language::Ar),
        (
            "Dia memberitahu saya yang dia benar-benar letih.",
            Language::Ms,
        ),
        ("Não vou chegar em casa até segunda.", Language::Pt),
        ("Bonjour le monde", Language::Fr),
        ("वह मेरे पिताजी की माँ है। वह मेरी दादी है।", Language::Hi),
        ("Они были здесь.", Language::Ru),
        ("私は２日間忙しくありません。", Language::Ja),
        ("Was ist das?", Language::De),
        ("Çok büyük bir musibet.", Language::Tr),
        ("Đây là 1 lời nói đùa cợt", Language::Vi),
        ("물이 별로 없다.", Language::Ko),
        ("La incontrerai domani sera.", Language::It),
    ];

    for (text, language) in cases {
        let result = detector.identify(text);
        assert_eq!(result.source, LanguageSource::Automatic, "{text}");
        assert_eq!(
            result.resolution,
            LanguageResolution::Known(language),
            "{text}"
        );
        assert!(result.reliable, "{text}");
        assert!(result.score.is_some_and(|score| score > 0.0), "{text}");
        assert!(
            result.feature_count.is_some_and(|count| count >= 3),
            "{text}"
        );
    }
}

#[test]
#[cfg(feature = "language-detection")]
fn automatic_detection_returns_unknown_for_unreliable_input() {
    let detector = LanguageDetector::new().expect("embedded language model");

    for text in ["", "!@#$%^&*()", "😀🚀🧪❤️", "Hello"] {
        let result = resolve_language(LanguageSelection::Auto, text, &detector);
        assert_eq!(result.source, LanguageSource::Automatic, "{text:?}");
        assert_eq!(result.resolution, LanguageResolution::Unknown, "{text:?}");
        assert!(!result.reliable, "{text:?}");
        assert!(result.feature_count.is_some(), "{text:?}");
    }
}

struct PanicIdentifier;

impl LanguageIdentifier for PanicIdentifier {
    fn identify(&self, _text: &str) -> LanguageDetection {
        panic!("explicit language resolution called the identifier")
    }
}

#[test]
fn explicit_resolution_bypasses_language_identification() {
    let result = resolve_language(
        LanguageSelection::Explicit(Language::Ms),
        "any text",
        &PanicIdentifier,
    );

    assert_eq!(
        result,
        LanguageDetection {
            source: LanguageSource::Explicit,
            resolution: LanguageResolution::Known(Language::Ms),
            reliable: true,
            score: None,
            feature_count: None,
        }
    );
}
