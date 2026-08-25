//! A safe Rust port of the selected Efficient Language Detector core.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

const MAGIC: &[u8; 8] = b"BLASPHEM";
const FORMAT_VERSION: u32 = 1;
const LANGUAGE_COUNT: usize = 15;
const LETTER_TABLE_LEN: usize = 8_192;
const CJK_TABLE_LEN: usize = 8_192;
const LOWERCASE_TABLE_LEN: usize = 1_920;
const SOURCE_COMMIT: &[u8; 40] = b"a0301db809ff2e48a418018aa5359fb0c4354eb8";
const HEADER_LEN: usize = 76;
const MAX_INPUT_BYTES: usize = 1_000;
const MAX_FEATURES: usize = 500;

static EMBEDDED_MODEL: &[u8] = include_bytes!("../data/blasphem-language-15-v1.bin");

/// A language profile in the compact language model.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Language {
    Arabic,
    German,
    English,
    Spanish,
    French,
    Hindi,
    Italian,
    Japanese,
    Korean,
    Malay,
    Portuguese,
    Russian,
    Turkish,
    Vietnamese,
    Chinese,
}

impl Language {
    const ALL: [Self; LANGUAGE_COUNT] = [
        Self::Arabic,
        Self::German,
        Self::English,
        Self::Spanish,
        Self::French,
        Self::Hindi,
        Self::Italian,
        Self::Japanese,
        Self::Korean,
        Self::Malay,
        Self::Portuguese,
        Self::Russian,
        Self::Turkish,
        Self::Vietnamese,
        Self::Chinese,
    ];

    /// Returns the lowercase ISO 639-1 code used by the language model.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Arabic => "ar",
            Self::German => "de",
            Self::English => "en",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::Hindi => "hi",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::Malay => "ms",
            Self::Portuguese => "pt",
            Self::Russian => "ru",
            Self::Turkish => "tr",
            Self::Vietnamese => "vi",
            Self::Chinese => "zh",
        }
    }

    const fn from_index(index: usize) -> Self {
        Self::ALL[index]
    }
}

/// One normalized language score in descending score order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RankedScore {
    pub language: Language,
    pub score: f32,
}

/// The result of one language detection call.
#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    pub language: Option<Language>,
    pub reliable: bool,
    pub top_score: f32,
    pub second_score: f32,
    pub feature_count: usize,
    pub ranked_scores: Vec<RankedScore>,
}

/// A model validation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelError {
    InvalidMagic,
    UnsupportedVersion(u32),
    InvalidLanguageCount(u32),
    InvalidTableLength(u32),
    InvalidLetterTableLength(u32),
    InvalidCjkTableLength(u32),
    InvalidLowercaseTableLength(u32),
    InvalidSourceCommit,
    InvalidEmptySlot { slot: usize },
    InvalidBlobRange { slot: usize },
    InvalidLanguageIndex { index: usize },
    Truncated,
    TrailingData,
}

impl Display for ModelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => formatter.write_str("the language model magic is invalid"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "the language model version {version} is unsupported"
                )
            }
            Self::InvalidLanguageCount(count) => {
                write!(
                    formatter,
                    "the language model language count {count} is invalid"
                )
            }
            Self::InvalidTableLength(length) => {
                write!(
                    formatter,
                    "the language model table length {length} is invalid"
                )
            }
            Self::InvalidLetterTableLength(length) => {
                write!(
                    formatter,
                    "the language model letter table length {length} is invalid"
                )
            }
            Self::InvalidCjkTableLength(length) => {
                write!(
                    formatter,
                    "the language model CJK table length {length} is invalid"
                )
            }
            Self::InvalidLowercaseTableLength(length) => {
                write!(
                    formatter,
                    "the language model lowercase table length {length} is invalid"
                )
            }
            Self::InvalidSourceCommit => {
                formatter.write_str("the language model source commit is invalid")
            }
            Self::InvalidEmptySlot { slot } => {
                write!(
                    formatter,
                    "the language model empty slot {slot} has metadata"
                )
            }
            Self::InvalidBlobRange { slot } => {
                write!(
                    formatter,
                    "the language model slot {slot} has an invalid blob range"
                )
            }
            Self::InvalidLanguageIndex { index } => {
                write!(
                    formatter,
                    "the language model blob item {index} has an invalid language"
                )
            }
            Self::Truncated => formatter.write_str("the language model is truncated"),
            Self::TrailingData => formatter.write_str("the language model has trailing data"),
        }
    }
}

impl Error for ModelError {}

#[derive(Clone, Copy, Debug)]
struct Slot {
    fingerprint: u32,
    metadata: u32,
}

/// An immutable language detector.
#[derive(Debug)]
pub struct Detector {
    averages: [f32; LANGUAGE_COUNT],
    letter_bits: Box<[u8]>,
    cjk_bits: Box<[u8]>,
    lowercase: Box<[u16]>,
    table: Box<[Slot]>,
    blob: Box<[u32]>,
}

impl Detector {
    /// Loads and validates the embedded 15-profile model.
    pub fn new() -> Result<Self, ModelError> {
        Self::from_bytes(EMBEDDED_MODEL)
    }

    /// Loads and validates a model from its documented binary format.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ModelError> {
        parse_model(bytes)
    }

    /// Detects the highest scoring selected language.
    #[must_use]
    pub fn detect(&self, text: &str) -> Detection {
        let features =
            extract_features_with_tables(text, &self.letter_bits, &self.cjk_bits, &self.lowercase);
        if features.is_empty() {
            return Detection {
                language: None,
                reliable: false,
                top_score: 0.0,
                second_score: 0.0,
                feature_count: 0,
                ranked_scores: Vec::new(),
            };
        }

        let mut raw = [1.0_f32; LANGUAGE_COUNT];
        let mask = self.table.len() - 1;
        for feature in &features {
            let hash = h64(*feature);
            let mut index = (hash as u32 as usize) & mask;
            let fingerprint = ((hash >> 32) as u32).max(1);
            while self.table[index].fingerprint != 0 {
                let slot = self.table[index];
                if slot.fingerprint == fingerprint {
                    let offset = (slot.metadata & 0x00ff_ffff) as usize;
                    let count = (slot.metadata >> 24) as usize;
                    for packed_score in &self.blob[offset..offset + count] {
                        let language = (*packed_score & 0xff) as usize;
                        let weight = f32::from_bits(*packed_score & 0xffff_ff00);
                        raw[language] += weight;
                    }
                    break;
                }
                index = (index + 1) & mask;
            }
        }

        let mut ranked_raw = Vec::with_capacity(LANGUAGE_COUNT);
        for (index, score) in raw.into_iter().enumerate() {
            if score > 1.0 {
                ranked_raw.push((index, score));
            }
        }
        for index in 1..ranked_raw.len() {
            let item = ranked_raw[index];
            let mut insertion = index;
            while insertion > 0 && ranked_raw[insertion - 1].1 < item.1 {
                ranked_raw[insertion] = ranked_raw[insertion - 1];
                insertion -= 1;
            }
            ranked_raw[insertion] = item;
        }

        let inverse_count = -0.0001_f32 / features.len() as f32;
        let ranked_scores: Vec<_> = ranked_raw
            .into_iter()
            .map(|(index, score)| RankedScore {
                language: Language::from_index(index),
                score: 1.0_f32 - (inverse_count * score).exp(),
            })
            .collect();
        let language = ranked_scores.first().map(|entry| entry.language);
        let top_score = ranked_scores.first().map_or(0.0, |entry| entry.score);
        let second_score = ranked_scores.get(1).map_or(0.0, |entry| entry.score);
        let reliable = language.is_some()
            && features.len() >= 3
            && top_score >= 0.85_f32 * self.averages[language.expect("checked above") as usize]
            && top_score - second_score > 0.02_f32;

        Detection {
            language,
            reliable,
            top_score,
            second_score,
            feature_count: features.len(),
            ranked_scores,
        }
    }
}

fn parse_model(bytes: &[u8]) -> Result<Detector, ModelError> {
    if bytes.len() < HEADER_LEN {
        return Err(ModelError::Truncated);
    }
    if &bytes[..8] != MAGIC {
        return Err(ModelError::InvalidMagic);
    }

    let version = read_u32(bytes, 8)?;
    if version != FORMAT_VERSION {
        return Err(ModelError::UnsupportedVersion(version));
    }
    let language_count = read_u32(bytes, 12)?;
    if language_count as usize != LANGUAGE_COUNT {
        return Err(ModelError::InvalidLanguageCount(language_count));
    }
    let table_len = read_u32(bytes, 16)?;
    if table_len == 0 || !table_len.is_power_of_two() {
        return Err(ModelError::InvalidTableLength(table_len));
    }
    let blob_len = read_u32(bytes, 20)?;
    let letter_len = read_u32(bytes, 24)?;
    if letter_len as usize != LETTER_TABLE_LEN {
        return Err(ModelError::InvalidLetterTableLength(letter_len));
    }
    let cjk_len = read_u32(bytes, 28)?;
    if cjk_len as usize != CJK_TABLE_LEN {
        return Err(ModelError::InvalidCjkTableLength(cjk_len));
    }
    let lowercase_len = read_u32(bytes, 32)?;
    if lowercase_len as usize != LOWERCASE_TABLE_LEN {
        return Err(ModelError::InvalidLowercaseTableLength(lowercase_len));
    }
    if &bytes[36..HEADER_LEN] != SOURCE_COMMIT {
        return Err(ModelError::InvalidSourceCommit);
    }

    let table_bytes = usize::try_from(table_len)
        .ok()
        .and_then(|length| length.checked_mul(8))
        .ok_or(ModelError::Truncated)?;
    let blob_bytes = usize::try_from(blob_len)
        .ok()
        .and_then(|length| length.checked_mul(4))
        .ok_or(ModelError::Truncated)?;
    let expected_len = HEADER_LEN
        .checked_add(LANGUAGE_COUNT * 4)
        .and_then(|length| length.checked_add(LETTER_TABLE_LEN))
        .and_then(|length| length.checked_add(CJK_TABLE_LEN))
        .and_then(|length| length.checked_add(LOWERCASE_TABLE_LEN * 2))
        .and_then(|length| length.checked_add(table_bytes))
        .and_then(|length| length.checked_add(blob_bytes))
        .ok_or(ModelError::Truncated)?;
    if bytes.len() < expected_len {
        return Err(ModelError::Truncated);
    }
    if bytes.len() > expected_len {
        return Err(ModelError::TrailingData);
    }

    let mut cursor = HEADER_LEN;
    let mut averages = [0.0_f32; LANGUAGE_COUNT];
    for average in &mut averages {
        *average = f32::from_bits(take_u32(bytes, &mut cursor)?);
    }
    let letter_bits = take_bytes(bytes, &mut cursor, LETTER_TABLE_LEN)?.into();
    let cjk_bits = take_bytes(bytes, &mut cursor, CJK_TABLE_LEN)?.into();
    let mut lowercase = Vec::with_capacity(LOWERCASE_TABLE_LEN);
    for _ in 0..LOWERCASE_TABLE_LEN {
        lowercase.push(take_u16(bytes, &mut cursor)?);
    }
    let mut table = Vec::with_capacity(table_len as usize);
    let mut has_empty_slot = false;
    for slot_index in 0..table_len as usize {
        let fingerprint = take_u32(bytes, &mut cursor)?;
        let metadata = take_u32(bytes, &mut cursor)?;
        if fingerprint == 0 {
            has_empty_slot = true;
            if metadata != 0 {
                return Err(ModelError::InvalidEmptySlot { slot: slot_index });
            }
        }
        let offset = (metadata & 0x00ff_ffff) as usize;
        let count = (metadata >> 24) as usize;
        if offset
            .checked_add(count)
            .is_none_or(|end| end > blob_len as usize)
        {
            return Err(ModelError::InvalidBlobRange { slot: slot_index });
        }
        table.push(Slot {
            fingerprint,
            metadata,
        });
    }
    if !has_empty_slot {
        return Err(ModelError::InvalidTableLength(table_len));
    }
    let mut blob = Vec::with_capacity(blob_len as usize);
    for index in 0..blob_len as usize {
        let packed_score = take_u32(bytes, &mut cursor)?;
        if packed_score as u8 as usize >= LANGUAGE_COUNT {
            return Err(ModelError::InvalidLanguageIndex { index });
        }
        blob.push(packed_score);
    }

    Ok(Detector {
        averages,
        letter_bits,
        cjk_bits,
        lowercase: lowercase.into_boxed_slice(),
        table: table.into_boxed_slice(),
        blob: blob.into_boxed_slice(),
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ModelError> {
    let value = bytes.get(offset..offset + 4).ok_or(ModelError::Truncated)?;
    Ok(u32::from_le_bytes(value.try_into().expect("four bytes")))
}

fn take_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, ModelError> {
    let value = take_bytes(bytes, cursor, 2)?;
    Ok(u16::from_le_bytes(value.try_into().expect("two bytes")))
}

fn take_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, ModelError> {
    let value = take_bytes(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(value.try_into().expect("four bytes")))
}

fn take_bytes<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], ModelError> {
    let end = cursor.checked_add(length).ok_or(ModelError::Truncated)?;
    let value = bytes.get(*cursor..end).ok_or(ModelError::Truncated)?;
    *cursor = end;
    Ok(value)
}

fn h64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum CharacterType {
    Separator,
    Letter,
    Cjk,
    Apostrophe,
}

fn input_bytes(text: &str) -> &[u8] {
    let bytes = text.as_bytes();
    let inspected_len = bytes
        .iter()
        .take(MAX_INPUT_BYTES + 2)
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len().min(MAX_INPUT_BYTES + 2));
    let mut length = inspected_len.min(MAX_INPUT_BYTES);
    while length > 0 && length < inspected_len && bytes[length] & 0xc0 == 0x80 {
        length -= 1;
    }
    &bytes[..length]
}

fn lowercase_bytes(input: &[u8], lowercase: &[u16]) -> Vec<u8> {
    let mut output = vec![0_u8; input.len()];
    let mut index = 0;
    while index < input.len() {
        let first = input[index];
        if first < 0x80 {
            output[index] = if first.is_ascii_uppercase() {
                first + 32
            } else {
                first
            };
            index += 1;
        } else if first < 0xc2 || index + 1 >= input.len() {
            output[index] = first;
            index += 1;
        } else if first < 0xe0 {
            let table_index =
                ((usize::from(first - 0xc2)) << 6) | usize::from(input[index + 1] & 0x3f);
            let original = 0x80_u16 + table_index as u16;
            let codepoint = if lowercase[table_index] == 0 {
                original
            } else {
                lowercase[table_index]
            };
            output[index] = 0xc0 | (codepoint >> 6) as u8;
            output[index + 1] = 0x80 | (codepoint & 0x3f) as u8;
            index += 2;
        } else if first < 0xf0 {
            if index + 2 >= input.len() {
                output[index] = first;
                index += 1;
                continue;
            }
            let second = input[index + 1];
            let third = input[index + 2];
            if first == 0xe1 && second == 0x82 && third >= 0xa0 {
                output[index..index + 3].copy_from_slice(&[0xe2, 0xb4, third - 0x20]);
            } else if first == 0xe1 && second == 0x83 && (0x80..=0x85).contains(&third) {
                output[index..index + 3].copy_from_slice(&[0xe2, 0xb4, third + 0x20]);
            } else if first == 0xe1
                && (0xb8..=0xbb).contains(&second)
                && third & 1 == 0
                && !(second == 0xba && third == 0x9e)
            {
                output[index..index + 3].copy_from_slice(&[first, second, third + 1]);
            } else if first == 0xef && second == 0xbc && (0xa1..=0xba).contains(&third) {
                output[index..index + 3].copy_from_slice(&[0xef, 0xbd, third - 0x20]);
            } else {
                output[index..index + 3].copy_from_slice(&input[index..index + 3]);
            }
            index += 3;
        } else {
            let width = (input.len() - index).min(4);
            output[index..index + width].copy_from_slice(&input[index..index + width]);
            index += width;
        }
    }
    output
}

fn bit_is_set(bits: &[u8], codepoint: u32) -> bool {
    codepoint <= 0xffff && bits[codepoint as usize >> 3] >> (codepoint & 7) & 1 == 1
}

fn next_character(
    bytes: &[u8],
    index: usize,
    letter_bits: &[u8],
    cjk_bits: &[u8],
) -> (usize, CharacterType) {
    let first = bytes[index];
    let remaining = bytes.len() - index;
    if first < 0x80 {
        let character_type = if first.is_ascii_alphabetic() {
            CharacterType::Letter
        } else if first == b'\'' || first == b'`' {
            CharacterType::Apostrophe
        } else {
            CharacterType::Separator
        };
        return (1, character_type);
    }
    if first < 0xc2 || remaining < 2 {
        return (1, CharacterType::Separator);
    }
    if first < 0xe0 {
        let codepoint = (u32::from(first & 0x1f) << 6) | u32::from(bytes[index + 1] & 0x3f);
        let character_type = if bit_is_set(letter_bits, codepoint) {
            CharacterType::Letter
        } else {
            CharacterType::Separator
        };
        return (2, character_type);
    }
    if first < 0xf0 {
        if remaining < 3 {
            return (1, CharacterType::Separator);
        }
        let codepoint = (u32::from(first & 0x0f) << 12)
            | (u32::from(bytes[index + 1] & 0x3f) << 6)
            | u32::from(bytes[index + 2] & 0x3f);
        let character_type = if codepoint == 0x2019 {
            CharacterType::Apostrophe
        } else if (0x1100..=0xffdc).contains(&codepoint) && bit_is_set(cjk_bits, codepoint) {
            CharacterType::Cjk
        } else if bit_is_set(letter_bits, codepoint) {
            CharacterType::Letter
        } else {
            CharacterType::Separator
        };
        return (3, character_type);
    }
    if remaining < 4 {
        return (remaining, CharacterType::Separator);
    }
    let codepoint = (u32::from(first & 0x07) << 18)
        | (u32::from(bytes[index + 1] & 0x3f) << 12)
        | (u32::from(bytes[index + 2] & 0x3f) << 6)
        | u32::from(bytes[index + 3] & 0x3f);
    let is_cjk = (0x20000..=0x2a6df).contains(&codepoint)
        || (0x2a700..=0x2ceaf).contains(&codepoint)
        || (0x2ceb0..=0x2ebef).contains(&codepoint)
        || (0x2f800..=0x2fa1f).contains(&codepoint);
    (
        4,
        if is_cjk {
            CharacterType::Cjk
        } else {
            CharacterType::Separator
        },
    )
}

fn add_feature(features: &mut Vec<u64>, bytes: &[u8]) {
    if features.len() >= MAX_FEATURES {
        return;
    }
    let mut packed = [0_u8; 8];
    packed[..bytes.len()].copy_from_slice(bytes);
    let key = u64::from_le_bytes(packed);
    if !features.contains(&key) {
        features.push(key);
    }
}

fn extract_features_with_tables(
    text: &str,
    letter_bits: &[u8],
    cjk_bits: &[u8],
    lowercase: &[u16],
) -> Vec<u64> {
    let input = input_bytes(text);
    let bytes = lowercase_bytes(input, lowercase);
    let mut features = Vec::new();
    let mut index = 0;
    while index < bytes.len() && features.len() < MAX_FEATURES {
        let (width, character_type) = next_character(&bytes, index, letter_bits, cjk_bits);
        if character_type == CharacterType::Cjk {
            let mut feature = [0_u8; 8];
            feature[0] = b' ';
            feature[1..1 + width].copy_from_slice(&bytes[index..index + width]);
            feature[1 + width] = b' ';
            add_feature(&mut features, &feature);
            index += width;
            continue;
        }
        if character_type != CharacterType::Letter {
            index += width;
            continue;
        }

        let word_start = index;
        index += width;
        while index < bytes.len() {
            let (inner_width, inner_type) = next_character(&bytes, index, letter_bits, cjk_bits);
            if inner_type == CharacterType::Letter {
                index += inner_width;
            } else if inner_type == CharacterType::Apostrophe {
                let next = index + inner_width;
                if next < bytes.len()
                    && next_character(&bytes, next, letter_bits, cjk_bits).1
                        == CharacterType::Letter
                {
                    index += inner_width;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let word = &bytes[word_start..index];
        if word.len() <= 6 {
            let mut feature = [0_u8; 8];
            feature[0] = b' ';
            feature[1..1 + word.len()].copy_from_slice(word);
            feature[1 + word.len()] = b' ';
            add_feature(&mut features, &feature);
        } else {
            let mut first = [0_u8; 8];
            first[0] = b' ';
            first[1..7].copy_from_slice(&word[..6]);
            add_feature(&mut features, &first);
            if features.len() >= MAX_FEATURES {
                break;
            }
            let mut chunk_start = 6;
            while chunk_start + 6 < word.len() && features.len() < MAX_FEATURES {
                add_feature(&mut features, &word[chunk_start..chunk_start + 6]);
                chunk_start += 6;
            }
            if features.len() >= MAX_FEATURES {
                break;
            }
            let mut last = [0_u8; 8];
            last[..6].copy_from_slice(&word[word.len() - 6..]);
            last[6] = b' ';
            add_feature(&mut features, &last);
        }
    }
    features
}

#[cfg(test)]
fn extract_features(text: &str) -> Vec<u64> {
    let detector = Detector::new().expect("embedded model must be valid");
    extract_features_with_tables(
        text,
        &detector.letter_bits,
        &detector.cjk_bits,
        &detector.lowercase,
    )
}

#[cfg(test)]
mod tests;
