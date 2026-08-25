use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::Path,
};

use blasphem::{EvalLabel, Language};
use sha2::{Digest, Sha256};

use crate::{
    datasets::{PreparedCounts, PreparedFileIdentity, PreparedManifest, PreparedRow},
    evidence::Sha256Digest,
    model_manifest::ModelSetError,
    publication::PREPARED_MANIFEST_SCHEMA_VERSION,
    source_manifest::SourceRecord,
};

const PREPARED_HEADER: [&str; 4] = ["detector_language", "label", "source_id", "text"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedLanguageInput {
    pub language: Language,
    pub development: Vec<PreparedRow>,
    pub validation: Vec<PreparedRow>,
    pub counts: PreparedCounts,
    pub sources: Vec<SourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedValidationInput {
    pub language: Language,
    pub validation: Vec<PreparedRow>,
    pub counts: PreparedCounts,
}

pub fn parse_prepared_manifest(reader: impl Read) -> Result<PreparedManifest, ModelSetError> {
    let manifest: PreparedManifest =
        serde_json::from_reader(reader).map_err(ModelSetError::PreparedManifestJson)?;
    if manifest.schema_version != PREPARED_MANIFEST_SCHEMA_VERSION {
        return Err(ModelSetError::InvalidPreparedManifestSchema {
            actual: manifest.schema_version,
        });
    }
    Ok(manifest)
}

pub fn load_prepared_language(
    root: &Path,
    language: Language,
) -> Result<PreparedLanguageInput, ModelSetError> {
    let manifest_path = root.join("manifest.json");
    let bytes = fs::read(&manifest_path).map_err(|source| ModelSetError::PreparedFileIo {
        path: manifest_path,
        source,
    })?;
    let manifest = parse_prepared_manifest(bytes.as_slice())?;
    validate_manifest_structure(&manifest)?;

    let counts = *manifest
        .language_counts
        .get(language.storage_code())
        .ok_or(ModelSetError::MissingPreparedLanguage(language))?;
    let sources = joined_language_sources(&manifest, language)?;
    let development = load_split(root, &manifest, language, "development")?;
    let validation = load_split(root, &manifest, language, "validation")?;
    reject_duplicate_row_ids(&development, &validation)?;

    Ok(PreparedLanguageInput {
        language,
        development,
        validation,
        counts,
        sources,
    })
}

/// Loads one validated validation split without opening development or test data.
///
/// # Errors
///
/// Returns an error for any malformed prepared manifest or validation file.
pub fn load_prepared_validation(
    root: &Path,
    language: Language,
) -> Result<PreparedValidationInput, ModelSetError> {
    let manifest_path = root.join("manifest.json");
    let bytes = fs::read(&manifest_path).map_err(|source| ModelSetError::PreparedFileIo {
        path: manifest_path,
        source,
    })?;
    let manifest = parse_prepared_manifest(bytes.as_slice())?;
    validate_manifest_structure(&manifest)?;
    let counts = *manifest
        .language_counts
        .get(language.storage_code())
        .ok_or(ModelSetError::MissingPreparedLanguage(language))?;
    let validation = load_split(root, &manifest, language, "validation")?;
    Ok(PreparedValidationInput {
        language,
        validation,
        counts,
    })
}

pub(crate) fn validate_manifest_structure(
    manifest: &PreparedManifest,
) -> Result<(), ModelSetError> {
    let expected_languages = Language::ALL
        .into_iter()
        .map(|language| language.storage_code().to_owned())
        .collect::<BTreeSet<_>>();
    for (field, actual) in [
        (
            "language_counts",
            manifest
                .language_counts
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
        ),
        (
            "language_sources",
            manifest
                .language_sources
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>(),
        ),
    ] {
        if actual != expected_languages {
            return Err(ModelSetError::PreparedLanguageKeySet { field });
        }
    }

    let mut expected_files = BTreeSet::new();
    for language in Language::ALL {
        for split in ["development", "validation", "test"] {
            expected_files.insert(format!("{}/{split}.tsv", language.storage_code()));
        }
    }
    let actual_files = manifest
        .prepared_files
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_files != expected_files {
        return Err(ModelSetError::PreparedFileKeySet);
    }

    for (path, identity) in &manifest.prepared_files {
        if identity.relative_path != *path {
            return Err(ModelSetError::PreparedIdentityPathMismatch {
                key: path.clone(),
                declared: identity.relative_path.clone(),
            });
        }
    }
    validate_source_joins(manifest)?;
    validate_split_counts(manifest)
}

fn validate_source_joins(manifest: &PreparedManifest) -> Result<(), ModelSetError> {
    let mut source_index = BTreeMap::new();
    for source in &manifest.sources {
        if source_index
            .insert(source.source_file_id.as_str(), source)
            .is_some()
        {
            return Err(ModelSetError::DuplicateSourceRecord(
                source.source_file_id.clone(),
            ));
        }
    }
    for language in Language::ALL {
        let identifiers = &manifest.language_sources[language.storage_code()];
        let mut unique = BTreeSet::new();
        for source_id in identifiers {
            if !unique.insert(source_id) {
                return Err(ModelSetError::DuplicateLanguageSourceId {
                    language,
                    source_id: source_id.clone(),
                });
            }
            let source = source_index.get(source_id.as_str()).ok_or_else(|| {
                ModelSetError::UnknownLanguageSource {
                    language,
                    source_id: source_id.clone(),
                }
            })?;
            if source.detector_language != language {
                return Err(ModelSetError::WrongLanguageSource {
                    expected: language,
                    actual: source.detector_language,
                    source_id: source_id.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_split_counts(manifest: &PreparedManifest) -> Result<(), ModelSetError> {
    for language in Language::ALL {
        let counts = manifest
            .language_counts
            .get(language.storage_code())
            .ok_or(ModelSetError::MissingPreparedLanguage(language))?;
        for (split, declared) in [
            ("development", counts.development),
            ("validation", counts.validation),
            ("test", counts.test),
        ] {
            let relative_path = format!("{}/{split}.tsv", language.storage_code());
            let identity = manifest
                .prepared_files
                .get(&relative_path)
                .ok_or_else(|| ModelSetError::MissingPreparedIdentity(relative_path.clone()))?;
            if declared != identity.rows {
                return Err(ModelSetError::PreparedSplitCountMismatch {
                    language,
                    split,
                    declared,
                    file_rows: identity.rows,
                });
            }
        }
    }
    Ok(())
}

fn joined_language_sources(
    manifest: &PreparedManifest,
    language: Language,
) -> Result<Vec<SourceRecord>, ModelSetError> {
    let index = manifest
        .sources
        .iter()
        .map(|source| (source.source_file_id.as_str(), source))
        .collect::<BTreeMap<_, _>>();
    manifest.language_sources[language.storage_code()]
        .iter()
        .map(|source_id| {
            index
                .get(source_id.as_str())
                .map(|source| (*source).clone())
                .ok_or_else(|| ModelSetError::UnknownLanguageSource {
                    language,
                    source_id: source_id.clone(),
                })
        })
        .collect()
}

fn load_split(
    root: &Path,
    manifest: &PreparedManifest,
    language: Language,
    split: &'static str,
) -> Result<Vec<PreparedRow>, ModelSetError> {
    let relative_path = format!("{}/{split}.tsv", language.storage_code());
    let identity = manifest
        .prepared_files
        .get(&relative_path)
        .ok_or_else(|| ModelSetError::MissingPreparedIdentity(relative_path.clone()))?;
    let path = root.join(&relative_path);
    let bytes = fs::read(&path).map_err(|source| ModelSetError::PreparedFileIo {
        path: path.clone(),
        source,
    })?;
    let actual_digest = sha256(&bytes);
    if actual_digest != identity.sha256 {
        return Err(ModelSetError::PreparedDigestMismatch {
            path: relative_path,
            expected: identity.sha256.clone(),
            actual: actual_digest,
        });
    }
    parse_split(&bytes, identity, language, split)
}

fn parse_split(
    bytes: &[u8],
    identity: &PreparedFileIdentity,
    language: Language,
    split: &'static str,
) -> Result<Vec<PreparedRow>, ModelSetError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(true)
        .from_reader(bytes);
    let actual_header = reader
        .headers()
        .map_err(|source| ModelSetError::PreparedCsv {
            path: identity.relative_path.clone(),
            source,
        })?
        .iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if actual_header != PREPARED_HEADER {
        return Err(ModelSetError::PreparedHeaderMismatch {
            language,
            split,
            actual: actual_header,
        });
    }

    let mut rows = Vec::with_capacity(identity.rows);
    let mut clean_rows = 0_usize;
    let mut toxic_rows = 0_usize;
    for result in reader.records() {
        let record = result.map_err(|source| ModelSetError::PreparedCsv {
            path: identity.relative_path.clone(),
            source,
        })?;
        let actual =
            record[0]
                .parse::<Language>()
                .map_err(|_| ModelSetError::InvalidPreparedLanguage {
                    path: identity.relative_path.clone(),
                    value: record[0].to_owned(),
                })?;
        let source_id = record[2].to_owned();
        if actual != language {
            return Err(ModelSetError::PreparedRowLanguageMismatch {
                expected: language,
                actual,
                split,
                source_id,
            });
        }
        let label = record[1].parse::<EvalLabel>().map_err(|value| {
            ModelSetError::InvalidPreparedLabel {
                path: identity.relative_path.clone(),
                value,
            }
        })?;
        match label {
            EvalLabel::Clean => clean_rows += 1,
            EvalLabel::Toxic => toxic_rows += 1,
        }
        rows.push(PreparedRow {
            detector_language: actual,
            label,
            source_id,
            text: record[3].to_owned(),
        });
    }
    if rows.len() != identity.rows
        || clean_rows != identity.clean_rows
        || toxic_rows != identity.toxic_rows
    {
        return Err(ModelSetError::PreparedFileCountMismatch {
            path: identity.relative_path.clone(),
            rows: rows.len(),
            clean_rows,
            toxic_rows,
        });
    }
    Ok(rows)
}

fn reject_duplicate_row_ids(
    development: &[PreparedRow],
    validation: &[PreparedRow],
) -> Result<(), ModelSetError> {
    let mut identifiers = BTreeSet::new();
    for row in development.iter().chain(validation) {
        if !identifiers.insert(row.source_id.as_str()) {
            return Err(ModelSetError::DuplicatePreparedSourceId(
                row.source_id.clone(),
            ));
        }
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("SHA-256 output is a valid digest")
}
