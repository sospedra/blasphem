//! One committed TSV file per language holds every corpus row for that language.
//!
//! The file is the single source of truth. Contributors edit it by hand, so the
//! format stays line oriented: one row per line, three tab separated columns,
//! and an escape rule that keeps a tab or a newline inside the text column.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use blasphem::{EvalLabel, Language, normalize_text};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    datasets::{DatasetSplit, PreparedCounts, PreparedRow},
    evidence::Sha256Digest,
    prepared_input::PreparedLanguageInput,
    source_manifest::{FrozenSource, FrozenSourceLock, SourceRecord},
};

pub const CORPUS_HEADER: [&str; 3] = ["split", "label", "text"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRow {
    pub split: DatasetSplit,
    pub label: EvalLabel,
    pub text: String,
}

#[derive(Debug, Error)]
pub enum CorpusError {
    #[error("cannot read the corpus file: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot read the corpus file {path}: {source}")]
    FileIo {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("the corpus header is wrong: expected {expected:?}, got {actual:?}")]
    Header {
        expected: Vec<String>,
        actual: Vec<String>,
    },
    #[error("line {line} has {actual} columns, expected 3")]
    ColumnCount { line: usize, actual: usize },
    #[error("line {line} has an unknown {field} value {value}")]
    UnknownValue {
        line: usize,
        field: &'static str,
        value: String,
    },
    #[error("line {line} contains a raw tab or newline in its text")]
    UnescapedText { line: usize },
    #[error("{language} line {line} repeats the text of line {first}")]
    DuplicateText {
        language: Language,
        line: usize,
        first: usize,
    },
    #[error("{language} line {line} repeats the normalized text of line {first}")]
    DuplicateNormalizedText {
        language: Language,
        line: usize,
        first: usize,
    },
    #[error("{language} line {line} sorts before line {previous}")]
    Unsorted {
        language: Language,
        line: usize,
        previous: usize,
    },
    #[error("{language} {split} rows changed: expected {expected}, got {actual}")]
    SealedDigest {
        language: Language,
        split: &'static str,
        expected: Sha256Digest,
        actual: Sha256Digest,
    },
    #[error("the evaluation lock has no sealed digests for {0}")]
    MissingSealedLanguage(Language),
}

#[must_use]
pub fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// Restores one text column value written by [`escape_text`].
///
/// # Errors
///
/// Returns an error when the column still holds a raw tab, newline, or
/// carriage return. Editors and diff tools break a line on a bare carriage
/// return, so the file may not carry one.
pub fn unescape_text(line: usize, value: &str) -> Result<String, CorpusError> {
    if value.contains(['\t', '\n', '\r']) {
        return Err(CorpusError::UnescapedText { line });
    }
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        push_escape(&mut output, characters.next());
    }
    Ok(output)
}

fn push_escape(output: &mut String, escaped: Option<char>) {
    match escaped {
        Some('t') => output.push('\t'),
        Some('n') => output.push('\n'),
        Some('r') => output.push('\r'),
        Some('\\') => output.push('\\'),
        other => {
            output.push('\\');
            if let Some(value) = other {
                output.push(value);
            }
        }
    }
}

/// The identifier the compiler carries for one corpus row.
///
/// The corpus stores no identifier column. The text decides the value, so the
/// identifier survives an insertion anywhere else in the file. `corpus verify`
/// rejects a repeated text, which keeps the value unique inside a language.
#[must_use]
pub fn row_source_id(language: Language, text: &str) -> String {
    let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
    format!("{}:{}", language.storage_code(), &digest[..12])
}

#[must_use]
pub fn corpus_path(root: &Path, language: Language) -> PathBuf {
    root.join(format!("{}.tsv", language.storage_code()))
}

#[must_use]
pub fn split_name(split: DatasetSplit) -> &'static str {
    match split {
        DatasetSplit::Development => "development",
        DatasetSplit::Validation => "validation",
        DatasetSplit::Test => "test",
    }
}

fn parse_split(line: usize, value: &str) -> Result<DatasetSplit, CorpusError> {
    match value {
        "development" => Ok(DatasetSplit::Development),
        "validation" => Ok(DatasetSplit::Validation),
        "test" => Ok(DatasetSplit::Test),
        _ => Err(CorpusError::UnknownValue {
            line,
            field: "split",
            value: value.to_owned(),
        }),
    }
}

#[must_use]
pub fn label_name(label: EvalLabel) -> &'static str {
    match label {
        EvalLabel::Clean => "clean",
        EvalLabel::Toxic => "toxic",
    }
}

fn parse_label(line: usize, value: &str) -> Result<EvalLabel, CorpusError> {
    value.parse().map_err(|_| CorpusError::UnknownValue {
        line,
        field: "label",
        value: value.to_owned(),
    })
}

/// Reads every row of one corpus file.
///
/// # Errors
///
/// Returns an error for a wrong header, a wrong column count, an unknown split
/// or label, or a text column that still holds a raw tab or newline.
pub fn parse_corpus(mut reader: impl Read) -> Result<Vec<CorpusRow>, CorpusError> {
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let mut lines = text.split('\n');
    check_header(lines.next().unwrap_or_default())?;

    let mut rows = Vec::new();
    for (offset, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        rows.push(parse_row(offset + 2, line)?);
    }
    Ok(rows)
}

fn check_header(line: &str) -> Result<(), CorpusError> {
    let actual = line.split('\t').collect::<Vec<_>>();
    if actual == CORPUS_HEADER {
        return Ok(());
    }
    Err(CorpusError::Header {
        expected: CORPUS_HEADER
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        actual: actual.into_iter().map(str::to_owned).collect(),
    })
}

fn parse_row(line: usize, value: &str) -> Result<CorpusRow, CorpusError> {
    let fields = value.split('\t').collect::<Vec<_>>();
    if fields.len() != CORPUS_HEADER.len() {
        return Err(CorpusError::ColumnCount {
            line,
            actual: fields.len(),
        });
    }
    Ok(CorpusRow {
        split: parse_split(line, fields[0])?,
        label: parse_label(line, fields[1])?,
        text: unescape_text(line, fields[2])?,
    })
}

/// Writes one corpus file: the header, then one line per row.
///
/// # Errors
///
/// Returns an error when the writer fails.
pub fn write_corpus(mut writer: impl Write, rows: &[CorpusRow]) -> Result<(), CorpusError> {
    writeln!(writer, "{}", CORPUS_HEADER.join("\t"))?;
    for row in rows {
        writeln!(
            writer,
            "{}\t{}\t{}",
            split_name(row.split),
            label_name(row.label),
            escape_text(&row.text),
        )?;
    }
    Ok(())
}

/// The line one row writes, used as the sort key and the digest input.
#[must_use]
pub fn row_line(row: &CorpusRow) -> String {
    format!(
        "{}\t{}\t{}",
        split_name(row.split),
        label_name(row.label),
        escape_text(&row.text)
    )
}

/// Hashes the rows of one split, in file order.
///
/// The digest covers the label and the escaped text, so a relabelled or
/// reworded sealed row changes it.
#[must_use]
pub fn split_digest(rows: &[CorpusRow], split: DatasetSplit) -> Sha256Digest {
    let mut hasher = Sha256::new();
    for row in rows.iter().filter(|row| row.split == split) {
        hasher.update(label_name(row.label).as_bytes());
        hasher.update(b"\t");
        hasher.update(escape_text(&row.text).as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
        .try_into()
        .expect("SHA-256 output is a valid digest")
}

/// Reads one corpus file from disk.
///
/// # Errors
///
/// Returns an error when the file is missing or malformed.
pub fn read_corpus_file(root: &Path, language: Language) -> Result<Vec<CorpusRow>, CorpusError> {
    let path = corpus_path(root, language);
    let file = File::open(&path).map_err(|source| CorpusError::FileIo {
        path: path.clone(),
        source,
    })?;
    parse_corpus(file)
}

/// Every locked source that belongs to one language, ordered by identifier.
///
/// The HurtLex lexicon has no corpus rows, so the language, not the row set,
/// selects the sources. `acquired_at_unix_seconds` is zero because a committed
/// corpus is never acquired at run time.
fn language_sources(lock: &FrozenSourceLock, language: Language) -> Vec<SourceRecord> {
    let mut sources = lock
        .sources
        .iter()
        .filter(|source| source.detector_language == language)
        .map(source_record)
        .collect::<Vec<_>>();
    sources.sort_by(|left, right| left.source_file_id.cmp(&right.source_file_id));
    sources
}

fn source_record(source: &FrozenSource) -> SourceRecord {
    SourceRecord {
        dataset: source.dataset,
        detector_language: source.detector_language,
        source_role: source.source_role,
        source_file_id: source.source_file_id.clone(),
        immutable_source_url: source.immutable_source_url.clone(),
        archive_member: source.archive_member.clone(),
        revision: source.revision.clone(),
        file_path: source.file_path.clone(),
        file_sha256: source.file_sha256.clone(),
        download_sha256: source.download_sha256.clone(),
        acquired_at_unix_seconds: 0,
        license_id: source.license_id.clone(),
        license_url: source.license_url.clone(),
        license_year: source.license_year,
        citation: source.citation.clone(),
        upstream_lineage: source.upstream_lineage.clone(),
        lineage_status: source.lineage_status,
    }
}

fn prepared_row(language: Language, row: &CorpusRow) -> PreparedRow {
    PreparedRow {
        detector_language: language,
        label: row.label,
        source_id: row_source_id(language, &row.text),
        text: row.text.clone(),
    }
}

fn rows_of(language: Language, rows: &[CorpusRow], split: DatasetSplit) -> Vec<PreparedRow> {
    rows.iter()
        .filter(|row| row.split == split)
        .map(|row| prepared_row(language, row))
        .collect()
}

/// Loads one language as the compiler's prepared input.
///
/// Test rows stay sealed: the returned value carries development and
/// validation only, exactly as the prepared loader did.
///
/// # Errors
///
/// Returns an error when the file is missing or malformed.
pub fn load_corpus_language(
    root: &Path,
    language: Language,
    lock: &FrozenSourceLock,
) -> Result<PreparedLanguageInput, CorpusError> {
    let rows = read_corpus_file(root, language)?;
    let development = rows_of(language, &rows, DatasetSplit::Development);
    let validation = rows_of(language, &rows, DatasetSplit::Validation);
    // A committed corpus holds the rows that survived import, so the three
    // import counters are zero. They described the prepare step, which the
    // reproduction path no longer runs.
    let counts = PreparedCounts {
        development: development.len(),
        validation: validation.len(),
        test: rows
            .iter()
            .filter(|row| row.split == DatasetSplit::Test)
            .count(),
        duplicates: 0,
        conflicts: 0,
        excluded: 0,
    };
    Ok(PreparedLanguageInput {
        language,
        development,
        validation,
        counts,
        sources: language_sources(lock, language),
    })
}

/// Loads one language's validation rows, leaving development and test unread.
///
/// # Errors
///
/// Returns an error when the file is missing or malformed.
pub fn load_corpus_validation(
    root: &Path,
    language: Language,
) -> Result<Vec<PreparedRow>, CorpusError> {
    let rows = read_corpus_file(root, language)?;
    Ok(rows_of(language, &rows, DatasetSplit::Validation))
}

/// Hashes every committed corpus file, in language order.
///
/// # Errors
///
/// Returns an error when a corpus file is unreadable.
pub fn corpus_digest(root: &Path) -> Result<Sha256Digest, CorpusError> {
    let mut hasher = Sha256::new();
    for language in Language::ALL {
        let path = corpus_path(root, language);
        let bytes = std::fs::read(&path).map_err(|source| CorpusError::FileIo {
            path: path.clone(),
            source,
        })?;
        hasher.update(language.storage_code().as_bytes());
        hasher.update(b"\t");
        hasher.update(format!("{:x}", Sha256::digest(&bytes)).as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!("{:x}", hasher.finalize())
        .try_into()
        .expect("SHA-256 output is a valid digest"))
}

/// What one `corpus verify` run inspected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpusReport {
    pub languages: usize,
    pub rows: usize,
}

fn check_rows(language: Language, rows: &[CorpusRow]) -> Result<(), CorpusError> {
    let mut texts = BTreeMap::new();
    let mut normalized = BTreeMap::new();
    let mut previous: Option<(usize, String)> = None;
    for (offset, row) in rows.iter().enumerate() {
        let line = offset + 2;
        let key = row_line(row);
        check_order(language, line, &key, previous.as_ref())?;
        check_unique_text(language, line, row, &mut texts, &mut normalized)?;
        previous = Some((line, key));
    }
    Ok(())
}

fn check_order(
    language: Language,
    line: usize,
    key: &str,
    previous: Option<&(usize, String)>,
) -> Result<(), CorpusError> {
    match previous {
        Some((earlier, value)) if value.as_str() > key => Err(CorpusError::Unsorted {
            language,
            line,
            previous: *earlier,
        }),
        _ => Ok(()),
    }
}

fn check_unique_text(
    language: Language,
    line: usize,
    row: &CorpusRow,
    texts: &mut BTreeMap<String, usize>,
    normalized: &mut BTreeMap<String, usize>,
) -> Result<(), CorpusError> {
    // The text decides the compiler's row identifier, so an exact repeat would
    // hand two rows the same identifier.
    if let Some(first) = texts.insert(row.text.clone(), line) {
        return Err(CorpusError::DuplicateText {
            language,
            line,
            first,
        });
    }
    // A punctuation-only message normalizes to nothing. Those rows carry no
    // lexical content, so they collide with each other by construction.
    let key = normalize_text(&row.text);
    if key.is_empty() {
        return Ok(());
    }
    match normalized.insert(key, line) {
        Some(first) => Err(CorpusError::DuplicateNormalizedText {
            language,
            line,
            first,
        }),
        None => Ok(()),
    }
}

fn check_sealed(
    language: Language,
    rows: &[CorpusRow],
    sealed: &crate::evaluation_lock::SealedLanguage,
) -> Result<(), CorpusError> {
    for (split, expected) in [
        (DatasetSplit::Validation, &sealed.validation_sha256),
        (DatasetSplit::Test, &sealed.test_sha256),
    ] {
        let actual = split_digest(rows, split);
        if actual != *expected {
            return Err(CorpusError::SealedDigest {
                language,
                split: split_name(split),
                expected: expected.clone(),
                actual,
            });
        }
    }
    Ok(())
}

/// Checks every committed corpus file against the seal.
///
/// # Errors
///
/// Returns the first failing language.
pub fn verify_corpus(
    root: &Path,
    evaluation: &crate::evaluation_lock::EvaluationLock,
) -> Result<CorpusReport, CorpusError> {
    let mut rows = 0;
    for language in Language::ALL {
        let parsed = read_corpus_file(root, language)?;
        check_rows(language, &parsed)?;
        let sealed = evaluation
            .languages
            .get(language.storage_code())
            .ok_or(CorpusError::MissingSealedLanguage(language))?;
        check_sealed(language, &parsed, sealed)?;
        rows += parsed.len();
    }
    Ok(CorpusReport {
        languages: Language::ALL.len(),
        rows,
    })
}
