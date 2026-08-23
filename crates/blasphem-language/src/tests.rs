use super::{Detector, Language, ModelError, extract_features};

const HEADER_LEN: usize = 76;

fn valid_model() -> Vec<u8> {
    let table_len = 2_u32;
    let blob_len = 0_u32;
    let mut bytes = Vec::with_capacity(
        HEADER_LEN + 15 * 4 + 8_192 + 8_192 + 1_920 * 2 + table_len as usize * 8,
    );
    bytes.extend_from_slice(b"BLASPHEM");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&15_u32.to_le_bytes());
    bytes.extend_from_slice(&table_len.to_le_bytes());
    bytes.extend_from_slice(&blob_len.to_le_bytes());
    bytes.extend_from_slice(&8_192_u32.to_le_bytes());
    bytes.extend_from_slice(&8_192_u32.to_le_bytes());
    bytes.extend_from_slice(&1_920_u32.to_le_bytes());
    bytes.extend_from_slice(b"a0301db809ff2e48a418018aa5359fb0c4354eb8");
    for _ in 0..15 {
        bytes.extend_from_slice(&0.9_f32.to_bits().to_le_bytes());
    }
    bytes.resize(bytes.len() + 8_192 + 8_192 + 1_920 * 2, 0);
    bytes.resize(bytes.len() + table_len as usize * 8, 0);
    bytes
}

fn scored_model(scores_per_feature: &[u32]) -> Vec<u8> {
    let table_len = 8_u32;
    let blob_len = 3 * scores_per_feature.len() as u32;
    let mut bytes = Vec::with_capacity(
        HEADER_LEN
            + 15 * 4
            + 8_192
            + 8_192
            + 1_920 * 2
            + table_len as usize * 8
            + blob_len as usize * 4,
    );
    bytes.extend_from_slice(b"BLASPHEM");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&15_u32.to_le_bytes());
    bytes.extend_from_slice(&table_len.to_le_bytes());
    bytes.extend_from_slice(&blob_len.to_le_bytes());
    bytes.extend_from_slice(&8_192_u32.to_le_bytes());
    bytes.extend_from_slice(&8_192_u32.to_le_bytes());
    bytes.extend_from_slice(&1_920_u32.to_le_bytes());
    bytes.extend_from_slice(b"a0301db809ff2e48a418018aa5359fb0c4354eb8");
    for _ in 0..15 {
        bytes.extend_from_slice(&0.9_f32.to_bits().to_le_bytes());
    }
    bytes.resize(bytes.len() + 8_192 + 8_192 + 1_920 * 2, 0);

    let mut slots = [(0_u32, 0_u32); 8];
    for (feature_index, (bucket, fingerprint)) in
        [(1, 0x4bbc_e23a), (6, 0xa404_4f5b), (3, 0xfc4b_bc73)]
            .into_iter()
            .enumerate()
    {
        let offset = feature_index * scores_per_feature.len();
        slots[bucket] = (
            fingerprint,
            ((scores_per_feature.len() as u32) << 24) | offset as u32,
        );
    }
    for (fingerprint, metadata) in slots {
        bytes.extend_from_slice(&fingerprint.to_le_bytes());
        bytes.extend_from_slice(&metadata.to_le_bytes());
    }
    for _ in 0..3 {
        for score in scores_per_feature {
            bytes.extend_from_slice(&score.to_le_bytes());
        }
    }
    bytes
}

fn packed(bytes: &[u8]) -> u64 {
    let mut feature = [0_u8; 8];
    feature[..bytes.len()].copy_from_slice(bytes);
    u64::from_le_bytes(feature)
}

#[test]
fn language_codes_follow_the_compact_language_order() {
    let languages = [
        Language::Arabic,
        Language::German,
        Language::English,
        Language::Spanish,
        Language::French,
        Language::Hindi,
        Language::Italian,
        Language::Japanese,
        Language::Korean,
        Language::Malay,
        Language::Portuguese,
        Language::Russian,
        Language::Turkish,
        Language::Vietnamese,
        Language::Chinese,
    ];
    let codes: Vec<_> = languages.into_iter().map(Language::code).collect();
    assert_eq!(
        codes,
        [
            "ar", "de", "en", "es", "fr", "hi", "it", "ja", "ko", "ms", "pt", "ru", "tr", "vi",
            "zh",
        ]
    );
}

#[test]
fn model_parser_rejects_each_header_contract_break() {
    let cases = [
        (8, 0_u32, ModelError::UnsupportedVersion(0)),
        (12, 14_u32, ModelError::InvalidLanguageCount(14)),
        (16, 3_u32, ModelError::InvalidTableLength(3)),
        (24, 8_191_u32, ModelError::InvalidLetterTableLength(8_191)),
        (28, 8_191_u32, ModelError::InvalidCjkTableLength(8_191)),
        (
            32,
            1_919_u32,
            ModelError::InvalidLowercaseTableLength(1_919),
        ),
    ];

    let mut bad_magic = valid_model();
    bad_magic[0] = b'X';
    assert_eq!(
        Detector::from_bytes(&bad_magic).unwrap_err(),
        ModelError::InvalidMagic
    );

    for (offset, value, expected) in cases {
        let mut bytes = valid_model();
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert_eq!(Detector::from_bytes(&bytes).unwrap_err(), expected);
    }
}

#[test]
fn model_parser_rejects_truncation_and_trailing_bytes() {
    let bytes = valid_model();
    assert_eq!(
        Detector::from_bytes(&bytes[..bytes.len() - 1]).unwrap_err(),
        ModelError::Truncated
    );

    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        Detector::from_bytes(&trailing).unwrap_err(),
        ModelError::TrailingData
    );
}

#[test]
fn model_parser_rejects_a_blob_range_outside_the_blob() {
    let mut bytes = valid_model();
    let table_offset = HEADER_LEN + 15 * 4 + 8_192 + 8_192 + 1_920 * 2;
    bytes[table_offset..table_offset + 4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[table_offset + 4..table_offset + 8].copy_from_slice(&(1_u32 << 24).to_le_bytes());
    assert_eq!(
        Detector::from_bytes(&bytes).unwrap_err(),
        ModelError::InvalidBlobRange { slot: 0 }
    );
}

#[test]
fn empty_and_punctuation_text_have_no_features() {
    assert!(extract_features("").is_empty());
    assert!(extract_features(" .,! ").is_empty());
}

#[test]
fn ascii_case_uses_the_same_little_endian_feature() {
    let expected = packed(b" hello \0");
    assert_eq!(extract_features("HELLO"), vec![expected]);
    assert_eq!(extract_features("hello"), vec![expected]);
}

#[test]
fn internal_apostrophes_remain_inside_words() {
    assert_eq!(extract_features("don't"), vec![packed(b" don't \0")]);
    assert_eq!(extract_features("don`t"), vec![packed(b" don`t \0")]);
    assert_eq!(
        extract_features("don\u{2019}t"),
        vec![packed(b" don\xE2\x80\x99\0"), packed(b"on\xE2\x80\x99t \0")]
    );
}

#[test]
fn cjk_characters_are_individual_features() {
    assert_eq!(
        extract_features("日本"),
        vec![
            packed(b" \xE6\x97\xA5 \0\0\0"),
            packed(b" \xE6\x9C\xAC \0\0\0")
        ]
    );
}

#[test]
fn nul_and_duplicate_features_stop_or_deduplicate_input() {
    let hello = packed(b" hello \0");
    assert_eq!(extract_features("hello\0bonjour"), vec![hello]);
    assert_eq!(extract_features("hello hello HELLO"), vec![hello]);
}

#[test]
fn long_words_use_six_byte_boundary_chunks() {
    assert_eq!(
        extract_features("abcdefghijklmn"),
        vec![
            packed(b" abcdef\0"),
            packed(b"ghijkl\0\0"),
            packed(b"ijklmn \0"),
        ]
    );
}

#[test]
fn input_stops_at_1000_bytes_and_drops_a_split_codepoint() {
    let mut ascii = ".".repeat(998);
    ascii.push_str("AZignored");
    assert_eq!(extract_features(&ascii), vec![packed(b" az \0\0\0\0")]);

    let mut split = ".".repeat(999);
    split.push('\u{00e9}');
    assert!(extract_features(&split).is_empty());
}

#[test]
fn detector_empty_result_is_unknown_and_unreliable() {
    let detector = Detector::from_bytes(&valid_model()).unwrap();
    let detection = detector.detect("");
    assert_eq!(detection.language, None);
    assert!(!detection.reliable);
    assert_eq!(detection.top_score, 0.0);
    assert_eq!(detection.second_score, 0.0);
    assert_eq!(detection.feature_count, 0);
    assert!(detection.ranked_scores.is_empty());
}

#[test]
fn detector_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Detector>();
}

#[test]
fn reliability_uses_feature_average_and_gap_gates() {
    let high_weight = 100_000_f32.to_bits() & 0xffff_ff00;
    let strong = Detector::from_bytes(&scored_model(&[high_weight | 2])).unwrap();
    let short = strong.detect("a");
    assert_eq!(short.language, Some(Language::English));
    assert!(!short.reliable);

    let reliable = strong.detect("a b c");
    assert!(reliable.reliable);
    assert!((reliable.top_score - 0.999_954_6).abs() < 0.000_000_1);

    let low_weight = 1_000_f32.to_bits() & 0xffff_ff00;
    let weak = Detector::from_bytes(&scored_model(&[low_weight | 2])).unwrap();
    assert!(!weak.detect("a b c").reliable);

    let tied = Detector::from_bytes(&scored_model(&[high_weight | 2, high_weight | 3])).unwrap();
    let tied_result = tied.detect("a b c");
    assert_eq!(tied_result.language, Some(Language::English));
    assert_eq!(tied_result.ranked_scores[1].language, Language::Spanish);
    assert!(!tied_result.reliable);
}

#[test]
fn embedded_model_detects_representative_selected_languages() {
    let detector = Detector::new().unwrap();
    let cases = [
        (
            "This language detector reads a complete English sentence with common words.",
            Language::English,
        ),
        (
            "Este detector de idioma lee una oración completa en español con palabras comunes.",
            Language::Spanish,
        ),
        ("这是一个包含常用词语的完整中文句子。", Language::Chinese),
    ];
    for (text, expected) in cases {
        let detection = detector.detect(text);
        assert_eq!(detection.language, Some(expected));
        assert!(detection.top_score >= detection.second_score);
        assert!(!detection.ranked_scores.is_empty());
    }
}

#[test]
fn the_committed_artifact_uses_the_blasphem_magic() {
    let bytes = include_bytes!("../data/blasphem-language-15-v1.bin");
    assert_eq!(&bytes[..8], b"BLASPHEM");
}
