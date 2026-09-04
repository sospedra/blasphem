use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use blasphem::{EvalLabel, Language};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::atomic_publish::{AtomicPublishError, atomic_publish_noreplace};
use crate::datasets::{
    DatasetId, DatasetSplit, ExclusionReason, InclusionStatus, PreparedCounts,
    PreparedFileIdentity, PreparedLanguage, PreparedManifest, PreparedRow, ProvenanceRow,
    SourceSplit,
};
use crate::evidence::Sha256Digest;
use crate::source_manifest::{SourceObservation, SourceRecord};

pub const PREPARED_MANIFEST_SCHEMA_VERSION: &str = "prepared-v1";

const PREPARED_HEADER: [&str; 4] = ["detector_language", "label", "source_id", "text"];
const PROVENANCE_HEADER: [&str; 27] = [
    "dataset",
    "source_file_id",
    "source_id",
    "immutable_source_url",
    "archive_member",
    "revision",
    "file_path",
    "file_sha256",
    "acquired_at_unix_seconds",
    "license_id",
    "license_url",
    "citation",
    "upstream_lineage",
    "lineage_status",
    "source_language_code",
    "detector_language_code",
    "source_label",
    "detector_label",
    "label_conversion_version",
    "split_version",
    "normalization_version",
    "canonical_group_id",
    "representative_source_id",
    "source_split",
    "detector_split",
    "inclusion_status",
    "exclusion_reason",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedPublication {
    pub path: PathBuf,
    pub manifest: PreparedManifest,
}

pub type PreparedPublicationResult = Result<PreparedPublication, PreparedPublicationError>;

#[derive(Debug, Error)]
pub enum PreparedPublicationError {
    #[error("prepared output directory already exists: {0}")]
    ExistingOutput(PathBuf),
    #[error("prepared output directory must have a UTF-8 file name: {0}")]
    InvalidOutput(PathBuf),
    #[error("prepared staging directory already exists: {0}")]
    ExistingStaging(PathBuf),
    #[error("prepared publication file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot write prepared TSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("cannot write prepared manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("prepared publication misses language {0}")]
    MissingPreparedLanguage(String),
    #[error("prepared publication repeats language {0}")]
    DuplicatePreparedLanguage(String),
    #[error("source observation repeats source file identifier {0}")]
    DuplicateSourceRecord(String),
    #[error("source observation misses a source record for language {0}")]
    MissingLanguageSource(String),
    #[error("prepared provenance repeats source identifier {0}")]
    DuplicateProvenanceSourceId(String),
    #[error("prepared rows repeat source identifier {0}")]
    DuplicatePreparedSourceId(String),
    #[error("prepared row {source_id} has detector language {actual:?}; expected {expected:?}")]
    PreparedRowLanguageMismatch {
        source_id: String,
        actual: Language,
        expected: Language,
    },
    #[error("prepared provenance has a blank required field: {0}")]
    BlankRequiredProvenanceField(&'static str),
    #[error("prepared provenance source is unknown or mismatched: {0}")]
    UnknownOrMismatchedSource(String),
    #[error("prepared provenance has incomplete source metadata: {0}")]
    IncompleteSourceMetadata(String),
    #[error("prepared provenance has no label conversion version for dataset {0}")]
    MissingLabelConversionVersion(DatasetId),
    #[error("prepared provenance is invalid: {0}")]
    InvalidProvenance(String),
    #[error("prepared rows do not match included provenance rows for language {0}")]
    PreparedRowMismatch(String),
    #[error("prepared {split} split has no {label} rows for language {language}")]
    MissingClass {
        language: String,
        split: &'static str,
        label: &'static str,
    },
    #[error("atomic no-replace publication is unsupported on this target")]
    UnsupportedAtomicPublish,
}

pub fn publish_prepared(
    output: &Path,
    languages: &[PreparedLanguage],
    observation: &SourceObservation,
) -> PreparedPublicationResult {
    if output.exists() {
        return Err(PreparedPublicationError::ExistingOutput(output.to_owned()));
    }
    let (languages, mut manifest) = prepare_publication(languages, observation)?;
    let staging = prepared_staging_path(output)?;
    if staging.exists() {
        return Err(PreparedPublicationError::ExistingStaging(staging));
    }
    fs::create_dir(&staging)?;
    let result = (|| {
        for language in &languages {
            write_language(&staging, language, &mut manifest.prepared_files)?;
        }
        write_prepared_provenance(&staging, &languages)?;
        write_prepared_manifest(&staging, &manifest)?;
        atomic_publish_noreplace(&staging, output)
            .map_err(|error| map_prepared_atomic_error(error, output))?;
        Ok(())
    })();
    if result.is_err() && staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    result.map(|()| PreparedPublication {
        path: output.to_owned(),
        manifest,
    })
}

fn prepare_publication(
    languages: &[PreparedLanguage],
    observation: &SourceObservation,
) -> Result<(Vec<PreparedLanguage>, PreparedManifest), PreparedPublicationError> {
    let sources = sorted_sources(observation)?;
    let source_records = source_record_index(&sources)?;
    let language_sources = language_sources(&sources)?;
    let mut languages = prepared_language_index(languages)?;
    let mut language_counts = Language::ALL
        .into_iter()
        .map(|language| {
            (
                language.storage_code().to_owned(),
                PreparedCounts::default(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut source_label_counts = BTreeMap::new();
    let mut detector_label_counts = BTreeMap::new();
    let mut source_split_counts = BTreeMap::new();
    let mut detector_split_counts = BTreeMap::new();
    let mut inclusion_status_counts = BTreeMap::new();
    let mut exclusion_reason_counts = BTreeMap::new();
    let mut source_ids = BTreeSet::new();
    let mut source_rows = 0;

    for language in languages.values_mut() {
        hydrate_language(
            language,
            &source_records,
            &mut language_counts,
            &mut source_label_counts,
            &mut detector_label_counts,
            &mut source_split_counts,
            &mut detector_split_counts,
            &mut inclusion_status_counts,
            &mut exclusion_reason_counts,
            &mut source_ids,
            &mut source_rows,
        )?;
        validate_prepared_rows(language)?;
        validate_split_classes(language)?;
        sort_prepared_rows(language);
    }

    Ok((
        languages.into_values().collect(),
        PreparedManifest {
            schema_version: PREPARED_MANIFEST_SCHEMA_VERSION.to_owned(),
            sources,
            language_sources,
            language_counts,
            source_rows,
            source_label_counts,
            detector_label_counts,
            source_split_counts,
            detector_split_counts,
            inclusion_status_counts,
            exclusion_reason_counts,
            prepared_files: BTreeMap::new(),
        },
    ))
}

fn sorted_sources(
    observation: &SourceObservation,
) -> Result<Vec<SourceRecord>, PreparedPublicationError> {
    let mut sources = observation.sources.clone();
    sources.sort_by(|left, right| left.source_file_id.cmp(&right.source_file_id));
    for source in &sources {
        validate_source_metadata(source)?;
    }
    Ok(sources)
}

fn source_record_index(
    sources: &[SourceRecord],
) -> Result<BTreeMap<(DatasetId, String, Language), SourceRecord>, PreparedPublicationError> {
    let mut source_file_ids = BTreeSet::new();
    let mut records = BTreeMap::new();
    for source in sources {
        if !source_file_ids.insert(source.source_file_id.clone()) {
            return Err(PreparedPublicationError::DuplicateSourceRecord(
                source.source_file_id.clone(),
            ));
        }
        let key = (
            source.dataset,
            source.source_file_id.clone(),
            source.detector_language,
        );
        if records.insert(key, source.clone()).is_some() {
            return Err(PreparedPublicationError::DuplicateSourceRecord(
                source.source_file_id.clone(),
            ));
        }
    }
    Ok(records)
}

fn language_sources(
    sources: &[SourceRecord],
) -> Result<BTreeMap<String, Vec<String>>, PreparedPublicationError> {
    let mut values = Language::ALL
        .into_iter()
        .map(|language| (language.storage_code().to_owned(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for source in sources {
        values
            .get_mut(source.detector_language.storage_code())
            .expect("all language codes are initialized")
            .push(source.source_file_id.clone());
    }
    for (language, identifiers) in &mut values {
        identifiers.sort();
        identifiers.dedup();
        if identifiers.is_empty() {
            return Err(PreparedPublicationError::MissingLanguageSource(
                language.clone(),
            ));
        }
    }
    Ok(values)
}

fn prepared_language_index(
    languages: &[PreparedLanguage],
) -> Result<BTreeMap<Language, PreparedLanguage>, PreparedPublicationError> {
    let mut values = BTreeMap::new();
    for language in languages {
        if values.insert(language.language, language.clone()).is_some() {
            return Err(PreparedPublicationError::DuplicatePreparedLanguage(
                language.language.code().to_owned(),
            ));
        }
    }
    for language in Language::ALL {
        if !values.contains_key(&language) {
            return Err(PreparedPublicationError::MissingPreparedLanguage(
                language.code().to_owned(),
            ));
        }
    }
    Ok(values)
}

#[allow(clippy::too_many_arguments)]
fn hydrate_language(
    language: &mut PreparedLanguage,
    sources: &BTreeMap<(DatasetId, String, Language), SourceRecord>,
    language_counts: &mut BTreeMap<String, PreparedCounts>,
    source_label_counts: &mut BTreeMap<String, usize>,
    detector_label_counts: &mut BTreeMap<String, usize>,
    source_split_counts: &mut BTreeMap<String, usize>,
    detector_split_counts: &mut BTreeMap<String, usize>,
    inclusion_status_counts: &mut BTreeMap<String, usize>,
    exclusion_reason_counts: &mut BTreeMap<String, usize>,
    source_ids: &mut BTreeSet<String>,
    source_rows: &mut usize,
) -> Result<(), PreparedPublicationError> {
    for row in &mut language.provenance {
        validate_required_provenance_fields(row)?;
        if row.detector_language_code.as_deref() != Some(language.language.storage_code()) {
            return Err(PreparedPublicationError::UnknownOrMismatchedSource(
                row.source_id.clone(),
            ));
        }
        if !source_ids.insert(row.source_id.clone()) {
            return Err(PreparedPublicationError::DuplicateProvenanceSourceId(
                row.source_id.clone(),
            ));
        }
        let source = sources
            .get(&(row.dataset, row.source_file_id.clone(), language.language))
            .ok_or_else(|| {
                PreparedPublicationError::UnknownOrMismatchedSource(row.source_id.clone())
            })?;
        hydrate_provenance(row, source)?;
        row.validate()
            .map_err(|error| PreparedPublicationError::InvalidProvenance(error.to_string()))?;
        *source_rows += 1;
        increment(
            source_label_counts,
            format!(
                "{}/{}/{}",
                row.dataset, row.source_language_code, row.source_label
            ),
        );
        if let Some(label) = row.detector_label {
            increment(
                detector_label_counts,
                format!("{}/{}", language.language.storage_code(), label_name(label)),
            );
        }
        increment(
            source_split_counts,
            format!("{}/{}", row.dataset, source_split_name(row.source_split)),
        );
        if let Some(split) = row.detector_split {
            increment(
                detector_split_counts,
                format!("{}/{}", language.language.storage_code(), split_name(split)),
            );
        }
        increment(
            inclusion_status_counts,
            inclusion_status_name(row.inclusion_status).to_owned(),
        );
        if let Some(reason) = row.exclusion_reason {
            increment(
                exclusion_reason_counts,
                exclusion_reason_name(reason).to_owned(),
            );
        }
        let counts = language_counts
            .get_mut(language.language.storage_code())
            .expect("all language codes are initialized");
        if row.inclusion_status == InclusionStatus::Included
            && let Some(split) = row.detector_split
        {
            increment_split_count(counts, split);
        }
        if row.inclusion_status == InclusionStatus::Excluded {
            counts.excluded += 1;
            if row.exclusion_reason == Some(ExclusionReason::Duplicate)
                || row.exclusion_reason == Some(ExclusionReason::SealedBaselineDuplicate)
            {
                counts.duplicates += 1;
            }
            if row.exclusion_reason == Some(ExclusionReason::LabelConflict) {
                counts.conflicts += 1;
            }
        }
    }
    language
        .provenance
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    Ok(())
}

fn validate_required_provenance_fields(
    row: &ProvenanceRow,
) -> Result<(), PreparedPublicationError> {
    for (field, value) in [
        ("source_id", row.source_id.as_str()),
        ("source_language_code", row.source_language_code.as_str()),
        ("source_label", row.source_label.as_str()),
        ("split_version", row.split_version.as_str()),
        ("normalization_version", row.normalization_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(PreparedPublicationError::BlankRequiredProvenanceField(
                field,
            ));
        }
    }
    Ok(())
}

fn hydrate_provenance(
    row: &mut ProvenanceRow,
    source: &SourceRecord,
) -> Result<(), PreparedPublicationError> {
    validate_source_metadata(source)?;
    row.immutable_source_url = source.immutable_source_url.clone();
    row.archive_member = source.archive_member.clone();
    row.revision = source.revision.clone();
    row.file_path = source.file_path.clone();
    row.file_sha256 = source.file_sha256.clone();
    row.acquired_at_unix_seconds = source.acquired_at_unix_seconds;
    row.license_id = source.license_id.clone();
    row.license_url = source.license_url.clone();
    row.citation = source.citation.clone();
    row.upstream_lineage = source.upstream_lineage.clone();
    row.lineage_status = source.lineage_status;
    row.label_conversion_version = label_conversion_version(row.dataset)
        .ok_or(PreparedPublicationError::MissingLabelConversionVersion(
            row.dataset,
        ))?
        .to_owned();
    Ok(())
}

fn validate_source_metadata(source: &SourceRecord) -> Result<(), PreparedPublicationError> {
    if source.source_file_id.is_empty()
        || source.immutable_source_url.is_empty()
        || source.file_path.is_empty()
        || source.acquired_at_unix_seconds == 0
        || source.license_id.is_empty()
        || source.license_url.is_empty()
        || source.citation.is_empty()
        || source.upstream_lineage.is_empty()
    {
        return Err(PreparedPublicationError::IncompleteSourceMetadata(
            source.source_file_id.clone(),
        ));
    }
    Ok(())
}

fn label_conversion_version(dataset: DatasetId) -> Option<&'static str> {
    match dataset {
        DatasetId::TextDetox => Some("textdetox-binary-v1"),
        DatasetId::IbrohimBudi => Some("ibrohim-budi-hs-or-abusive-v1"),
        DatasetId::ToldBr => Some("told-br-annotator-consensus-v1"),
        DatasetId::OffensEvalTr => Some("offenseval-tr-off-not-v1"),
        DatasetId::ViHos => Some("vihos-span-presence-v1"),
        DatasetId::KMHas => Some("k-mhas-clean-8-toxic-0-7-v1"),
        DatasetId::GermEval2018 => Some("germeval-2018-coarse-v1"),
        DatasetId::Community => Some("community-binary-v1"),
        DatasetId::Lexicon => None,
    }
}

fn validate_prepared_rows(language: &PreparedLanguage) -> Result<(), PreparedPublicationError> {
    let mut source_ids = BTreeSet::new();
    for (_, row) in all_prepared_rows(language) {
        if row.detector_language != language.language {
            return Err(PreparedPublicationError::PreparedRowLanguageMismatch {
                source_id: row.source_id.clone(),
                actual: row.detector_language,
                expected: language.language,
            });
        }
        if !source_ids.insert(row.source_id.as_str()) {
            return Err(PreparedPublicationError::DuplicatePreparedSourceId(
                row.source_id.clone(),
            ));
        }
    }
    let included = language
        .provenance
        .iter()
        .filter(|row| row.inclusion_status == InclusionStatus::Included)
        .map(|row| {
            (
                row.source_id.as_str(),
                row.detector_label.map(label_name),
                row.detector_split.map(split_name),
            )
        })
        .collect::<BTreeSet<_>>();
    let prepared = all_prepared_rows(language)
        .map(|(split, row)| {
            (
                row.source_id.as_str(),
                Some(label_name(row.label)),
                Some(split_name(split)),
            )
        })
        .collect::<BTreeSet<_>>();
    if included != prepared {
        return Err(PreparedPublicationError::PreparedRowMismatch(
            language.language.code().to_owned(),
        ));
    }
    Ok(())
}

fn sort_prepared_rows(language: &mut PreparedLanguage) {
    language
        .development
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    language
        .validation
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
    language
        .test
        .sort_by(|left, right| left.source_id.cmp(&right.source_id));
}

fn validate_split_classes(language: &PreparedLanguage) -> Result<(), PreparedPublicationError> {
    for (split, rows, minimum) in [
        ("development", language.development.as_slice(), 1),
        ("validation", language.validation.as_slice(), 300),
        ("test", language.test.as_slice(), 300),
    ] {
        for label in [EvalLabel::Clean, EvalLabel::Toxic] {
            if rows.iter().filter(|row| row.label == label).count() < minimum {
                return Err(PreparedPublicationError::MissingClass {
                    language: language.language.code().to_owned(),
                    split,
                    label: label_name(label),
                });
            }
        }
    }
    Ok(())
}

fn write_language(
    staging: &Path,
    language: &PreparedLanguage,
    files: &mut BTreeMap<String, PreparedFileIdentity>,
) -> Result<(), PreparedPublicationError> {
    let code = language.language.storage_code();
    let directory = staging.join(code);
    fs::create_dir(&directory)?;
    for (name, rows) in [
        ("development", language.development.as_slice()),
        ("validation", language.validation.as_slice()),
        ("test", language.test.as_slice()),
    ] {
        let relative_path = format!("{code}/{name}.tsv");
        let identity =
            write_prepared_tsv(&directory.join(format!("{name}.tsv")), &relative_path, rows)?;
        files.insert(relative_path, identity);
    }
    sync_directory(&directory)?;
    Ok(())
}

fn write_prepared_tsv(
    path: &Path,
    relative_path: &str,
    rows: &[PreparedRow],
) -> Result<PreparedFileIdentity, PreparedPublicationError> {
    let file = File::create(path)?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(HashingFile::new(file));
    writer.write_record(PREPARED_HEADER)?;
    let mut clean_rows = 0;
    let mut toxic_rows = 0;
    for row in rows {
        writer.write_record([
            row.detector_language.storage_code(),
            label_name(row.label),
            row.source_id.as_str(),
            row.text.as_str(),
        ])?;
        match row.label {
            EvalLabel::Clean => clean_rows += 1,
            EvalLabel::Toxic => toxic_rows += 1,
        }
    }
    writer.flush()?;
    let mut hashing = writer.into_inner().map_err(|error| error.into_error())?;
    hashing.file.flush()?;
    hashing.file.sync_all()?;
    let sha256 = digest(hashing.hasher);
    Ok(PreparedFileIdentity {
        relative_path: relative_path.to_owned(),
        sha256,
        rows: rows.len(),
        clean_rows,
        toxic_rows,
    })
}

fn write_prepared_provenance(
    staging: &Path,
    languages: &[PreparedLanguage],
) -> Result<(), PreparedPublicationError> {
    let mut rows = languages
        .iter()
        .flat_map(|language| language.provenance.iter())
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.source_id.cmp(&right.source_id));
    let file = File::create(staging.join("provenance.tsv"))?;
    let mut writer = csv::WriterBuilder::new()
        .delimiter(b'\t')
        .terminator(csv::Terminator::Any(b'\n'))
        .from_writer(file);
    writer.write_record(PROVENANCE_HEADER)?;
    for row in rows {
        let lineage = serde_json::to_string(&row.upstream_lineage)?;
        writer.write_record([
            row.dataset.to_string(),
            row.source_file_id.clone(),
            row.source_id.clone(),
            row.immutable_source_url.clone(),
            row.archive_member.clone().unwrap_or_default(),
            row.revision.clone().unwrap_or_default(),
            row.file_path.clone(),
            row.file_sha256.to_string(),
            row.acquired_at_unix_seconds.to_string(),
            row.license_id.clone(),
            row.license_url.clone(),
            row.citation.clone(),
            lineage,
            lineage_status_name(row.lineage_status).to_owned(),
            row.source_language_code.clone(),
            row.detector_language_code.clone().unwrap_or_default(),
            row.source_label.clone(),
            row.detector_label.map(label_name).unwrap_or("").to_owned(),
            row.label_conversion_version.clone(),
            row.split_version.clone(),
            row.normalization_version.clone(),
            row.canonical_group_id.clone().unwrap_or_default(),
            row.representative_source_id.clone().unwrap_or_default(),
            source_split_name(row.source_split).to_owned(),
            row.detector_split.map(split_name).unwrap_or("").to_owned(),
            inclusion_status_name(row.inclusion_status).to_owned(),
            row.exclusion_reason
                .map(exclusion_reason_name)
                .unwrap_or("")
                .to_owned(),
        ])?;
    }
    writer.flush()?;
    let mut file = writer.into_inner().map_err(|error| error.into_error())?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

fn write_prepared_manifest(
    staging: &Path,
    manifest: &PreparedManifest,
) -> Result<(), PreparedPublicationError> {
    let mut file = File::create(staging.join("manifest.json"))?;
    serde_json::to_writer_pretty(&mut file, manifest)?;
    file.write_all(b"\n")?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

fn prepared_staging_path(output: &Path) -> Result<PathBuf, PreparedPublicationError> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PreparedPublicationError::InvalidOutput(output.to_owned()))?;
    Ok(parent.join(format!(".{name}.staging-{}", std::process::id())))
}

fn map_prepared_atomic_error(error: AtomicPublishError, output: &Path) -> PreparedPublicationError {
    match error {
        AtomicPublishError::DestinationExists => {
            PreparedPublicationError::ExistingOutput(output.to_owned())
        }
        AtomicPublishError::Unsupported => PreparedPublicationError::UnsupportedAtomicPublish,
        AtomicPublishError::Rename(source)
        | AtomicPublishError::StagingSync(source)
        | AtomicPublishError::ParentSync(source)
        | AtomicPublishError::Cleanup(source) => PreparedPublicationError::Io(source),
    }
}

fn sync_directory(path: &Path) -> Result<(), PreparedPublicationError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

struct HashingFile {
    file: File,
    hasher: Sha256,
}

impl HashingFile {
    fn new(file: File) -> Self {
        Self {
            file,
            hasher: Sha256::new(),
        }
    }
}

impl Write for HashingFile {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let bytes = self.file.write(buffer)?;
        self.hasher.update(&buffer[..bytes]);
        Ok(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn digest(hasher: Sha256) -> Sha256Digest {
    format!("{:x}", hasher.finalize())
        .try_into()
        .expect("SHA-256 output is a valid digest")
}

fn all_prepared_rows(
    language: &PreparedLanguage,
) -> impl Iterator<Item = (DatasetSplit, &PreparedRow)> {
    [
        (DatasetSplit::Development, language.development.as_slice()),
        (DatasetSplit::Validation, language.validation.as_slice()),
        (DatasetSplit::Test, language.test.as_slice()),
    ]
    .into_iter()
    .flat_map(|(split, rows)| rows.iter().map(move |row| (split, row)))
}

fn increment(values: &mut BTreeMap<String, usize>, key: String) {
    *values.entry(key).or_default() += 1;
}

fn increment_split_count(counts: &mut PreparedCounts, split: DatasetSplit) {
    match split {
        DatasetSplit::Development => counts.development += 1,
        DatasetSplit::Validation => counts.validation += 1,
        DatasetSplit::Test => counts.test += 1,
    }
}

const fn label_name(label: EvalLabel) -> &'static str {
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

const fn source_split_name(split: SourceSplit) -> &'static str {
    match split {
        SourceSplit::Unsplit => "unsplit",
        SourceSplit::Train => "train",
        SourceSplit::Development => "development",
        SourceSplit::Validation => "validation",
        SourceSplit::Test => "test",
    }
}

const fn lineage_status_name(status: crate::datasets::LineageStatus) -> &'static str {
    match status {
        crate::datasets::LineageStatus::Resolved => "resolved",
        crate::datasets::LineageStatus::Unresolved => "unresolved",
    }
}

const fn inclusion_status_name(status: InclusionStatus) -> &'static str {
    match status {
        InclusionStatus::Included => "included",
        InclusionStatus::Excluded => "excluded",
    }
}

const fn exclusion_reason_name(reason: ExclusionReason) -> &'static str {
    match reason {
        ExclusionReason::AmbiguousLabel => "ambiguous_label",
        ExclusionReason::AuditOnly => "audit_only",
        ExclusionReason::Duplicate => "duplicate",
        ExclusionReason::EmptyText => "empty_text",
        ExclusionReason::LabelConflict => "label_conflict",
        ExclusionReason::SealedBaselineDuplicate => "sealed_baseline_duplicate",
        ExclusionReason::UnsupportedLanguage => "unsupported_language",
    }
}
