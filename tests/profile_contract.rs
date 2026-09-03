use std::str::FromStr;

use blasphem::{
    FeatureProfile, FeatureSchema, Language, NormalizationError, NormalizationProfile,
    language_spec, normalize_v2,
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
        (
            NormalizationProfile::GenericV2,
            "ＦＯＯ Straße",
            "foo straße",
        ),
        (NormalizationProfile::TurkishV2, "I İ ı i", "ı i ı i"),
        (
            NormalizationProfile::VietnameseV2,
            "Tôi rất tệ",
            "tôi rất tệ",
        ),
        (
            NormalizationProfile::ArabicV2,
            "إِنَّ ـآدم فتاة",
            "ان ادم فتاة",
        ),
        (NormalizationProfile::HindiV2, "क्\u{200d}ष", "क्\u{200d}ष"),
        (NormalizationProfile::ChineseV2, "ＡＢＣ你", "abc你"),
        (NormalizationProfile::JapaneseV2, "ガＡ", "ガa"),
        (NormalizationProfile::KoreanV2, "한글Ａ", "한글a"),
    ];
    for (profile, input, expected) in cases {
        assert_eq!(normalize_v2(profile, input).expect("normalize"), expected);
    }

    assert_eq!(
        normalize_v2(NormalizationProfile::EsLegacyCharabiaV1, "texto"),
        Err(NormalizationError::LegacyProfile)
    );
}

#[test]
fn language_profiles_and_indexes_match_the_exact_table() {
    let cases = [
        (
            Language::En,
            0,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Zh,
            1,
            FeatureProfile::ChineseScriptChar15V3,
            NormalizationProfile::ChineseV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Es,
            2,
            FeatureProfile::EsLegacyWordChar35V1,
            NormalizationProfile::EsLegacyCharabiaV1,
            FeatureSchema::EsLegacyV1,
        ),
        (
            Language::Ar,
            3,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::ArabicV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Ms,
            4,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Pt,
            5,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Fr,
            6,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Hi,
            7,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::HindiV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Ru,
            8,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Ja,
            9,
            FeatureProfile::Char25V2,
            NormalizationProfile::JapaneseV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::De,
            10,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Tr,
            11,
            FeatureProfile::TurkishChar35V3,
            NormalizationProfile::TurkishV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Vi,
            12,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::VietnameseV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::Ko,
            13,
            FeatureProfile::KoreanWordChar25V3,
            NormalizationProfile::KoreanV2,
            FeatureSchema::SparseV2,
        ),
        (
            Language::It,
            14,
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            FeatureSchema::SparseV2,
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
        (FeatureProfile::EsLegacyWordChar35V1, "EsLegacyWordChar35V1"),
        (FeatureProfile::WordChar35V2, "WordChar35V2"),
        (FeatureProfile::Char25V2, "Char25V2"),
        (FeatureProfile::TurkishChar35V3, "TurkishChar35V3"),
        (
            FeatureProfile::ChineseScriptChar15V3,
            "ChineseScriptChar15V3",
        ),
        (FeatureProfile::KoreanWordChar25V3, "KoreanWordChar25V3"),
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
        (
            NormalizationProfile::EsLegacyCharabiaV1,
            "EsLegacyCharabiaV1",
        ),
        (NormalizationProfile::GenericV2, "GenericV2"),
        (NormalizationProfile::TurkishV2, "TurkishV2"),
        (NormalizationProfile::VietnameseV2, "VietnameseV2"),
        (NormalizationProfile::ArabicV2, "ArabicV2"),
        (NormalizationProfile::HindiV2, "HindiV2"),
        (NormalizationProfile::ChineseV2, "ChineseV2"),
        (NormalizationProfile::JapaneseV2, "JapaneseV2"),
        (NormalizationProfile::KoreanV2, "KoreanV2"),
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
        (FeatureSchema::EsLegacyV1, "EsLegacyV1"),
        (FeatureSchema::SparseV2, "SparseV2"),
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
