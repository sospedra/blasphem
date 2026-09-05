use std::str::FromStr;

use blasphem::{
    FeatureProfile, FeatureSchema, Language, NormalizationProfile, language_spec, normalize,
};

#[test]
fn language_contract_contains_exactly_fifteen_codes() {
    let expected = [
        "EN", "ZH", "ES", "AR", "MS", "PT", "FR", "HI", "RU", "JA", "DE", "TR", "VI", "KO", "IT",
    ];
    let actual = Language::ALL.map(Language::code);
    assert_eq!(actual, expected);
    for code in expected {
        assert_eq!(Language::from_str(code).expect("supported").code(), code);
        assert_eq!(
            Language::from_str(&code.to_ascii_lowercase())
                .expect("supported")
                .code(),
            code
        );
    }
    assert!(Language::from_str("HINGLISH").is_err());
    assert_eq!(Language::from_str("ID"), Ok(Language::Ms));
    assert_eq!(
        serde_json::from_str::<Language>("\"ID\"").expect("legacy ID alias"),
        Language::Ms
    );
    assert_eq!(Language::Ms.storage_code(), "ID");
    assert_eq!(Language::En.storage_code(), "EN");
    assert_eq!(Language::Ms.to_string(), "MS");
}

#[test]
fn normalization_profiles_match_frozen_vectors() {
    let cases = [
        (NormalizationProfile::Generic, "ＦＯＯ Straße", "foo straße"),
        (NormalizationProfile::Turkish, "I İ ı i", "ı i ı i"),
        (NormalizationProfile::Vietnamese, "Tôi rất tệ", "tôi rất tệ"),
        (NormalizationProfile::Arabic, "إِنَّ ـآدم فتاة", "ان ادم فتاة"),
        (NormalizationProfile::Hindi, "क्\u{200d}ष", "क्\u{200d}ष"),
        (NormalizationProfile::Chinese, "ＡＢＣ你", "abc你"),
        (NormalizationProfile::Japanese, "ガＡ", "ガa"),
        (NormalizationProfile::Korean, "한글Ａ", "한글a"),
    ];
    for (profile, input, expected) in cases {
        assert_eq!(normalize(profile, input), expected);
    }

    assert_eq!(
        normalize(NormalizationProfile::SpanishCharabia, "texto"),
        "texto".to_owned()
    );
}

#[test]
fn language_profiles_and_indexes_match_the_exact_table() {
    let cases = [
        (
            Language::En,
            0,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Zh,
            1,
            FeatureProfile::ChineseScriptChar15,
            NormalizationProfile::Chinese,
            FeatureSchema::Sparse,
        ),
        (
            Language::Es,
            2,
            FeatureProfile::SpanishWordChar35,
            NormalizationProfile::SpanishCharabia,
            FeatureSchema::Sparse,
        ),
        (
            Language::Ar,
            3,
            FeatureProfile::WordChar35,
            NormalizationProfile::Arabic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Ms,
            4,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Pt,
            5,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Fr,
            6,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Hi,
            7,
            FeatureProfile::WordChar35,
            NormalizationProfile::Hindi,
            FeatureSchema::Sparse,
        ),
        (
            Language::Ru,
            8,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Ja,
            9,
            FeatureProfile::Char25,
            NormalizationProfile::Japanese,
            FeatureSchema::Sparse,
        ),
        (
            Language::De,
            10,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
        (
            Language::Tr,
            11,
            FeatureProfile::TurkishChar35,
            NormalizationProfile::Turkish,
            FeatureSchema::Sparse,
        ),
        (
            Language::Vi,
            12,
            FeatureProfile::WordChar35,
            NormalizationProfile::Vietnamese,
            FeatureSchema::Sparse,
        ),
        (
            Language::Ko,
            13,
            FeatureProfile::KoreanWordChar25,
            NormalizationProfile::Korean,
            FeatureSchema::Sparse,
        ),
        (
            Language::It,
            14,
            FeatureProfile::WordChar35,
            NormalizationProfile::Generic,
            FeatureSchema::Sparse,
        ),
    ];

    for (language, index, feature, normalization, schema) in cases {
        assert_eq!(Language::ALL[index], language);
        assert_eq!(language.index(), index);
        assert_eq!(language.profiles(), (feature, normalization, schema));
    }
}

#[test]
fn every_language_has_one_immutable_runtime_spec() {
    for language in Language::ALL {
        let spec = language_spec(language);
        assert_eq!(spec.language, language);
        assert_eq!(
            (
                spec.feature_profile,
                spec.normalization_profile,
                spec.feature_schema
            ),
            language.profiles()
        );
    }
}

#[test]
fn language_json_uses_only_uppercase_codes() {
    for language in Language::ALL {
        let json = serde_json::to_string(&language).expect("serialize language");
        assert_eq!(json, format!("\"{}\"", language.code()));
        assert_eq!(
            serde_json::from_str::<Language>(&json).expect("deserialize language"),
            language
        );
    }
    assert!(serde_json::from_str::<Language>("\"es\"").is_err());
}

#[test]
fn profile_json_names_match_the_exact_tables() {
    let feature_cases = [
        (FeatureProfile::SpanishWordChar35, "SpanishWordChar35"),
        (FeatureProfile::WordChar35, "WordChar35"),
        (FeatureProfile::Char25, "Char25"),
        (FeatureProfile::TurkishChar35, "TurkishChar35"),
        (FeatureProfile::ChineseScriptChar15, "ChineseScriptChar15"),
        (FeatureProfile::KoreanWordChar25, "KoreanWordChar25"),
    ];
    for (profile, name) in feature_cases {
        let json = format!("\"{name}\"");
        assert_eq!(
            serde_json::to_string(&profile).expect("serialize feature profile"),
            json
        );
        assert_eq!(
            serde_json::from_str::<FeatureProfile>(&json).expect("deserialize feature profile"),
            profile
        );
    }

    let normalization_cases = [
        (NormalizationProfile::SpanishCharabia, "SpanishCharabia"),
        (NormalizationProfile::Generic, "Generic"),
        (NormalizationProfile::Turkish, "Turkish"),
        (NormalizationProfile::Vietnamese, "Vietnamese"),
        (NormalizationProfile::Arabic, "Arabic"),
        (NormalizationProfile::Hindi, "Hindi"),
        (NormalizationProfile::Chinese, "Chinese"),
        (NormalizationProfile::Japanese, "Japanese"),
        (NormalizationProfile::Korean, "Korean"),
    ];
    for (profile, name) in normalization_cases {
        let json = format!("\"{name}\"");
        assert_eq!(
            serde_json::to_string(&profile).expect("serialize normalization profile"),
            json
        );
        assert_eq!(
            serde_json::from_str::<NormalizationProfile>(&json)
                .expect("deserialize normalization profile"),
            profile
        );
    }

    let schema_cases = [
        (FeatureSchema::Sparse, "Sparse"),
        (FeatureSchema::Sparse, "Sparse"),
    ];
    for (schema, name) in schema_cases {
        let json = format!("\"{name}\"");
        assert_eq!(
            serde_json::to_string(&schema).expect("serialize feature schema"),
            json
        );
        assert_eq!(
            serde_json::from_str::<FeatureSchema>(&json).expect("deserialize feature schema"),
            schema
        );
    }
}
