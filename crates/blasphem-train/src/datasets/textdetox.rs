use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Write},
};

use serde::Deserialize;
use thiserror::Error;

use bytes::Bytes;
use parquet::{
    file::reader::{FileReader, SerializedFileReader},
    record::RowAccessor,
};

use blasphem::{EvalLabel, EvalRow, Language, normalize_text};

use super::{DatasetAdapter, DatasetId, ImportError, ImportedRow, SourceInput};

pub const TEXTDETOX_REVISION: &str = "01907546324b0330d2d8b7669648cc18823323e5";
pub const TEXTDETOX_CODES: &[&str] = &["en", "zh", "es", "ar", "fr", "hi", "ru", "ja", "de", "it"];
pub const MAX_TEXTDETOX_PARQUET_ROWS: u64 = 100_000;
pub const MAX_TEXTDETOX_PARQUET_TEXT_BYTES: u64 = 67_108_864;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextDetoxParquetLimits {
    pub max_rows: u64,
    pub max_text_bytes: u64,
}

impl Default for TextDetoxParquetLimits {
    fn default() -> Self {
        Self {
            max_rows: MAX_TEXTDETOX_PARQUET_ROWS,
            max_text_bytes: MAX_TEXTDETOX_PARQUET_TEXT_BYTES,
        }
    }
}

pub struct TextDetoxAdapter;

impl DatasetAdapter for TextDetoxAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::TextDetox
    }

    fn label_conversion_version(&self) -> &'static str {
        "textdetox-binary-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let mut output = Vec::new();
        for input in inputs {
            let source_code = source_code_from_file_id(input.source_file_id)?;
            let detector_language = detector_language(source_code)?;
            let mut bytes = Vec::new();
            input.reader.read_to_end(&mut bytes)?;
            let rows = if bytes
                .iter()
                .copied()
                .find(|byte| !byte.is_ascii_whitespace())
                == Some(b'{')
            {
                parse_textdetox_page(bytes.as_slice(), source_code, TEXTDETOX_REVISION)
                    .map_err(|error| ImportError::InvalidSource(error.to_string()))?
                    .rows
            } else {
                parse_textdetox_rows(bytes.as_slice())
                    .map_err(|error| ImportError::InvalidSource(error.to_string()))?
            };
            for row in rows {
                if row.language.source_code() != source_code
                    || !row
                        .source_id
                        .starts_with(&format!("textdetox@{TEXTDETOX_REVISION}/{source_code}/"))
                {
                    return Err(ImportError::InvalidSource(row.source_id));
                }
                output.push(ImportedRow {
                    dataset: DatasetId::TextDetox,
                    source_file_id: input.source_file_id.to_owned(),
                    source_id: row.source_id,
                    source_language_code: source_code.to_owned(),
                    detector_language: Some(detector_language),
                    detector_language_code: Some(detector_language.code().to_owned()),
                    source_label: source_label(row.label).to_owned(),
                    text: row.text,
                    source_split: input.source_split,
                    disposition: super::RowDisposition::Candidate(row.label),
                });
            }
        }
        Ok(output)
    }
}

fn source_code_from_file_id(source_file_id: &str) -> Result<&str, ImportError> {
    let Some(source_code) = source_file_id.strip_prefix("textdetox-") else {
        return Err(ImportError::InvalidSource(source_file_id.to_owned()));
    };
    if !TEXTDETOX_CODES.contains(&source_code) {
        return Err(ImportError::InvalidSource(source_file_id.to_owned()));
    }
    Ok(source_code)
}

fn detector_language(source_code: &str) -> Result<Language, ImportError> {
    match source_code {
        "en" => Ok(Language::En),
        "zh" => Ok(Language::Zh),
        "es" => Ok(Language::Es),
        "ar" => Ok(Language::Ar),
        "fr" => Ok(Language::Fr),
        "hi" => Ok(Language::Hi),
        "ru" => Ok(Language::Ru),
        "ja" => Ok(Language::Ja),
        "de" => Ok(Language::De),
        "it" => Ok(Language::It),
        _ => Err(ImportError::InvalidSource(source_code.to_owned())),
    }
}

pub const TEXTDETOX_PREPARATION_VERSION: &str = "v1";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextDetoxLanguage {
    Amharic,
    Arabic,
    German,
    English,
    Spanish,
    French,
    Hebrew,
    Hindi,
    Hinglish,
    Italian,
    Japanese,
    Russian,
    Tatar,
    Ukrainian,
    Chinese,
}

impl TextDetoxLanguage {
    #[must_use]
    pub const fn source_code(self) -> &'static str {
        match self {
            Self::Amharic => "am",
            Self::Arabic => "ar",
            Self::German => "de",
            Self::English => "en",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::Hebrew => "he",
            Self::Hindi => "hi",
            Self::Hinglish => "hin",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::Russian => "ru",
            Self::Tatar => "tt",
            Self::Ukrainian => "uk",
            Self::Chinese => "zh",
        }
    }

    #[must_use]
    pub const fn detector_code(self) -> &'static str {
        match self {
            Self::Amharic => "AM",
            Self::Arabic => "AR",
            Self::German => "DE",
            Self::English => "EN",
            Self::Spanish => "ES",
            Self::French => "FR",
            Self::Hebrew => "HE",
            Self::Hindi => "HI",
            Self::Hinglish => "HINGLISH",
            Self::Italian => "IT",
            Self::Japanese => "JA",
            Self::Russian => "RU",
            Self::Tatar => "TT",
            Self::Ukrainian => "UK",
            Self::Chinese => "ZH",
        }
    }

    fn parse_source_code(value: &str) -> Result<Self, TextDetoxError> {
        match value {
            "am" => Ok(Self::Amharic),
            "ar" => Ok(Self::Arabic),
            "de" => Ok(Self::German),
            "en" => Ok(Self::English),
            "es" => Ok(Self::Spanish),
            "fr" => Ok(Self::French),
            "he" => Ok(Self::Hebrew),
            "hi" => Ok(Self::Hindi),
            "hin" => Ok(Self::Hinglish),
            "it" => Ok(Self::Italian),
            "ja" => Ok(Self::Japanese),
            "ru" => Ok(Self::Russian),
            "tt" => Ok(Self::Tatar),
            "uk" => Ok(Self::Ukrainian),
            "zh" => Ok(Self::Chinese),
            _ => Err(TextDetoxError::InvalidLanguage(value.to_owned())),
        }
    }

    fn parse_detector_code(value: &str) -> Result<Self, TextDetoxError> {
        match value.trim().to_ascii_uppercase().as_str() {
            "AM" => Ok(Self::Amharic),
            "AR" => Ok(Self::Arabic),
            "DE" => Ok(Self::German),
            "EN" => Ok(Self::English),
            "ES" => Ok(Self::Spanish),
            "FR" => Ok(Self::French),
            "HE" => Ok(Self::Hebrew),
            "HI" => Ok(Self::Hindi),
            "HINGLISH" => Ok(Self::Hinglish),
            "IT" => Ok(Self::Italian),
            "JA" => Ok(Self::Japanese),
            "RU" => Ok(Self::Russian),
            "TT" => Ok(Self::Tatar),
            "UK" => Ok(Self::Ukrainian),
            "ZH" => Ok(Self::Chinese),
            _ => Err(TextDetoxError::InvalidLanguage(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDetoxSourceRow {
    pub source_id: String,
    pub language: TextDetoxLanguage,
    pub label: EvalLabel,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTextDetoxPage {
    pub rows: Vec<TextDetoxSourceRow>,
    pub total_rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatasetSplit {
    Development,
    Validation,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceStatus {
    Representative,
    Duplicate,
    LabelConflict,
    UnsupportedLanguage,
    EmptyText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceRow {
    pub source_id: String,
    pub source_language: String,
    pub detector_language: String,
    pub group_id: Option<String>,
    pub split: Option<DatasetSplit>,
    pub canonical_source_id: Option<String>,
    pub status: ProvenanceStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextDetoxSummary {
    pub source_rows: usize,
    pub evaluation_rows: usize,
    pub duplicate_rows: usize,
    pub conflicting_groups: usize,
    pub unsupported_rows: usize,
    pub empty_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTextDetox {
    pub development: Vec<EvalRow>,
    pub validation: Vec<EvalRow>,
    pub test: Vec<EvalRow>,
    pub provenance: Vec<ProvenanceRow>,
    pub summary: TextDetoxSummary,
}

impl PreparedTextDetox {
    #[must_use]
    pub fn rows(&self, split: DatasetSplit) -> &[EvalRow] {
        match split {
            DatasetSplit::Development => &self.development,
            DatasetSplit::Validation => &self.validation,
            DatasetSplit::Test => &self.test,
        }
    }
}

#[must_use]
pub fn split_for_key(language: &str, normalized_text: &str) -> DatasetSplit {
    match fnv_hash(language, normalized_text) % 100 {
        0..=69 => DatasetSplit::Development,
        70..=84 => DatasetSplit::Validation,
        _ => DatasetSplit::Test,
    }
}

fn fnv_hash(language: &str, normalized_text: &str) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in language
        .bytes()
        .chain(std::iter::once(0))
        .chain(normalized_text.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn group_id(language: &str, normalized_text: &str) -> String {
    format!(
        "{TEXTDETOX_PREPARATION_VERSION}-{:016x}",
        fnv_hash(language, normalized_text)
    )
}

struct TextGroup<'a> {
    detector_language: &'static str,
    normalized_text: String,
    group_id: String,
    split: DatasetSplit,
    rows: Vec<&'a TextDetoxSourceRow>,
}

pub fn prepare_textdetox(
    rows: &[TextDetoxSourceRow],
    included_languages: &BTreeSet<String>,
) -> Result<PreparedTextDetox, TextDetoxError> {
    prepare_textdetox_with_group_id(rows, included_languages, group_id)
}

fn prepare_textdetox_with_group_id(
    rows: &[TextDetoxSourceRow],
    included_languages: &BTreeSet<String>,
    group_id_for: impl Fn(&str, &str) -> String,
) -> Result<PreparedTextDetox, TextDetoxError> {
    let included_languages = validate_included_languages(included_languages)?;
    validate_source_ids(rows)?;

    let mut summary = TextDetoxSummary {
        source_rows: rows.len(),
        ..TextDetoxSummary::default()
    };
    let mut provenance = Vec::with_capacity(rows.len());
    let mut groups = BTreeMap::<String, TextGroup<'_>>::new();
    let mut group_keys_by_id = BTreeMap::<String, String>::new();

    for row in rows {
        let detector_language = row.language.detector_code();
        let mut normalized_text = normalize_text(&row.text);
        if normalized_text.is_empty() {
            normalized_text = row.text.trim().to_owned();
        }
        if normalized_text.is_empty() {
            summary.empty_rows += 1;
            provenance.push(unclassified_provenance(row, ProvenanceStatus::EmptyText));
            continue;
        }
        if !included_languages.contains(detector_language) {
            summary.unsupported_rows += 1;
            provenance.push(unclassified_provenance(
                row,
                ProvenanceStatus::UnsupportedLanguage,
            ));
            continue;
        }

        let key = format!("{detector_language}\0{normalized_text}");
        let group_id = group_id_for(detector_language, &normalized_text);
        if let Some(existing_key) = group_keys_by_id.get(&group_id)
            && existing_key != &key
        {
            return Err(TextDetoxError::GroupCollision { group_id });
        }
        group_keys_by_id.insert(group_id.clone(), key.clone());
        let split = split_for_key(detector_language, &normalized_text);
        groups
            .entry(key)
            .or_insert_with(|| TextGroup {
                detector_language,
                normalized_text,
                split,
                group_id,
                rows: Vec::new(),
            })
            .rows
            .push(row);
    }

    let mut evaluation_rows = Vec::<(String, DatasetSplit, EvalRow)>::new();
    for group in groups.values_mut() {
        group
            .rows
            .sort_by(|left, right| left.source_id.cmp(&right.source_id));
        group.split = split_for_key(group.detector_language, &group.normalized_text);
        let has_clean = group.rows.iter().any(|row| row.label == EvalLabel::Clean);
        let has_toxic = group.rows.iter().any(|row| row.label == EvalLabel::Toxic);
        if has_clean && has_toxic {
            summary.conflicting_groups += 1;
            provenance.extend(group.rows.iter().map(|row| ProvenanceRow {
                source_id: row.source_id.clone(),
                source_language: row.language.source_code().to_owned(),
                detector_language: group.detector_language.to_owned(),
                group_id: Some(group.group_id.clone()),
                split: Some(group.split),
                canonical_source_id: None,
                status: ProvenanceStatus::LabelConflict,
            }));
            continue;
        }

        let representative = group.rows[0];
        let canonical_source_id = representative.source_id.clone();
        evaluation_rows.push((
            group.group_id.clone(),
            group.split,
            EvalRow {
                language: group.detector_language.to_owned(),
                label: representative.label,
                text: representative.text.clone(),
            },
        ));
        summary.duplicate_rows += group.rows.len() - 1;
        provenance.extend(group.rows.iter().map(|row| ProvenanceRow {
            source_id: row.source_id.clone(),
            source_language: row.language.source_code().to_owned(),
            detector_language: group.detector_language.to_owned(),
            group_id: Some(group.group_id.clone()),
            split: Some(group.split),
            canonical_source_id: Some(canonical_source_id.clone()),
            status: if row.source_id == canonical_source_id {
                ProvenanceStatus::Representative
            } else {
                ProvenanceStatus::Duplicate
            },
        }));
    }

    evaluation_rows.sort_by(|left, right| left.0.cmp(&right.0));
    provenance.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    summary.evaluation_rows = evaluation_rows.len();
    let mut prepared = PreparedTextDetox {
        development: Vec::new(),
        validation: Vec::new(),
        test: Vec::new(),
        provenance,
        summary,
    };
    for (_, split, row) in evaluation_rows {
        match split {
            DatasetSplit::Development => prepared.development.push(row),
            DatasetSplit::Validation => prepared.validation.push(row),
            DatasetSplit::Test => prepared.test.push(row),
        }
    }
    Ok(prepared)
}

fn validate_included_languages(
    included_languages: &BTreeSet<String>,
) -> Result<BTreeSet<String>, TextDetoxError> {
    included_languages
        .iter()
        .map(|language| {
            TextDetoxLanguage::parse_detector_code(language)
                .map(|language| language.detector_code().to_owned())
        })
        .collect()
}

fn validate_source_ids(rows: &[TextDetoxSourceRow]) -> Result<(), TextDetoxError> {
    let mut source_ids = BTreeSet::new();
    for row in rows {
        if row.source_id.trim().is_empty() {
            return Err(TextDetoxError::BlankSourceId);
        }
        if !source_ids.insert(row.source_id.clone()) {
            return Err(TextDetoxError::DuplicateSourceId(row.source_id.clone()));
        }
    }
    Ok(())
}

fn unclassified_provenance(row: &TextDetoxSourceRow, status: ProvenanceStatus) -> ProvenanceRow {
    ProvenanceRow {
        source_id: row.source_id.clone(),
        source_language: row.language.source_code().to_owned(),
        detector_language: row.language.detector_code().to_owned(),
        group_id: None,
        split: None,
        canonical_source_id: None,
        status,
    }
}

#[derive(Debug, Error)]
pub enum TextDetoxError {
    #[error("cannot parse TextDetox TSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("cannot parse TextDetox JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot parse TextDetox Parquet: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),
    #[error("invalid TextDetox Parquet schema")]
    InvalidParquetSchema,
    #[error("invalid TextDetox Parquet metadata")]
    InvalidParquetMetadata,
    #[error("TextDetox Parquet row count {actual} exceeds limit {limit}")]
    ParquetRowLimit { actual: u64, limit: u64 },
    #[error("TextDetox Parquet text bytes {actual} exceed limit {limit}")]
    ParquetTextByteLimit { actual: u64, limit: u64 },
    #[error("invalid TextDetox language: {0}")]
    InvalidLanguage(String),
    #[error("unsupported TextDetox source language: {0}")]
    UnsupportedSourceLanguage(String),
    #[error("invalid TextDetox label: {0}")]
    InvalidLabel(String),
    #[error("blank TextDetox source ID")]
    BlankSourceId,
    #[error("duplicate TextDetox source ID: {0}")]
    DuplicateSourceId(String),
    #[error("invalid TextDetox page length: {0}")]
    InvalidPageLength(usize),
    #[error("blank TextDetox page revision")]
    BlankRevision,
    #[error("duplicate TextDetox page row index: {0}")]
    DuplicateRowIndex(usize),
    #[error("TextDetox page row index {row_index} is outside {total_rows} total rows")]
    RowIndexOutOfBounds { row_index: usize, total_rows: usize },
    #[error("TextDetox group ID collision: {group_id}")]
    GroupCollision { group_id: String },
}

#[derive(Debug, Deserialize)]
struct RawSourceRow {
    source_id: String,
    language: String,
    toxic: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawPage {
    rows: Vec<RawPageRow>,
    num_rows_total: usize,
}

#[derive(Debug, Deserialize)]
struct RawPageRow {
    row_idx: usize,
    row: RawPageValues,
}

#[derive(Debug, Deserialize)]
struct RawPageValues {
    text: String,
    toxic: i64,
}

pub fn parse_textdetox_rows(reader: impl Read) -> Result<Vec<TextDetoxSourceRow>, TextDetoxError> {
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .trim(csv::Trim::Headers)
        .from_reader(reader);
    let mut rows = Vec::new();
    let mut source_ids = BTreeSet::new();
    for row in csv.deserialize::<RawSourceRow>() {
        let row = row?;
        if row.source_id.trim().is_empty() {
            return Err(TextDetoxError::BlankSourceId);
        }
        if !source_ids.insert(row.source_id.clone()) {
            return Err(TextDetoxError::DuplicateSourceId(row.source_id));
        }
        let label = match row.toxic.trim() {
            "0" => EvalLabel::Clean,
            "1" => EvalLabel::Toxic,
            value => return Err(TextDetoxError::InvalidLabel(value.to_owned())),
        };
        rows.push(TextDetoxSourceRow {
            source_id: row.source_id,
            language: TextDetoxLanguage::parse_source_code(&row.language)?,
            label,
            text: row.text,
        });
    }
    Ok(rows)
}

pub fn parse_textdetox_page(
    reader: impl Read,
    language: &str,
    revision: &str,
) -> Result<ParsedTextDetoxPage, TextDetoxError> {
    if revision.trim().is_empty() {
        return Err(TextDetoxError::BlankRevision);
    }
    let language = TextDetoxLanguage::parse_source_code(language)?;
    let page: RawPage = serde_json::from_reader(reader)?;
    let mut row_indices = BTreeSet::new();
    let mut rows = Vec::with_capacity(page.rows.len());
    for raw in page.rows {
        if !row_indices.insert(raw.row_idx) {
            return Err(TextDetoxError::DuplicateRowIndex(raw.row_idx));
        }
        if raw.row_idx >= page.num_rows_total {
            return Err(TextDetoxError::RowIndexOutOfBounds {
                row_index: raw.row_idx,
                total_rows: page.num_rows_total,
            });
        }
        let label = match raw.row.toxic {
            0 => EvalLabel::Clean,
            1 => EvalLabel::Toxic,
            value => return Err(TextDetoxError::InvalidLabel(value.to_string())),
        };
        rows.push(TextDetoxSourceRow {
            source_id: format!(
                "textdetox@{revision}/{}/{:06}",
                language.source_code(),
                raw.row_idx
            ),
            language,
            label,
            text: raw.row.text,
        });
    }
    Ok(ParsedTextDetoxPage {
        rows,
        total_rows: page.num_rows_total,
    })
}

pub fn parse_textdetox_parquet(
    bytes: &[u8],
    language: &str,
    revision: &str,
) -> Result<Vec<TextDetoxSourceRow>, TextDetoxError> {
    parse_textdetox_parquet_with_limits(
        bytes,
        language,
        revision,
        TextDetoxParquetLimits::default(),
    )
}

pub fn parse_textdetox_parquet_with_limits(
    bytes: &[u8],
    language: &str,
    revision: &str,
    limits: TextDetoxParquetLimits,
) -> Result<Vec<TextDetoxSourceRow>, TextDetoxError> {
    if revision.trim().is_empty() {
        return Err(TextDetoxError::BlankRevision);
    }
    let language = TextDetoxLanguage::parse_source_code(language)?;
    let reader = SerializedFileReader::new(Bytes::copy_from_slice(bytes))?;
    let fields = reader
        .metadata()
        .file_metadata()
        .schema_descr()
        .root_schema()
        .get_fields();
    if fields.len() != 2 || fields[0].name() != "text" || fields[1].name() != "toxic" {
        return Err(TextDetoxError::InvalidParquetSchema);
    }
    let row_count = u64::try_from(reader.metadata().file_metadata().num_rows())
        .map_err(|_| TextDetoxError::InvalidParquetMetadata)?;
    if row_count > limits.max_rows {
        return Err(TextDetoxError::ParquetRowLimit {
            actual: row_count,
            limit: limits.max_rows,
        });
    }
    let mut encoded_text_bytes = 0_u64;
    for row_group in reader.metadata().row_groups() {
        let text_column = row_group
            .columns()
            .first()
            .ok_or(TextDetoxError::InvalidParquetMetadata)?;
        let column_bytes = u64::try_from(text_column.uncompressed_size())
            .map_err(|_| TextDetoxError::InvalidParquetMetadata)?;
        encoded_text_bytes = encoded_text_bytes.checked_add(column_bytes).ok_or(
            TextDetoxError::ParquetTextByteLimit {
                actual: u64::MAX,
                limit: limits.max_text_bytes,
            },
        )?;
        if encoded_text_bytes > limits.max_text_bytes {
            return Err(TextDetoxError::ParquetTextByteLimit {
                actual: encoded_text_bytes,
                limit: limits.max_text_bytes,
            });
        }
    }
    let capacity =
        usize::try_from(row_count).map_err(|_| TextDetoxError::InvalidParquetMetadata)?;
    let mut rows = Vec::with_capacity(capacity);
    let mut text_bytes = 0_u64;
    for (row_index, row) in reader.get_row_iter(None)?.enumerate() {
        let row = row?;
        let text = row.get_string(0)?.to_owned();
        text_bytes = text_bytes.checked_add(text.len() as u64).ok_or(
            TextDetoxError::ParquetTextByteLimit {
                actual: u64::MAX,
                limit: limits.max_text_bytes,
            },
        )?;
        if text_bytes > limits.max_text_bytes {
            return Err(TextDetoxError::ParquetTextByteLimit {
                actual: text_bytes,
                limit: limits.max_text_bytes,
            });
        }
        let label = match row.get_long(1)? {
            0 => EvalLabel::Clean,
            1 => EvalLabel::Toxic,
            value => return Err(TextDetoxError::InvalidLabel(value.to_string())),
        };
        rows.push(TextDetoxSourceRow {
            source_id: format!(
                "textdetox@{revision}/{}/{row_index:06}",
                language.source_code()
            ),
            language,
            label,
            text,
        });
    }
    if rows.len() != capacity {
        return Err(TextDetoxError::InvalidParquetMetadata);
    }
    Ok(rows)
}

pub fn textdetox_rows_url(
    language: &str,
    offset: usize,
    length: usize,
) -> Result<String, TextDetoxError> {
    if !(1..=100).contains(&length) {
        return Err(TextDetoxError::InvalidPageLength(length));
    }
    let language = TextDetoxLanguage::parse_source_code(language)
        .map_err(|_| TextDetoxError::UnsupportedSourceLanguage(language.to_owned()))?;
    Ok(format!(
        "https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split={}&offset={offset}&length={length}",
        language.source_code()
    ))
}

pub fn write_textdetox_source_tsv(
    writer: impl Write,
    rows: &[TextDetoxSourceRow],
) -> Result<(), TextDetoxError> {
    let mut csv = tsv_writer(writer);
    csv.write_record(["source_id", "language", "toxic", "text"])?;
    for row in rows {
        csv.write_record([
            row.source_id.as_str(),
            row.language.source_code(),
            source_label(row.label),
            row.text.as_str(),
        ])?;
    }
    csv.flush().map_err(csv::Error::from)?;
    Ok(())
}

pub fn write_textdetox_eval_tsv(
    writer: impl Write,
    rows: &[EvalRow],
) -> Result<(), TextDetoxError> {
    let mut csv = tsv_writer(writer);
    csv.write_record(["language", "label", "text"])?;
    for row in rows {
        csv.write_record([
            row.language.as_str(),
            evaluation_label(row.label),
            row.text.as_str(),
        ])?;
    }
    csv.flush().map_err(csv::Error::from)?;
    Ok(())
}

pub fn write_textdetox_provenance_tsv(
    writer: impl Write,
    rows: &[ProvenanceRow],
) -> Result<(), TextDetoxError> {
    let mut rows = rows.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    let mut csv = tsv_writer(writer);
    csv.write_record([
        "source_id",
        "source_language",
        "detector_language",
        "group_id",
        "split",
        "canonical_source_id",
        "status",
    ])?;
    for row in rows {
        csv.write_record([
            row.source_id.as_str(),
            row.source_language.as_str(),
            row.detector_language.as_str(),
            row.group_id.as_deref().unwrap_or(""),
            row.split.map(split_name).unwrap_or(""),
            row.canonical_source_id.as_deref().unwrap_or(""),
            provenance_status_name(row.status),
        ])?;
    }
    csv.flush().map_err(csv::Error::from)?;
    Ok(())
}

fn tsv_writer(writer: impl Write) -> csv::Writer<impl Write> {
    csv::WriterBuilder::new()
        .delimiter(b'\t')
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(writer)
}

const fn source_label(label: EvalLabel) -> &'static str {
    match label {
        EvalLabel::Clean => "0",
        EvalLabel::Toxic => "1",
    }
}

const fn evaluation_label(label: EvalLabel) -> &'static str {
    match label {
        EvalLabel::Clean => "clean",
        EvalLabel::Toxic => "toxic",
    }
}

const fn split_name(split: DatasetSplit) -> &'static str {
    match split {
        DatasetSplit::Development => "development",
        DatasetSplit::Validation => "validation",
        DatasetSplit::Test => "test",
    }
}

const fn provenance_status_name(status: ProvenanceStatus) -> &'static str {
    match status {
        ProvenanceStatus::Representative => "representative",
        ProvenanceStatus::Duplicate => "duplicate",
        ProvenanceStatus::LabelConflict => "label_conflict",
        ProvenanceStatus::UnsupportedLanguage => "unsupported_language",
        ProvenanceStatus::EmptyText => "empty_text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_group_id_collisions_before_conflict_removal() {
        let rows = vec![
            source_row("a", EvalLabel::Clean, "same"),
            source_row("b", EvalLabel::Toxic, "same"),
            source_row("c", EvalLabel::Clean, "other"),
        ];

        let error =
            prepare_textdetox_with_group_id(&rows, &BTreeSet::from(["EN".to_owned()]), |_, _| {
                "v1-collision".to_owned()
            })
            .expect_err("group ID collision");

        assert!(matches!(
            error,
            TextDetoxError::GroupCollision { group_id } if group_id == "v1-collision"
        ));
    }

    fn source_row(source_id: &str, label: EvalLabel, text: &str) -> TextDetoxSourceRow {
        TextDetoxSourceRow {
            source_id: source_id.to_owned(),
            language: TextDetoxLanguage::English,
            label,
            text: text.to_owned(),
        }
    }
}
