//! Per-language slices of the 15-profile model.
//!
//! A slice carries one language's live entries. Each entry records the
//! distance from the start of its occupied run, so any set of slices routes
//! text without the global occupancy bitmap: a probe from the feature's home
//! slot reaches an entry exactly when that distance covers the home slot.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::{
    CJK_TABLE_LEN, Detection, Detector, HEADER_LEN, LANGUAGE_COUNT, LETTER_TABLE_LEN,
    LOWERCASE_TABLE_LEN, Language, ModelError, SOURCE_COMMIT, bit_at, extract_features_with_tables,
    finish_detection, h64, live_slots, parse_model,
};

pub const SLICE_MAGIC: &[u8; 8] = b"BLSPHDET";
pub const SLICE_FORMAT_VERSION: u32 = 1;
pub const SLICE_HEADER_LEN: usize = 68;
const ENTRY_LEN: usize = 12;
const MIN_TABLE_LEN: u32 = 64;
const TABLES_LEN: usize = LETTER_TABLE_LEN + CJK_TABLE_LEN + LOWERCASE_TABLE_LEN * 2;
const TABLES_OFFSET: usize = HEADER_LEN + LANGUAGE_COUNT * 4;

/// The Unicode tables every slice shares: letter bits, CJK bits, lowercase map.
pub const TABLES: &[u8] = include_bytes!("../data/eld-tables-v1.bin");

const _: () = assert!(TABLES.len() == TABLES_LEN, "eld-tables-v1.bin length");

/// A slice validation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SliceError {
    InvalidMagic,
    UnsupportedVersion(u32),
    UnknownLanguage([u8; 2]),
    InvalidTableLength(u32),
    TableLengthMismatch,
    InvalidSourceCommit,
    Truncated,
    TrailingData,
    InvalidEntry { index: usize },
    Unsorted { index: usize },
    DuplicateLanguage(Language),
    Empty,
}

impl Display for SliceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => formatter.write_str("the detect slice magic is invalid"),
            Self::UnsupportedVersion(version) => {
                write!(
                    formatter,
                    "the detect slice version {version} is unsupported"
                )
            }
            Self::UnknownLanguage(code) => write!(
                formatter,
                "the detect slice language {:?} is unknown",
                String::from_utf8_lossy(code)
            ),
            Self::InvalidTableLength(length) => {
                write!(
                    formatter,
                    "the detect slice table length {length} is invalid"
                )
            }
            Self::TableLengthMismatch => {
                formatter.write_str("the detect slices disagree on the table length")
            }
            Self::InvalidSourceCommit => {
                formatter.write_str("the detect slice source commit is invalid")
            }
            Self::Truncated => formatter.write_str("the detect slice is truncated"),
            Self::TrailingData => formatter.write_str("the detect slice has trailing data"),
            Self::InvalidEntry { index } => {
                write!(formatter, "the detect slice entry {index} is invalid")
            }
            Self::Unsorted { index } => {
                write!(formatter, "the detect slice entry {index} is out of order")
            }
            Self::DuplicateLanguage(language) => {
                write!(
                    formatter,
                    "the detect slice for {} was given twice",
                    language.code()
                )
            }
            Self::Empty => formatter.write_str("no detect slice was given"),
        }
    }
}

impl Error for SliceError {}

#[derive(Clone, Copy, Debug)]
struct SliceEntry {
    fingerprint: u32,
    slot: u32,
    run_offset: u8,
    language: u8,
    weight: f32,
}

struct SliceHeader {
    language: Language,
    table_len: usize,
    entry_count: usize,
    average: f32,
}

/// A detector over any subset of the fifteen language slices.
#[derive(Debug)]
pub struct SliceDetector {
    averages: [f32; LANGUAGE_COUNT],
    loaded: [bool; LANGUAGE_COUNT],
    mask: usize,
    entries: Box<[SliceEntry]>,
    lowercase: Box<[u16]>,
}

impl SliceDetector {
    /// Merges one or more slices into a detector for exactly those languages.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid slice, a repeated language, or no slices.
    pub fn from_slices(slices: &[&[u8]]) -> Result<Self, SliceError> {
        if slices.is_empty() {
            return Err(SliceError::Empty);
        }
        let mut averages = [0.0_f32; LANGUAGE_COUNT];
        let mut loaded = [false; LANGUAGE_COUNT];
        let mut entries = Vec::new();
        let mut table_len = None;
        for bytes in slices {
            let header = parse_header(bytes)?;
            let index = header.language.index();
            if loaded[index] {
                return Err(SliceError::DuplicateLanguage(header.language));
            }
            if table_len.is_some_and(|length| length != header.table_len) {
                return Err(SliceError::TableLengthMismatch);
            }
            table_len = Some(header.table_len);
            loaded[index] = true;
            averages[index] = header.average;
            read_entries(bytes, &header, &mut entries)?;
        }
        entries.sort_unstable_by_key(|entry| (entry.fingerprint, entry.slot, entry.language));
        Ok(Self {
            averages,
            loaded,
            mask: table_len.expect("at least one slice") - 1,
            entries: entries.into_boxed_slice(),
            lowercase: lowercase_table(),
        })
    }

    /// The languages this detector can return, in model order.
    #[must_use]
    pub fn languages(&self) -> Vec<Language> {
        Language::ALL
            .into_iter()
            .filter(|language| self.loaded[language.index()])
            .collect()
    }

    /// Detects the highest scoring loaded language.
    #[must_use]
    pub fn detect(&self, text: &str) -> Detection {
        let features =
            extract_features_with_tables(text, letter_bits(), cjk_bits(), &self.lowercase);
        if features.is_empty() {
            return Detection::empty();
        }
        let mut raw = [1.0_f32; LANGUAGE_COUNT];
        for feature in &features {
            self.add_feature_scores(*feature, &mut raw);
        }
        finish_detection(&self.averages, raw, features.len())
    }

    /// Finds the entry the full table's probe from the home slot would reach.
    fn add_feature_scores(&self, feature: u64, raw: &mut [f32; LANGUAGE_COUNT]) {
        let hash = h64(feature);
        let fingerprint = ((hash >> 32) as u32).max(1);
        let home = (hash as u32 as usize) & self.mask;
        let start = self
            .entries
            .partition_point(|entry| entry.fingerprint < fingerprint);
        let length =
            self.entries[start..].partition_point(|entry| entry.fingerprint == fingerprint);
        let group = &self.entries[start..start + length];

        let mut nearest: Option<(usize, u32)> = None;
        for entry in group {
            let distance = (entry.slot as usize).wrapping_sub(home) & self.mask;
            if distance > usize::from(entry.run_offset) {
                continue;
            }
            if nearest.is_none_or(|(best, _)| distance < best) {
                nearest = Some((distance, entry.slot));
            }
        }
        let Some((_, slot)) = nearest else {
            return;
        };
        for entry in group.iter().filter(|entry| entry.slot == slot) {
            raw[usize::from(entry.language)] += entry.weight;
        }
    }
}

fn letter_bits() -> &'static [u8] {
    &TABLES[..LETTER_TABLE_LEN]
}

fn cjk_bits() -> &'static [u8] {
    &TABLES[LETTER_TABLE_LEN..LETTER_TABLE_LEN + CJK_TABLE_LEN]
}

fn lowercase_table() -> Box<[u16]> {
    TABLES[LETTER_TABLE_LEN + CJK_TABLE_LEN..]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

fn parse_header(bytes: &[u8]) -> Result<SliceHeader, SliceError> {
    if bytes.len() < SLICE_HEADER_LEN {
        return Err(SliceError::Truncated);
    }
    if &bytes[..8] != SLICE_MAGIC {
        return Err(SliceError::InvalidMagic);
    }
    let version = read_u32(bytes, 8);
    if version != SLICE_FORMAT_VERSION {
        return Err(SliceError::UnsupportedVersion(version));
    }
    let code = [bytes[12], bytes[13]];
    let language = std::str::from_utf8(&code)
        .ok()
        .and_then(Language::from_code)
        .ok_or(SliceError::UnknownLanguage(code))?;
    let table_len = read_u32(bytes, 16);
    if table_len < MIN_TABLE_LEN || !table_len.is_power_of_two() {
        return Err(SliceError::InvalidTableLength(table_len));
    }
    let entry_count = read_u32(bytes, 20) as usize;
    let average = f32::from_bits(read_u32(bytes, 24));
    if &bytes[28..SLICE_HEADER_LEN] != SOURCE_COMMIT {
        return Err(SliceError::InvalidSourceCommit);
    }
    Ok(SliceHeader {
        language,
        table_len: table_len as usize,
        entry_count,
        average,
    })
}

fn read_entries(
    bytes: &[u8],
    header: &SliceHeader,
    entries: &mut Vec<SliceEntry>,
) -> Result<(), SliceError> {
    let length = header
        .entry_count
        .checked_mul(ENTRY_LEN)
        .ok_or(SliceError::Truncated)?;
    let body = bytes
        .get(SLICE_HEADER_LEN..SLICE_HEADER_LEN + length)
        .ok_or(SliceError::Truncated)?;
    if bytes.len() != SLICE_HEADER_LEN + length {
        return Err(SliceError::TrailingData);
    }
    let language = header.language.index() as u8;
    let mut previous: Option<(u32, u32)> = None;
    for (index, chunk) in body.chunks_exact(ENTRY_LEN).enumerate() {
        let slot = read_u32(chunk, 0);
        let fingerprint = read_u32(chunk, 4);
        let packed = read_u32(chunk, 8);
        if fingerprint == 0 || slot as usize >= header.table_len {
            return Err(SliceError::InvalidEntry { index });
        }
        if previous.is_some_and(|last| last >= (fingerprint, slot)) {
            return Err(SliceError::Unsorted { index });
        }
        previous = Some((fingerprint, slot));
        entries.push(SliceEntry {
            fingerprint,
            slot,
            run_offset: (packed & 0xff) as u8,
            language,
            weight: f32::from_bits(packed & 0xffff_ff00),
        });
    }
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"))
}

#[derive(Clone, Copy)]
struct RawEntry {
    fingerprint: u32,
    slot: u32,
    packed: u32,
}

/// Splits a full model into one slice per language, in model order.
///
/// # Errors
///
/// Returns an error when the model is invalid or an occupied run exceeds 255 slots.
pub fn write_slices(model: &[u8]) -> Result<Vec<(Language, Vec<u8>)>, ModelError> {
    let detector = parse_model(model)?;
    let table_len = detector.mask + 1;
    let run_offsets = run_offsets(&detector.occupied, table_len)?;
    let mut per_language: Vec<Vec<RawEntry>> = vec![Vec::new(); LANGUAGE_COUNT];
    for (rank, slot) in live_slots(&detector.live).enumerate() {
        let entry = detector.entries[rank];
        let offset = (entry.metadata & 0x00ff_ffff) as usize;
        let count = (entry.metadata >> 24) as usize;
        for packed_score in &detector.blob[offset..offset + count] {
            let language = (*packed_score & 0xff) as usize;
            per_language[language].push(RawEntry {
                fingerprint: entry.fingerprint,
                slot: slot as u32,
                packed: (packed_score & 0xffff_ff00) | u32::from(run_offsets[slot]),
            });
        }
    }
    Ok(Language::ALL
        .into_iter()
        .zip(per_language.iter_mut())
        .map(|(language, entries)| {
            entries.sort_unstable_by_key(|entry| (entry.fingerprint, entry.slot));
            let average = detector.averages[language.index()];
            (language, serialize(language, table_len, average, entries))
        })
        .collect())
}

/// The Unicode tables section of a full model, as `TABLES` stores it.
///
/// # Errors
///
/// Returns an error when the model is invalid.
pub fn write_tables(model: &[u8]) -> Result<Vec<u8>, ModelError> {
    let _ = parse_model(model)?;
    Ok(model[TABLES_OFFSET..TABLES_OFFSET + TABLES_LEN].to_vec())
}

/// The distance of every occupied slot from the start of its run. Runs wrap.
fn run_offsets(occupied: &[u64], table_len: usize) -> Result<Vec<u8>, ModelError> {
    let first_empty = (0..table_len)
        .find(|index| !bit_at(occupied, *index))
        .expect("parse_model keeps one empty slot");
    let mask = table_len - 1;
    let mut offsets = vec![0_u8; table_len];
    let mut run = 0_usize;
    for step in 1..=table_len {
        let index = (first_empty + step) & mask;
        if !bit_at(occupied, index) {
            run = 0;
            continue;
        }
        offsets[index] = u8::try_from(run).map_err(|_| ModelError::RunTooLong { slot: index })?;
        run += 1;
    }
    Ok(offsets)
}

fn serialize(language: Language, table_len: usize, average: f32, entries: &[RawEntry]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(SLICE_HEADER_LEN + entries.len() * ENTRY_LEN);
    bytes.extend_from_slice(SLICE_MAGIC);
    bytes.extend_from_slice(&SLICE_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(language.code().as_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&(table_len as u32).to_le_bytes());
    bytes.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&average.to_bits().to_le_bytes());
    bytes.extend_from_slice(SOURCE_COMMIT);
    for entry in entries {
        bytes.extend_from_slice(&entry.slot.to_le_bytes());
        bytes.extend_from_slice(&entry.fingerprint.to_le_bytes());
        bytes.extend_from_slice(&entry.packed.to_le_bytes());
    }
    bytes
}

impl Detector {
    /// Splits this model's bytes into slices. See [`write_slices`].
    ///
    /// # Errors
    ///
    /// Returns an error when the model is invalid.
    pub fn slices(model: &[u8]) -> Result<Vec<(Language, Vec<u8>)>, ModelError> {
        write_slices(model)
    }
}
