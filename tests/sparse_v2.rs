use blasphem::{
    FeatureProfile, FeatureSchema, Language, NormalizationProfile, SparseModel, SparseModelError,
    SparseV2Input, encode_sparse_v2,
};
use std::ops::Range;

fn fixture_v2_input(language: Language, weights: &[i16]) -> SparseV2Input<'_> {
    let (feature_profile, normalization_profile, feature_schema) = language.profiles();
    SparseV2Input {
        language,
        feature_profile,
        normalization_profile,
        feature_schema,
        bias: -64,
        decision_boundary: 128,
        score_scale: 256,
        max_false_warning_basis_points: 300,
        weights,
    }
}

#[test]
fn v1_infers_only_the_spanish_legacy_profiles() {
    let model = SparseModel::from_bytes(include_bytes!("../resources/models/es-chargram-v1.bin"))
        .expect("Spanish model");
    assert_eq!(model.language(), Language::Es);
    assert_eq!(
        model.feature_profile(),
        FeatureProfile::EsLegacyWordChar35V1
    );
    assert_eq!(
        model.normalization_profile(),
        NormalizationProfile::EsLegacyCharabiaV1
    );
    assert_eq!(model.feature_schema(), FeatureSchema::EsLegacyV1);
}

#[test]
fn v2_round_trip_preserves_every_header_field() {
    let weights = vec![0_i16; 65_536];
    let input = fixture_v2_input(Language::Tr, &weights);
    let artifact = encode_sparse_v2(&input).expect("encode");
    assert_eq!(artifact.len(), 131_112);
    let model = SparseModel::from_bytes(&artifact).expect("parse");
    assert_eq!(model.language(), Language::Tr);
    assert_eq!(model.feature_profile(), FeatureProfile::WordChar35V2);
    assert_eq!(
        model.normalization_profile(),
        NormalizationProfile::TurkishV2
    );
    assert_eq!(model.feature_schema(), FeatureSchema::SparseV2);
    assert_eq!(model.raw_score(""), -64);
    assert_eq!(model.raw_boundary(), 128);
    assert_eq!(model.score_scale(), 256);
    assert_eq!(model.max_false_warning_basis_points(), 300);
}

#[test]
fn v2_encoder_rejects_spanish_and_invalid_calibration() {
    let weights = vec![0_i16; 65_536];

    let spanish = fixture_v2_input(Language::Es, &weights);
    assert_eq!(
        encode_sparse_v2(&spanish),
        Err(SparseModelError::VersionTwoSpanish)
    );

    let mut invalid_scale = fixture_v2_input(Language::Tr, &weights);
    invalid_scale.score_scale = 0;
    assert_eq!(
        encode_sparse_v2(&invalid_scale),
        Err(SparseModelError::ZeroScoreScale)
    );

    let mut invalid_limit = fixture_v2_input(Language::Tr, &weights);
    invalid_limit.max_false_warning_basis_points = 10_001;
    assert_eq!(
        encode_sparse_v2(&invalid_limit),
        Err(SparseModelError::InvalidFalseWarningLimit(10_001))
    );
}

type ErrorCheck = fn(&SparseModelError) -> bool;

#[test]
fn v2_rejects_each_invalid_header_field() {
    let weights = vec![0_i16; 65_536];
    let input = fixture_v2_input(Language::Tr, &weights);
    let artifact = encode_sparse_v2(&input).expect("encode");
    let cases: &[(&str, Range<usize>, &[u8], ErrorCheck)] = &[
        ("magic", 0..8, b"BADMAGIC", |error| {
            matches!(error, &SparseModelError::InvalidMagic)
        }),
        ("version", 8..10, &[1, 0], |error| {
            matches!(error, &SparseModelError::UnsupportedVersion(1))
        }),
        ("language", 10..12, b"XX", |error| {
            matches!(error, &SparseModelError::InvalidLanguage)
        }),
        ("lowercase language", 10..12, b"tr", |error| {
            matches!(error, &SparseModelError::InvalidLanguage)
        }),
        ("bin count", 12..16, &[0, 0, 0, 0], |error| {
            matches!(error, &SparseModelError::InvalidBinCount(0))
        }),
        ("score scale", 24..28, &[0, 0, 0, 0], |error| {
            matches!(error, &SparseModelError::ZeroScoreScale)
        }),
        ("false-warning limit", 28..30, &[17, 39], |error| {
            matches!(error, &SparseModelError::InvalidFalseWarningLimit(10_001))
        }),
        ("weight scale", 30..32, &[0, 0], |error| {
            matches!(error, &SparseModelError::InvalidWeightScale(0))
        }),
        ("feature profile", 32..33, &[255], |error| {
            matches!(error, &SparseModelError::InvalidFeatureProfile(255))
        }),
        ("normalization profile", 33..34, &[255], |error| {
            matches!(error, &SparseModelError::InvalidNormalizationProfile(255))
        }),
        ("feature schema", 34..36, &[255, 255], |error| {
            matches!(error, &SparseModelError::InvalidFeatureSchema(65_535))
        }),
        ("payload length", 36..40, &[0, 0, 0, 0], |error| {
            matches!(error, &SparseModelError::InvalidPayloadLength(0))
        }),
        ("language profile", 32..33, &[3], |error| {
            matches!(error, &SparseModelError::ProfileMismatch)
        }),
    ];

    for (name, range, replacement, check) in cases {
        let mut damaged = artifact.clone();
        damaged[range.clone()].copy_from_slice(replacement);
        let error = SparseModel::from_bytes(&damaged).expect_err(name);
        assert!(check(&error), "{name}: {error}");
    }
}

#[test]
fn v2_rejects_spanish_and_nonexact_payload_sizes() {
    let weights = vec![0_i16; 65_536];
    let input = fixture_v2_input(Language::Tr, &weights);
    let artifact = encode_sparse_v2(&input).expect("encode");

    let mut spanish = artifact.clone();
    spanish[10..12].copy_from_slice(b"ES");
    assert_eq!(
        SparseModel::from_bytes(&spanish),
        Err(SparseModelError::VersionTwoSpanish)
    );

    let truncated = &artifact[..artifact.len() - 1];
    assert!(matches!(
        SparseModel::from_bytes(truncated),
        Err(SparseModelError::InvalidLength {
            expected: 131_112,
            actual: 131_111
        })
    ));

    let mut extended = artifact;
    extended.push(0);
    assert!(matches!(
        SparseModel::from_bytes(&extended),
        Err(SparseModelError::InvalidLength {
            expected: 131_112,
            actual: 131_113
        })
    ));
}
