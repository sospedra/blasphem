use super::slice::{SliceDetector, SliceError, write_slices};
use super::{Detector, Language, ModelError, extract_features, h64};

const HEADER_LEN: usize = 76;
const TABLE_LEN: usize = 64;
const TABLES_LEN: usize = 15 * 4 + 8_192 + 8_192 + 1_920 * 2;
const BITMAPS_OFFSET: usize = HEADER_LEN + TABLES_LEN;

/// A version-two header and zeroed unicode tables.
fn header(table_len: u32, blob_len: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BITMAPS_OFFSET + 16);
    bytes.extend_from_slice(b"BLASPHEM");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
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
    bytes.resize(BITMAPS_OFFSET, 0);
    bytes
}

fn with_bitmaps(mut bytes: Vec<u8>, occupied: u64, live: u64) -> Vec<u8> {
    bytes.extend_from_slice(&occupied.to_le_bytes());
    bytes.extend_from_slice(&live.to_le_bytes());
    bytes
}

fn with_entry(mut bytes: Vec<u8>, fingerprint: u32, metadata: u32) -> Vec<u8> {
    bytes.extend_from_slice(&fingerprint.to_le_bytes());
    bytes.extend_from_slice(&metadata.to_le_bytes());
    bytes
}

fn with_scores(mut bytes: Vec<u8>, scores: &[u32]) -> Vec<u8> {
    for score in scores {
        bytes.extend_from_slice(&score.to_le_bytes());
    }
    bytes
}

/// An empty 64-position table.
fn valid_model() -> Vec<u8> {
    with_bitmaps(header(TABLE_LEN as u32, 0), 0, 0)
}

/// The home bucket and fingerprint of one single-word feature.
fn placement(word: &str) -> (usize, u32) {
    let hash = h64(extract_features(word)[0]);
    (
        (hash as u32 as usize) % TABLE_LEN,
        ((hash >> 32) as u32).max(1),
    )
}

/// One model where the words `a`, `b`, and `c` each score `scores_per_feature`.
fn scored_model(scores_per_feature: &[u32]) -> Vec<u8> {
    let mut placed: Vec<_> = ["a", "b", "c"].into_iter().map(placement).collect();
    placed.sort_unstable();
    assert!(
        placed.windows(2).all(|pair| pair[0].0 != pair[1].0),
        "the test words must not share a bucket"
    );

    let count = scores_per_feature.len() as u32;
    let bits = placed
        .iter()
        .fold(0_u64, |bits, (bucket, _)| bits | (1 << bucket));
    let mut bytes = with_bitmaps(header(TABLE_LEN as u32, 3 * count), bits, bits);
    for (index, (_, fingerprint)) in placed.iter().enumerate() {
        bytes = with_entry(bytes, *fingerprint, (count << 24) | (index as u32 * count));
    }
    for _ in 0..3 {
        bytes = with_scores(bytes, scores_per_feature);
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
        (8, 1_u32, ModelError::UnsupportedVersion(1)),
        (12, 14_u32, ModelError::InvalidLanguageCount(14)),
        (16, 3_u32, ModelError::InvalidTableLength(3)),
        (16, 32_u32, ModelError::InvalidTableLength(32)),
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
    let bytes = with_entry(with_bitmaps(header(TABLE_LEN as u32, 0), 1, 1), 1, 1 << 24);
    assert_eq!(
        Detector::from_bytes(&bytes).unwrap_err(),
        ModelError::InvalidBlobRange { slot: 0 }
    );
}

#[test]
fn model_parser_rejects_a_live_slot_that_is_not_occupied() {
    let bytes = with_entry(
        with_bitmaps(header(TABLE_LEN as u32, 1), 0, 1 << 5),
        1,
        1 << 24,
    );
    let bytes = with_scores(bytes, &[2]);
    assert_eq!(
        Detector::from_bytes(&bytes).unwrap_err(),
        ModelError::InvalidLiveSlot { slot: 5 }
    );
}

#[test]
fn model_parser_rejects_entries_without_a_fingerprint_or_scores() {
    for (fingerprint, metadata) in [(0_u32, 1_u32 << 24), (7, 0)] {
        let bytes = with_entry(
            with_bitmaps(header(TABLE_LEN as u32, 1), 1, 1),
            fingerprint,
            metadata,
        );
        let bytes = with_scores(bytes, &[2]);
        assert_eq!(
            Detector::from_bytes(&bytes).unwrap_err(),
            ModelError::InvalidEntry { slot: 0 }
        );
    }
}

#[test]
fn model_parser_rejects_a_table_without_an_empty_slot() {
    let bytes = with_bitmaps(header(TABLE_LEN as u32, 0), u64::MAX, 0);
    assert_eq!(
        Detector::from_bytes(&bytes).unwrap_err(),
        ModelError::InvalidTableLength(64)
    );
}

#[test]
fn model_parser_rejects_a_score_for_a_language_outside_the_profile_set() {
    let bytes = with_entry(with_bitmaps(header(TABLE_LEN as u32, 1), 1, 1), 1, 1 << 24);
    let bytes = with_scores(bytes, &[15]);
    assert_eq!(
        Detector::from_bytes(&bytes).unwrap_err(),
        ModelError::InvalidLanguageIndex { index: 0 }
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
fn detector_probes_past_a_dead_slot_to_its_live_entry() {
    let (home, fingerprint) = placement("a");
    let next = (home + 1) % TABLE_LEN;
    let occupied = (1_u64 << home) | (1_u64 << next);
    let weight = 100_000_f32.to_bits() & 0xffff_ff00;
    let bytes = with_bitmaps(header(TABLE_LEN as u32, 1), occupied, 1_u64 << next);
    let bytes = with_scores(with_entry(bytes, fingerprint, 1 << 24), &[weight | 2]);

    let detector = Detector::from_bytes(&bytes).unwrap();
    let detection = detector.detect("a");
    assert_eq!(detection.language, Some(Language::English));
    assert_eq!(detection.feature_count, 1);
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
fn the_committed_artifact_uses_the_blasphem_magic_and_version_two() {
    let bytes = include_bytes!("../data/blasphem-language-15-v2.bin");
    assert_eq!(&bytes[..8], b"BLASPHEM");
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2);
}

fn slice_entry_count(slice: &[u8]) -> u32 {
    u32::from_le_bytes(slice[20..24].try_into().expect("four bytes"))
}

#[test]
fn slices_route_like_the_full_table_for_any_subset() {
    // English scores 1.0 and Spanish 0.5 on each of the three placed words.
    let model = scored_model(&[0x3f80_0000 | 2, 0x3f00_0000 | 3]);
    let full = Detector::from_bytes(&model).expect("valid model");
    let slices = write_slices(&model).expect("slices");
    assert_eq!(slices.len(), 15);
    let english = slices[Language::English.index()].1.as_slice();
    let spanish = slices[Language::Spanish.index()].1.as_slice();
    let hindi = slices[Language::Hindi.index()].1.as_slice();
    assert_eq!(slice_entry_count(english), 3);
    assert_eq!(slice_entry_count(spanish), 3);
    assert_eq!(slice_entry_count(hindi), 0);

    let merged = SliceDetector::from_slices(&[english, spanish]).expect("merged slices");
    for text in ["a", "b", "c", "a b c", "zzz", ""] {
        assert_eq!(merged.detect(text), full.detect(text), "{text:?}");
    }

    let english_only = SliceDetector::from_slices(&[english]).expect("one slice");
    let detection = english_only.detect("a b c");
    assert_eq!(detection.language, Some(Language::English));
    assert_eq!(detection.ranked_scores.len(), 1);
    assert_eq!(english_only.languages(), vec![Language::English]);
}

#[test]
fn slice_reader_rejects_broken_slices() {
    let model = scored_model(&[0x3f80_0000 | 2]);
    let english = write_slices(&model)
        .expect("slices")
        .swap_remove(Language::English.index())
        .1;

    let mut swapped = english.clone();
    let first = swapped[68..80].to_vec();
    let second = swapped[80..92].to_vec();
    swapped[68..80].copy_from_slice(&second);
    swapped[80..92].copy_from_slice(&first);
    assert_eq!(
        SliceDetector::from_slices(&[&swapped]).unwrap_err(),
        SliceError::Unsorted { index: 1 }
    );

    let mut truncated = english.clone();
    truncated.pop();
    assert_eq!(
        SliceDetector::from_slices(&[&truncated]).unwrap_err(),
        SliceError::Truncated
    );

    let mut foreign = english.clone();
    foreign[12..14].copy_from_slice(b"xx");
    assert_eq!(
        SliceDetector::from_slices(&[&foreign]).unwrap_err(),
        SliceError::UnknownLanguage(*b"xx")
    );

    assert_eq!(
        SliceDetector::from_slices(&[&english, &english]).unwrap_err(),
        SliceError::DuplicateLanguage(Language::English)
    );
    assert_eq!(
        SliceDetector::from_slices(&[]).unwrap_err(),
        SliceError::Empty
    );
}
