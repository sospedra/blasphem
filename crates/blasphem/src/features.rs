use std::{collections::BTreeSet, ops::RangeInclusive};

use thiserror::Error;
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_script::{Script, UnicodeScript};

use crate::{FeatureProfile, NormalizationProfile, normalize_text, normalize_v2};

const BIN_COUNT: usize = 65_536;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FeatureError {
    #[error("feature profile {feature:?} cannot use normalization profile {normalization:?}")]
    ProfileMismatch {
        feature: FeatureProfile,
        normalization: NormalizationProfile,
    },
}

pub(crate) fn spanish_feature_bins(text: &str) -> Vec<usize> {
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
    character_feature_hash_in(b'C', length, characters)
}

fn character_feature_hash_in(namespace: u8, length: u8, characters: &[char]) -> usize {
    let mut hash = FNV_OFFSET;
    update_hash(&mut hash, &[namespace, length]);
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
        (FeatureProfile::SpanishWordChar35, NormalizationProfile::SpanishCharabia) => {
            Ok(spanish_feature_bins(text))
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
        (FeatureProfile::TurkishChar35V3, NormalizationProfile::TurkishV2) => {
            token_char(normalization, text, 3..=5)
        }
        (FeatureProfile::ChineseScriptChar15V3, NormalizationProfile::ChineseV2) => {
            chinese_script_char_15(normalization, text)
        }
        (FeatureProfile::KoreanWordChar25V3, NormalizationProfile::KoreanV2) => {
            korean_word_char_25(normalization, text)
        }
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
    let normalized = normalize_v2(profile, text);
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

fn token_char(
    normalization: NormalizationProfile,
    text: &str,
    lengths: RangeInclusive<usize>,
) -> Result<Vec<usize>, FeatureError> {
    let tokens = word_tokens(normalization, text)?;
    let mut bins = BTreeSet::new();
    for token in &tokens {
        emit_character_grams(
            &token.text.chars().collect::<Vec<_>>(),
            *lengths.start(),
            *lengths.end(),
            |_, bin| {
                bins.insert(bin);
            },
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

fn korean_word_char_25(
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    let mut bins = compact_char_25(normalization, text)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    for token in word_tokens(normalization, text)? {
        bins.insert(feature_hash(b'W', 1, [token.text.as_bytes()]) & (BIN_COUNT - 1));
    }
    Ok(bins.into_iter().collect())
}

fn chinese_script_char_15(
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    let normalized = normalize_v2(normalization, text);
    let mut bins = BTreeSet::new();
    let mut segment = Vec::new();
    for character in normalized.chars() {
        if is_compact_boundary(character) {
            emit_chinese_character_grams(&segment, |_, bin| {
                bins.insert(bin);
            });
            segment.clear();
            continue;
        }
        if !character.is_whitespace() {
            segment.push(character);
        }
    }
    emit_chinese_character_grams(&segment, |_, bin| {
        bins.insert(bin);
    });
    Ok(bins.into_iter().collect())
}

fn emit_chinese_character_grams(content: &[char], mut emit: impl FnMut(u8, usize)) {
    if content.is_empty() {
        return;
    }
    let mut characters = Vec::with_capacity(content.len() + 2);
    characters.push('\u{2}');
    characters.extend_from_slice(content);
    characters.push('\u{3}');
    for length in 1..=5 {
        for gram in characters.windows(length) {
            let namespace = chinese_script_namespace(gram);
            if length == 1 && namespace != b'H' {
                continue;
            }
            emit(
                namespace,
                character_feature_hash_in(namespace, length as u8, gram) & (BIN_COUNT - 1),
            );
        }
    }
}

fn chinese_script_namespace(characters: &[char]) -> u8 {
    let mut script = None;
    for character in characters
        .iter()
        .filter(|character| !matches!(character, '\u{2}' | '\u{3}'))
    {
        let candidate = match character.script() {
            Script::Han => b'H',
            Script::Latin => b'L',
            _ => return b'C',
        };
        if script.is_some_and(|value| value != candidate) {
            return b'C';
        }
        script = Some(candidate);
    }
    script.unwrap_or(b'C')
}

fn compact_char_25_with(
    normalization: NormalizationProfile,
    text: &str,
    mut emit: impl FnMut(u8, usize),
) -> Result<(), FeatureError> {
    let normalized = normalize_v2(normalization, text);
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
    use std::collections::BTreeSet;

    use crate::{FeatureProfile, Language, NormalizationProfile};

    use super::{
        compact_char_25_with, emit_chinese_character_grams, extract_feature_bins,
        spanish_feature_bins, word_tokens,
    };

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
            let actual = spanish_feature_bins(text);
            assert_eq!(actual.as_slice(), expected);
        }
    }

    #[test]
    fn feature_profiles_match_exact_bin_tables() {
        let cases: &[(FeatureProfile, NormalizationProfile, &str, &[usize])] = &[
            (
                FeatureProfile::SpanishWordChar35,
                NormalizationProfile::SpanishCharabia,
                "tox",
                &[1722, 1731, 8133, 26526, 42498, 44885, 64854],
            ),
            (
                FeatureProfile::SpanishWordChar35,
                NormalizationProfile::SpanishCharabia,
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
            (
                FeatureProfile::ChineseScriptChar15V3,
                NormalizationProfile::ChineseV2,
                "你a",
                &[6375, 27751, 30612, 31883, 44600, 48369, 55720],
            ),
            (
                FeatureProfile::TurkishChar35V3,
                NormalizationProfile::TurkishV2,
                "aptal",
                &[
                    14770, 22416, 23221, 23671, 24107, 32100, 32515, 39396, 39410, 42525, 63138,
                    63143,
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
    fn korean_profile_keeps_character_grams_and_adds_word_boundaries() {
        let (feature, normalization, _) = Language::Ko.profiles();
        let spaced = extract_feature_bins(feature, normalization, "가 나").expect("features");
        let joined = extract_feature_bins(feature, normalization, "가나").expect("features");
        let legacy = extract_feature_bins(
            FeatureProfile::Char25V2,
            NormalizationProfile::KoreanV2,
            "가 나",
        )
        .expect("legacy features");

        assert!(legacy.iter().all(|bin| spaced.contains(bin)));
        assert_ne!(spaced, joined);
    }

    #[test]
    fn chinese_script_profile_scores_one_han_character() {
        let bins = extract_feature_bins(
            FeatureProfile::ChineseScriptChar15V3,
            NormalizationProfile::ChineseV2,
            "你",
        )
        .expect("features");

        assert!(!bins.is_empty());
    }

    #[test]
    fn chinese_script_profile_separates_han_and_latin_grams() {
        let mut namespaces = BTreeSet::new();
        emit_chinese_character_grams(&['你', 'a'], |namespace, _| {
            namespaces.insert(namespace);
        });

        assert_eq!(namespaces, BTreeSet::from(*b"CHL"));
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
