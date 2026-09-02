use std::collections::BTreeSet;

use thiserror::Error;
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_script::{Script, UnicodeScript};

use crate::{
    FeatureProfile, NormalizationError, NormalizationProfile, normalize_text, normalize_v2,
};

const BIN_COUNT: usize = 65_536;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FeatureError {
    #[error(transparent)]
    Normalization(#[from] NormalizationError),
    #[error("feature profile {feature:?} cannot use normalization profile {normalization:?}")]
    ProfileMismatch {
        feature: FeatureProfile,
        normalization: NormalizationProfile,
    },
}

pub(crate) fn es_legacy_feature_bins(text: &str) -> Vec<usize> {
    let normalized = normalize_text(text);
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    let mut bins = BTreeSet::new();

    for word in &words {
        bins.insert(feature_hash(b'W', 1, [word.as_bytes()]) & (BIN_COUNT - 1));
        let mut characters = Vec::with_capacity(word.chars().count() + 2);
        characters.push('\u{2}');
        characters.extend(word.chars());
        characters.push('\u{3}');
        for length in 3..=5 {
            for gram in characters.windows(length) {
                bins.insert(character_feature_hash(length as u8, gram) & (BIN_COUNT - 1));
            }
        }
    }
    for pair in words.windows(2) {
        bins.insert(
            feature_hash(b'W', 2, [pair[0].as_bytes(), pair[1].as_bytes()]) & (BIN_COUNT - 1),
        );
    }

    bins.into_iter().collect()
}

fn feature_hash<'a>(namespace: u8, arity: u8, parts: impl IntoIterator<Item = &'a [u8]>) -> usize {
    let mut hash = FNV_OFFSET;
    update_hash(&mut hash, &[namespace, arity]);
    for part in parts {
        update_hash(&mut hash, &[0]);
        update_hash(&mut hash, part);
    }
    hash as usize
}

fn character_feature_hash(length: u8, characters: &[char]) -> usize {
    let mut hash = FNV_OFFSET;
    update_hash(&mut hash, &[b'C', length]);
    for character in characters {
        let mut buffer = [0_u8; 4];
        update_hash(&mut hash, character.encode_utf8(&mut buffer).as_bytes());
    }
    hash as usize
}

fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

pub fn extract_feature_bins(
    feature: FeatureProfile,
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    match (feature, normalization) {
        (FeatureProfile::EsLegacyWordChar35V1, NormalizationProfile::EsLegacyCharabiaV1) => {
            Ok(es_legacy_feature_bins(text))
        }
        (
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2
            | NormalizationProfile::TurkishV2
            | NormalizationProfile::VietnameseV2
            | NormalizationProfile::ArabicV2
            | NormalizationProfile::HindiV2,
        ) => word_char_35(normalization, text),
        (
            FeatureProfile::Char25V2,
            NormalizationProfile::ChineseV2
            | NormalizationProfile::JapaneseV2
            | NormalizationProfile::KoreanV2,
        ) => compact_char_25(normalization, text),
        _ => Err(FeatureError::ProfileMismatch {
            feature,
            normalization,
        }),
    }
}

struct NormalizedToken {
    text: String,
    clause: u32,
}

fn word_tokens(
    profile: NormalizationProfile,
    text: &str,
) -> Result<Vec<NormalizedToken>, FeatureError> {
    let normalized = normalize_v2(profile, text)?;
    let characters = normalized.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut clause = 0_u32;

    for (index, character) in characters.iter().copied().enumerate() {
        if is_word_character(character) || is_hindi_joiner(&characters, index, profile) {
            current.push(character);
            continue;
        }
        if !current.is_empty() {
            tokens.push(NormalizedToken {
                text: std::mem::take(&mut current),
                clause,
            });
        }
        if is_clause_boundary(character) {
            clause = clause.saturating_add(1);
        }
    }
    if !current.is_empty() {
        tokens.push(NormalizedToken {
            text: current,
            clause,
        });
    }
    Ok(tokens)
}

fn is_word_character(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::DecimalNumber
            | GeneralCategory::LetterNumber
            | GeneralCategory::OtherNumber
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    )
}

fn is_hindi_joiner(characters: &[char], index: usize, profile: NormalizationProfile) -> bool {
    if profile != NormalizationProfile::HindiV2
        || !matches!(characters[index], '\u{200c}' | '\u{200d}')
    {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index + 1);
    previous.is_some_and(|character| {
        is_word_character(*character) && character.script() == Script::Devanagari
    }) && next.is_some_and(|character| {
        is_word_character(*character) && character.script() == Script::Devanagari
    })
}

fn is_clause_boundary(character: char) -> bool {
    matches!(
        character,
        '.' | '!'
            | '?'
            | ';'
            | ':'
            | '。'
            | '！'
            | '？'
            | '；'
            | '：'
            | '؟'
            | '؛'
            | '।'
            | '\n'
            | '\r'
    )
}

fn word_char_35(
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    let tokens = word_tokens(normalization, text)?;
    let mut bins = BTreeSet::new();
    for token in &tokens {
        bins.insert(feature_hash(b'W', 1, [token.text.as_bytes()]) & (BIN_COUNT - 1));
        emit_character_grams(&token.text.chars().collect::<Vec<_>>(), 3, 5, |_, bin| {
            bins.insert(bin);
        });
    }
    for pair in tokens
        .windows(2)
        .filter(|pair| pair[0].clause == pair[1].clause)
    {
        bins.insert(
            feature_hash(b'W', 2, [pair[0].text.as_bytes(), pair[1].text.as_bytes()])
                & (BIN_COUNT - 1),
        );
    }
    Ok(bins.into_iter().collect())
}

fn compact_char_25(
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    let mut bins = BTreeSet::new();
    compact_char_25_with(normalization, text, |_, bin| {
        bins.insert(bin);
    })?;
    Ok(bins.into_iter().collect())
}

fn compact_char_25_with(
    normalization: NormalizationProfile,
    text: &str,
    mut emit: impl FnMut(u8, usize),
) -> Result<(), FeatureError> {
    let normalized = normalize_v2(normalization, text)?;
    let mut segment = Vec::new();
    for character in normalized.chars() {
        if is_compact_boundary(character) {
            emit_character_grams(&segment, 2, 5, &mut emit);
            segment.clear();
        } else if !character.is_whitespace() {
            segment.push(character);
        }
    }
    emit_character_grams(&segment, 2, 5, emit);
    Ok(())
}

fn emit_character_grams(
    content: &[char],
    minimum: usize,
    maximum: usize,
    mut emit: impl FnMut(u8, usize),
) {
    if content.is_empty() {
        return;
    }
    let mut characters = Vec::with_capacity(content.len() + 2);
    characters.push('\u{2}');
    characters.extend_from_slice(content);
    characters.push('\u{3}');
    for length in minimum..=maximum {
        for gram in characters.windows(length) {
            emit(
                b'C',
                character_feature_hash(length as u8, gram) & (BIN_COUNT - 1),
            );
        }
    }
}

fn is_compact_boundary(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
            | GeneralCategory::OtherSymbol
            | GeneralCategory::LineSeparator
            | GeneralCategory::ParagraphSeparator
            | GeneralCategory::Control
            | GeneralCategory::Format
    )
}

#[cfg(test)]
mod tests {
    use crate::{FeatureProfile, NormalizationProfile};

    use super::{compact_char_25_with, es_legacy_feature_bins, extract_feature_bins, word_tokens};

    #[test]
    fn spanish_feature_bins_match_frozen_tables_after_move() {
        let cases: &[(&str, &[usize])] = &[
            ("tox", &[1722, 1731, 8133, 26526, 42498, 44885, 64854]),
            (
                "eres basura",
                &[
                    173, 1571, 4768, 7139, 8537, 9657, 10926, 13214, 15622, 16407, 16691, 18105,
                    24303, 29095, 29407, 29533, 31647, 33951, 37144, 40126, 41186, 46864, 48597,
                    50768, 54925, 57782, 63971,
                ],
            ),
        ];

        for &(text, expected) in cases {
            let actual = es_legacy_feature_bins(text);
            assert_eq!(actual.as_slice(), expected);
        }
    }

    #[test]
    fn feature_profiles_match_exact_bin_tables() {
        let cases: &[(FeatureProfile, NormalizationProfile, &str, &[usize])] = &[
            (
                FeatureProfile::EsLegacyWordChar35V1,
                NormalizationProfile::EsLegacyCharabiaV1,
                "tox",
                &[1722, 1731, 8133, 26526, 42498, 44885, 64854],
            ),
            (
                FeatureProfile::EsLegacyWordChar35V1,
                NormalizationProfile::EsLegacyCharabiaV1,
                "eres basura",
                &[
                    173, 1571, 4768, 7139, 8537, 9657, 10926, 13214, 15622, 16407, 16691, 18105,
                    24303, 29095, 29407, 29533, 31647, 33951, 37144, 40126, 41186, 46864, 48597,
                    50768, 54925, 57782, 63971,
                ],
            ),
            (
                FeatureProfile::WordChar35V2,
                NormalizationProfile::GenericV2,
                "ab cd. ef",
                &[
                    3680, 10476, 13789, 21170, 23008, 35036, 36904, 36952, 40269, 43645, 45500,
                    45548, 59368,
                ],
            ),
            (
                FeatureProfile::Char25V2,
                NormalizationProfile::ChineseV2,
                "你 去死。",
                &[
                    1283, 1579, 15489, 22698, 26691, 32640, 47167, 50706, 51814, 59498,
                ],
            ),
        ];

        for &(feature, normalization, text, expected) in cases {
            let actual = extract_feature_bins(feature, normalization, text).expect("features");
            assert_eq!(actual.as_slice(), expected);
        }
    }

    #[test]
    fn char_profile_emits_only_character_namespace_events() {
        let mut namespaces = Vec::new();
        compact_char_25_with(
            NormalizationProfile::ChineseV2,
            "你 去死。",
            |namespace, _| namespaces.push(namespace),
        )
        .expect("features");

        assert!(!namespaces.is_empty());
        assert!(namespaces.iter().all(|namespace| *namespace == b'C'));
    }

    #[test]
    fn hindi_joiner_rejects_devanagari_punctuation_neighbors() {
        let cases = [("\u{0970}\u{200d}क", "क"), ("क\u{200d}\u{0970}", "क")];

        for (text, expected) in cases {
            let tokens = word_tokens(NormalizationProfile::HindiV2, text).expect("tokens");
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].text, expected);
        }
    }
}
