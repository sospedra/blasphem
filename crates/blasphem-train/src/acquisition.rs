use std::{
    collections::BTreeSet,
    fmt, fs,
    fs::File,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use zip::ZipArchive;

use crate::atomic_publish::{AtomicPublishError, atomic_publish_noreplace};
use crate::datasets::textdetox::{
    TEXTDETOX_CODES, TEXTDETOX_REVISION, TextDetoxError, TextDetoxSourceRow, parse_textdetox_page,
    textdetox_rows_url, write_textdetox_source_tsv,
};
use crate::datasets::{DatasetId, LineageStatus};
use crate::evidence::Sha256Digest;
use crate::source_manifest::{
    FrozenSource, FrozenSourceLock, SOURCE_LOCK_SCHEMA_VERSION, SOURCE_OBSERVATION_SCHEMA_VERSION,
    SourceCatalog, SourceObservation, SourceRecord, SourceRequest,
};

pub const TEXTDETOX_REVISION_URL: &str =
    "https://huggingface.co/api/datasets/textdetox/multilingual_toxicity_dataset/revision/main";

pub const MAX_ARCHIVE_MEMBER_BYTES: usize = 67_108_864;
pub const MAX_SOURCE_DOWNLOAD_BYTES: usize = 268_435_456;

#[derive(Debug, Error)]
pub enum SourceAcquisitionError {
    #[error("source output already exists: {0}")]
    ExistingOutput(PathBuf),
    #[error("source source-file identifier repeats: {0}")]
    DuplicateSourceFileId(String),
    #[error("source source-file identifier is missing: {0}")]
    MissingSourceFileId(String),
    #[error("source observation has an extra source-file identifier: {0}")]
    ExtraSourceFileId(String),
    #[error("source {0} has unresolved lineage outside Chinese or French TextDetox")]
    InvalidUnresolvedLineage(String),
    #[error("source archive member is missing: {0}")]
    MissingArchiveMember(String),
    #[error("source archive member repeats: {0}")]
    DuplicateArchiveMember(String),
    #[error("source archive member exceeds 67108864 bytes: {0}")]
    ArchiveMemberTooLarge(String),
    #[error("source archive member cannot be read: {0}")]
    ArchiveMemberRead(String),
    #[error("cannot open source archive: {0}")]
    Archive(#[from] zip::result::ZipError),
    #[error("source {source_file_id} digest differs from the frozen lock")]
    DigestMismatch { source_file_id: String },
    #[error("source {source_file_id} download digest differs from the frozen lock")]
    DownloadDigestMismatch { source_file_id: String },
    #[error("TextDetox source {0} has no Parquet download digest")]
    MissingTextDetoxDownloadDigest(String),
    #[error("TextDetox source {0} has an invalid pinned Parquet identity")]
    InvalidTextDetoxParquetIdentity(String),
    #[error("TextDetox source records require separate Parquet bytes")]
    LegacyTextDetoxSourceRecord,
    #[error("source {source_file_id} identity differs from the frozen lock")]
    IdentityMismatch { source_file_id: String },
    #[error("cannot write source data: {0}")]
    Io(#[from] std::io::Error),
    #[error("cannot encode source JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn extract_archive_member(
    archive_bytes: &[u8],
    expected_member: &str,
) -> Result<Vec<u8>, SourceAcquisitionError> {
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes))?;
    let mut member_names = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let member = archive.by_index(index)?;
        member_names.push((index, member.name().to_owned()));
    }
    let index = select_exact_member_index(expected_member, &member_names)?;
    let member = archive.by_index(index)?;
    if member.size() > MAX_ARCHIVE_MEMBER_BYTES as u64 {
        return Err(SourceAcquisitionError::ArchiveMemberTooLarge(
            expected_member.to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(member.size() as usize);
    let mut limited = member.take(MAX_ARCHIVE_MEMBER_BYTES as u64 + 1);
    limited
        .read_to_end(&mut bytes)
        .map_err(|_| SourceAcquisitionError::ArchiveMemberRead(expected_member.to_owned()))?;
    if bytes.len() > MAX_ARCHIVE_MEMBER_BYTES {
        return Err(SourceAcquisitionError::ArchiveMemberTooLarge(
            expected_member.to_owned(),
        ));
    }
    Ok(bytes)
}

fn select_exact_member_index(
    expected_member: &str,
    member_names: &[(usize, String)],
) -> Result<usize, SourceAcquisitionError> {
    let mut selected_index = None;
    for (index, member_name) in member_names {
        if member_name == expected_member && selected_index.replace(*index).is_some() {
            return Err(SourceAcquisitionError::DuplicateArchiveMember(
                expected_member.to_owned(),
            ));
        }
    }
    selected_index
        .ok_or_else(|| SourceAcquisitionError::MissingArchiveMember(expected_member.to_owned()))
}

pub fn source_record_from_request(
    request: &SourceRequest,
    immutable_source_url: String,
    revision: Option<String>,
    downloaded_bytes: &[u8],
    acquired_at_unix_seconds: u64,
) -> Result<SourceRecord, SourceAcquisitionError> {
    if request.dataset == DatasetId::TextDetox {
        return Err(SourceAcquisitionError::LegacyTextDetoxSourceRecord);
    }
    source_record_from_request_with_download(
        request,
        immutable_source_url,
        revision,
        downloaded_bytes,
        downloaded_bytes,
        acquired_at_unix_seconds,
    )
}

pub fn source_record_from_request_with_download(
    request: &SourceRequest,
    immutable_source_url: String,
    revision: Option<String>,
    downloaded_bytes: &[u8],
    canonical_bytes: &[u8],
    acquired_at_unix_seconds: u64,
) -> Result<SourceRecord, SourceAcquisitionError> {
    validate_lineage(
        request.dataset,
        request.detector_language,
        &request.source_file_id,
        request.lineage_status,
    )?;
    let bytes = match request.archive_member.as_deref() {
        Some(member) => extract_archive_member(canonical_bytes, member)?,
        None => canonical_bytes.to_vec(),
    };
    Ok(SourceRecord {
        dataset: request.dataset,
        detector_language: request.detector_language,
        source_role: request.source_role,
        source_file_id: request.source_file_id.clone(),
        immutable_source_url,
        archive_member: request.archive_member.clone(),
        revision,
        file_path: request.file_path.clone(),
        file_sha256: sha256(&bytes),
        download_sha256: (request.dataset == DatasetId::TextDetox)
            .then(|| sha256(downloaded_bytes)),
        acquired_at_unix_seconds,
        license_id: request.license_id.clone(),
        license_url: request.license_url.clone(),
        license_year: request.license_year,
        citation: request.citation.clone(),
        upstream_lineage: request.upstream_lineage.clone(),
        lineage_status: request.lineage_status,
    })
}

pub fn frozen_source_from_record(record: &SourceRecord) -> FrozenSource {
    FrozenSource {
        dataset: record.dataset,
        detector_language: record.detector_language,
        source_role: record.source_role,
        source_file_id: record.source_file_id.clone(),
        immutable_source_url: record.immutable_source_url.clone(),
        archive_member: record.archive_member.clone(),
        revision: record.revision.clone(),
        file_path: record.file_path.clone(),
        file_sha256: record.file_sha256.clone(),
        download_sha256: record.download_sha256.clone(),
        license_id: record.license_id.clone(),
        license_url: record.license_url.clone(),
        license_year: record.license_year,
        citation: record.citation.clone(),
        upstream_lineage: record.upstream_lineage.clone(),
        lineage_status: record.lineage_status,
    }
}

pub fn freeze_observation(
    observation: SourceObservation,
) -> Result<FrozenSourceLock, SourceAcquisitionError> {
    validate_unique_records(&observation.sources)?;
    for source in &observation.sources {
        if source.dataset == DatasetId::TextDetox && source.download_sha256.is_none() {
            return Err(SourceAcquisitionError::MissingTextDetoxDownloadDigest(
                source.source_file_id.clone(),
            ));
        }
    }
    Ok(FrozenSourceLock {
        schema_version: SOURCE_LOCK_SCHEMA_VERSION.to_owned(),
        sources: observation
            .sources
            .iter()
            .map(frozen_source_from_record)
            .collect(),
    })
}

pub fn validate_source_lock_for_acquisition(
    source_lock: &FrozenSourceLock,
) -> Result<(), SourceAcquisitionError> {
    validate_unique_frozen(&source_lock.sources)?;
    for source in &source_lock.sources {
        if source.dataset != DatasetId::TextDetox {
            continue;
        }
        if source.download_sha256.is_none() {
            return Err(SourceAcquisitionError::MissingTextDetoxDownloadDigest(
                source.source_file_id.clone(),
            ));
        }
        validate_textdetox_download_identity(
            &source.source_file_id,
            source.revision.as_deref(),
            &source.immutable_source_url,
        )?;
    }
    Ok(())
}

pub fn validate_textdetox_download_identity<'a>(
    source_file_id: &str,
    revision: Option<&'a str>,
    url: &str,
) -> Result<&'a str, SourceAcquisitionError> {
    let Some(source_code) = source_file_id.strip_prefix("textdetox-") else {
        return Err(SourceAcquisitionError::InvalidTextDetoxParquetIdentity(
            source_file_id.to_owned(),
        ));
    };
    let Some(revision) = revision else {
        return Err(SourceAcquisitionError::InvalidTextDetoxParquetIdentity(
            source_file_id.to_owned(),
        ));
    };
    if revision != TEXTDETOX_REVISION || !TEXTDETOX_CODES.contains(&source_code) {
        return Err(SourceAcquisitionError::InvalidTextDetoxParquetIdentity(
            source_file_id.to_owned(),
        ));
    }
    let expected_url = format!(
        "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/{revision}/data/{source_code}-00000-of-00001.parquet"
    );
    if url != expected_url {
        return Err(SourceAcquisitionError::InvalidTextDetoxParquetIdentity(
            source_file_id.to_owned(),
        ));
    }
    Ok(revision)
}

pub fn validate_source_download(
    source: &FrozenSource,
    downloaded_bytes: &[u8],
) -> Result<(), SourceAcquisitionError> {
    if source.dataset != DatasetId::TextDetox {
        return Ok(());
    }
    let expected = source.download_sha256.as_ref().ok_or_else(|| {
        SourceAcquisitionError::MissingTextDetoxDownloadDigest(source.source_file_id.clone())
    })?;
    if &sha256(downloaded_bytes) != expected {
        return Err(SourceAcquisitionError::DownloadDigestMismatch {
            source_file_id: source.source_file_id.clone(),
        });
    }
    Ok(())
}

pub fn validate_observation_matches_catalog(
    observation: &SourceObservation,
    catalog: &SourceCatalog,
) -> Result<(), SourceAcquisitionError> {
    validate_catalog(catalog)?;
    validate_unique_records(&observation.sources)?;
    let catalog_ids = catalog
        .sources
        .iter()
        .map(|source| source.source_file_id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_ids = observation
        .sources
        .iter()
        .map(|source| source.source_file_id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(id) = catalog_ids.difference(&observed_ids).next() {
        return Err(SourceAcquisitionError::MissingSourceFileId(
            (*id).to_owned(),
        ));
    }
    if let Some(id) = observed_ids.difference(&catalog_ids).next() {
        return Err(SourceAcquisitionError::ExtraSourceFileId((*id).to_owned()));
    }
    for request in &catalog.sources {
        let record = observation
            .sources
            .iter()
            .find(|record| record.source_file_id == request.source_file_id)
            .expect("source identifiers were checked");
        if record.dataset != request.dataset
            || record.detector_language != request.detector_language
            || record.immutable_source_url != request.requested_url
            || record.archive_member != request.archive_member
            || record.revision != request.requested_revision
            || record.file_path != request.file_path
            || record.license_id != request.license_id
            || record.license_url != request.license_url
            || record.citation != request.citation
            || record.upstream_lineage != request.upstream_lineage
            || record.lineage_status != request.lineage_status
        {
            return Err(SourceAcquisitionError::IdentityMismatch {
                source_file_id: request.source_file_id.clone(),
            });
        }
    }
    Ok(())
}

pub fn validate_observation_matches_lock(
    observation: &SourceObservation,
    source_lock: &FrozenSourceLock,
) -> Result<(), SourceAcquisitionError> {
    validate_unique_records(&observation.sources)?;
    validate_unique_frozen(&source_lock.sources)?;
    let locked_ids = source_lock
        .sources
        .iter()
        .map(|source| source.source_file_id.as_str())
        .collect::<BTreeSet<_>>();
    let observed_ids = observation
        .sources
        .iter()
        .map(|source| source.source_file_id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(id) = locked_ids.difference(&observed_ids).next() {
        return Err(SourceAcquisitionError::MissingSourceFileId(
            (*id).to_owned(),
        ));
    }
    if let Some(id) = observed_ids.difference(&locked_ids).next() {
        return Err(SourceAcquisitionError::ExtraSourceFileId((*id).to_owned()));
    }
    for locked in &source_lock.sources {
        let observed = observation
            .sources
            .iter()
            .find(|source| source.source_file_id == locked.source_file_id)
            .expect("source identifiers were checked");
        if observed.dataset != locked.dataset
            || observed.detector_language != locked.detector_language
            || observed.immutable_source_url != locked.immutable_source_url
            || observed.archive_member != locked.archive_member
            || observed.revision != locked.revision
            || observed.file_path != locked.file_path
            || observed.license_id != locked.license_id
            || observed.license_url != locked.license_url
            || observed.citation != locked.citation
            || observed.upstream_lineage != locked.upstream_lineage
            || observed.lineage_status != locked.lineage_status
        {
            return Err(SourceAcquisitionError::IdentityMismatch {
                source_file_id: locked.source_file_id.clone(),
            });
        }
        if observed.file_sha256 != locked.file_sha256 {
            return Err(SourceAcquisitionError::DigestMismatch {
                source_file_id: locked.source_file_id.clone(),
            });
        }
        if observed.download_sha256 != locked.download_sha256 {
            return Err(SourceAcquisitionError::DownloadDigestMismatch {
                source_file_id: locked.source_file_id.clone(),
            });
        }
    }
    Ok(())
}

pub fn write_source_observation(
    output: &Path,
    observation: &SourceObservation,
) -> Result<(), SourceAcquisitionError> {
    if output.exists() {
        return Err(SourceAcquisitionError::ExistingOutput(output.to_owned()));
    }
    let staging = staging_path(output)?;
    if staging.exists() {
        return Err(SourceAcquisitionError::ExistingOutput(staging));
    }
    fs::create_dir(&staging)?;
    let result = (|| {
        let path = staging.join("source-observation-v1.json");
        let mut file = File::options().write(true).create_new(true).open(path)?;
        serde_json::to_writer_pretty(&mut file, observation)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        atomic_publish_noreplace(&staging, output).map_err(map_source_atomic_error)?;
        Ok(())
    })();
    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

pub fn write_frozen_source_lock(
    output: &Path,
    source_lock: &FrozenSourceLock,
) -> Result<(), SourceAcquisitionError> {
    if output.exists() {
        return Err(SourceAcquisitionError::ExistingOutput(output.to_owned()));
    }
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let staging = parent.join(format!(
        ".{}.staging-{}",
        file_name(output)?,
        std::process::id()
    ));
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(&staging)?;
    serde_json::to_writer_pretty(&mut file, source_lock)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    atomic_publish_noreplace(&staging, output).map_err(map_source_atomic_error)
}

pub fn write_acquired_sources(
    output: &Path,
    source_lock: &FrozenSourceLock,
    files: Vec<(String, Vec<u8>)>,
    acquired_at_unix_seconds: u64,
) -> Result<SourceObservation, SourceAcquisitionError> {
    if output.exists() {
        return Err(SourceAcquisitionError::ExistingOutput(output.to_owned()));
    }
    validate_unique_frozen(&source_lock.sources)?;
    let mut files_by_id = std::collections::BTreeMap::new();
    for (source_file_id, bytes) in files {
        if files_by_id.insert(source_file_id.clone(), bytes).is_some() {
            return Err(SourceAcquisitionError::DuplicateSourceFileId(
                source_file_id,
            ));
        }
    }
    let mut records = Vec::with_capacity(source_lock.sources.len());
    for source in &source_lock.sources {
        let bytes = files_by_id.get(&source.source_file_id).ok_or_else(|| {
            SourceAcquisitionError::MissingSourceFileId(source.source_file_id.clone())
        })?;
        let digest = sha256(bytes);
        if digest != source.file_sha256 {
            return Err(SourceAcquisitionError::DigestMismatch {
                source_file_id: source.source_file_id.clone(),
            });
        }
        records.push(SourceRecord {
            dataset: source.dataset,
            detector_language: source.detector_language,
            source_role: source.source_role,
            source_file_id: source.source_file_id.clone(),
            immutable_source_url: source.immutable_source_url.clone(),
            archive_member: source.archive_member.clone(),
            revision: source.revision.clone(),
            file_path: source.file_path.clone(),
            file_sha256: digest,
            download_sha256: source.download_sha256.clone(),
            acquired_at_unix_seconds,
            license_id: source.license_id.clone(),
            license_url: source.license_url.clone(),
            license_year: source.license_year,
            citation: source.citation.clone(),
            upstream_lineage: source.upstream_lineage.clone(),
            lineage_status: source.lineage_status,
        });
    }
    for source_file_id in files_by_id.keys() {
        if !source_lock
            .sources
            .iter()
            .any(|source| &source.source_file_id == source_file_id)
        {
            return Err(SourceAcquisitionError::ExtraSourceFileId(
                source_file_id.clone(),
            ));
        }
    }
    let observation = SourceObservation {
        schema_version: SOURCE_OBSERVATION_SCHEMA_VERSION.to_owned(),
        sources: records,
    };
    validate_observation_matches_lock(&observation, source_lock)?;
    let staging = staging_path(output)?;
    fs::create_dir(&staging)?;
    let result = (|| {
        let mut staged_paths = BTreeSet::new();
        let mut directories = BTreeSet::from([staging.clone()]);
        for source in &source_lock.sources {
            if !staged_paths.insert(source.file_path.clone()) {
                return Err(SourceAcquisitionError::IdentityMismatch {
                    source_file_id: source.source_file_id.clone(),
                });
            }
            let path = checked_source_path(&staging, &source.file_path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
                let mut directory = parent;
                while directory.starts_with(&staging) {
                    directories.insert(directory.to_owned());
                    if directory == staging {
                        break;
                    }
                    directory = directory
                        .parent()
                        .expect("nested staging directory has a parent");
                }
            }
            let mut file = File::options().write(true).create_new(true).open(path)?;
            file.write_all(
                files_by_id
                    .get(&source.source_file_id)
                    .expect("source bytes were checked"),
            )?;
            file.sync_all()?;
        }
        for directory in directories {
            File::open(directory)?.sync_all()?;
        }
        let path = staging.join("source-observation-v1.json");
        let mut file = File::options().write(true).create_new(true).open(path)?;
        serde_json::to_writer_pretty(&mut file, &observation)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        File::open(&staging)?.sync_all()?;
        atomic_publish_noreplace(&staging, output).map_err(map_source_atomic_error)?;
        Ok(())
    })();
    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result.map(|()| observation)
}

pub fn current_unix_seconds() -> Result<u64, SourceAcquisitionError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| SourceAcquisitionError::Io(std::io::Error::other(error)))?
        .as_secs())
}

pub fn validate_catalog(catalog: &SourceCatalog) -> Result<(), SourceAcquisitionError> {
    let records = catalog
        .sources
        .iter()
        .map(|request| {
            validate_lineage(
                request.dataset,
                request.detector_language,
                &request.source_file_id,
                request.lineage_status,
            )?;
            Ok(request.source_file_id.clone())
        })
        .collect::<Result<Vec<_>, SourceAcquisitionError>>()?;
    validate_unique_ids(records)
}

fn validate_unique_records(records: &[SourceRecord]) -> Result<(), SourceAcquisitionError> {
    for record in records {
        validate_lineage(
            record.dataset,
            record.detector_language,
            &record.source_file_id,
            record.lineage_status,
        )?;
    }
    validate_unique_ids(
        records
            .iter()
            .map(|record| record.source_file_id.clone())
            .collect(),
    )
}

fn validate_unique_frozen(records: &[FrozenSource]) -> Result<(), SourceAcquisitionError> {
    for record in records {
        validate_lineage(
            record.dataset,
            record.detector_language,
            &record.source_file_id,
            record.lineage_status,
        )?;
    }
    validate_unique_ids(
        records
            .iter()
            .map(|record| record.source_file_id.clone())
            .collect(),
    )
}

fn validate_unique_ids(ids: Vec<String>) -> Result<(), SourceAcquisitionError> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if !unique.insert(id.clone()) {
            return Err(SourceAcquisitionError::DuplicateSourceFileId(id));
        }
    }
    Ok(())
}

fn validate_lineage(
    dataset: DatasetId,
    language: blasphem::Language,
    source_file_id: &str,
    status: LineageStatus,
) -> Result<(), SourceAcquisitionError> {
    if status == LineageStatus::Unresolved
        && !(dataset == DatasetId::TextDetox
            && matches!(language, blasphem::Language::Zh | blasphem::Language::Fr))
    {
        return Err(SourceAcquisitionError::InvalidUnresolvedLineage(
            source_file_id.to_owned(),
        ));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    let digest = Sha256::digest(bytes);
    Sha256Digest::try_from(format!("{digest:x}")).expect("SHA-256 has a valid shape")
}

#[must_use]
pub fn sha256_digest(bytes: &[u8]) -> Sha256Digest {
    sha256(bytes)
}

fn staging_path(output: &Path) -> Result<PathBuf, SourceAcquisitionError> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!(
        ".{}.staging-{}",
        file_name(output)?,
        std::process::id()
    )))
}

fn checked_source_path(root: &Path, relative: &str) -> Result<PathBuf, SourceAcquisitionError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(SourceAcquisitionError::Io(std::io::Error::other(
            "source path escapes the raw output directory",
        )));
    }
    Ok(root.join(path))
}

fn file_name(output: &Path) -> Result<&str, SourceAcquisitionError> {
    output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SourceAcquisitionError::Io(std::io::Error::other("invalid output path")))
}

fn map_source_atomic_error(error: AtomicPublishError) -> SourceAcquisitionError {
    match error {
        AtomicPublishError::DestinationExists => {
            SourceAcquisitionError::ExistingOutput(PathBuf::from("destination"))
        }
        AtomicPublishError::Unsupported => SourceAcquisitionError::Io(std::io::Error::other(
            "atomic no-replace publication is unsupported",
        )),
        AtomicPublishError::Rename(error)
        | AtomicPublishError::StagingSync(error)
        | AtomicPublishError::ParentSync(error)
        | AtomicPublishError::Cleanup(error) => SourceAcquisitionError::Io(error),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDetoxHttpResponse {
    pub revision: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDetoxTransportError {
    message: String,
    transient: bool,
}

impl TextDetoxTransportError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            transient: false,
        }
    }

    #[must_use]
    pub fn transient(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            transient: true,
        }
    }

    #[must_use]
    pub const fn is_transient(&self) -> bool {
        self.transient
    }
}

impl fmt::Display for TextDetoxTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TextDetoxTransportError {}

pub trait TextDetoxHttpClient {
    fn get(&mut self, url: &str) -> Result<TextDetoxHttpResponse, TextDetoxTransportError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredTextDetox {
    pub revision: String,
    pub rows: Vec<TextDetoxSourceRow>,
}

#[derive(Debug, Error)]
pub enum TextDetoxAcquisitionError {
    #[error("TextDetox max rows must be greater than zero")]
    ZeroMaxRows,
    #[error("cannot fetch TextDetox data: {0}")]
    Transport(#[from] TextDetoxTransportError),
    #[error("cannot parse the TextDetox revision: {0}")]
    RevisionJson(#[source] serde_json::Error),
    #[error("the TextDetox revision is blank")]
    BlankRevision,
    #[error("the TextDetox page has no x-revision header for {language} at offset {offset}")]
    MissingPageRevision { language: String, offset: usize },
    #[error(
        "TextDetox page revision mismatch for {language} at offset {offset}: expected {expected}, got {actual}"
    )]
    PageRevisionMismatch {
        expected: String,
        actual: String,
        language: String,
        offset: usize,
    },
    #[error("TextDetox revision changed during acquisition: expected {expected}, got {actual}")]
    FinalRevisionMismatch { expected: String, actual: String },
    #[error("TextDetox returned an empty page for {language} at offset {offset}")]
    EmptyPage { language: String, offset: usize },
    #[error(
        "TextDetox returned a short page for {language} at offset {offset}: expected {expected}, got {actual}"
    )]
    ShortPage {
        language: String,
        offset: usize,
        expected: usize,
        actual: usize,
    },
    #[error(
        "TextDetox returned a noncontiguous row for {language}: expected {expected}, got {actual}"
    )]
    NonContiguousRow {
        language: String,
        expected: usize,
        actual: usize,
    },
    #[error(
        "TextDetox total changed for {language}: expected {expected}, got {actual} at offset {offset}"
    )]
    TotalChanged {
        language: String,
        offset: usize,
        expected: usize,
        actual: usize,
    },
    #[error(transparent)]
    Data(#[from] TextDetoxError),
}

#[derive(Debug, Error)]
pub enum TextDetoxFetchError {
    #[error(transparent)]
    Acquisition(#[from] TextDetoxAcquisitionError),
    #[error("TextDetox output already exists: {0}")]
    ExistingOutput(PathBuf),
    #[error("TextDetox output must have a UTF-8 file name: {0}")]
    InvalidOutput(PathBuf),
    #[error("TextDetox staging output already exists: {0}")]
    ExistingStaging(PathBuf),
    #[error("cannot create TextDetox staging output {path}: {source}")]
    CreateStaging {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write TextDetox staging output: {0}")]
    Write(#[source] TextDetoxError),
    #[error("cannot publish TextDetox output {path}: {source}")]
    Publish {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("atomic no-replace publication is unsupported on this target")]
    UnsupportedAtomicPublish,
    #[cfg(test)]
    #[error("injected writer failure")]
    InjectedWriterFailure,
}

#[derive(Deserialize)]
struct RevisionDocument {
    sha: String,
}

pub fn acquire_textdetox(
    client: &mut impl TextDetoxHttpClient,
    languages: &[String],
    max_rows: Option<usize>,
) -> Result<AcquiredTextDetox, TextDetoxAcquisitionError> {
    if max_rows == Some(0) {
        return Err(TextDetoxAcquisitionError::ZeroMaxRows);
    }
    for language in languages {
        textdetox_rows_url(language, 0, 1)?;
    }

    let revision = read_revision(client)?;

    let mut rows = Vec::new();
    for language in languages {
        let mut offset = 0;
        let mut stable_total = None;
        loop {
            let remaining_limit = max_rows.map_or(100, |limit| limit.saturating_sub(offset));
            let length = remaining_limit.min(100);
            if length == 0 {
                break;
            }
            let url = textdetox_rows_url(language, offset, length)?;
            let page_response = client.get(&url)?;
            let page_revision = page_response.revision.ok_or_else(|| {
                TextDetoxAcquisitionError::MissingPageRevision {
                    language: language.clone(),
                    offset,
                }
            })?;
            if page_revision != revision {
                return Err(TextDetoxAcquisitionError::PageRevisionMismatch {
                    expected: revision.clone(),
                    actual: page_revision,
                    language: language.clone(),
                    offset,
                });
            }
            let page = parse_textdetox_page(page_response.body.as_slice(), language, &revision)?;
            if let Some(expected) = stable_total {
                if page.total_rows != expected {
                    return Err(TextDetoxAcquisitionError::TotalChanged {
                        language: language.clone(),
                        offset,
                        expected,
                        actual: page.total_rows,
                    });
                }
            } else {
                stable_total = Some(page.total_rows);
            }
            let target = max_rows.map_or(page.total_rows, |limit| limit.min(page.total_rows));
            let expected_length = length.min(target.saturating_sub(offset));
            if expected_length == 0 {
                break;
            }
            if page.rows.is_empty() {
                return Err(TextDetoxAcquisitionError::EmptyPage {
                    language: language.clone(),
                    offset,
                });
            }
            if page.rows.len() != expected_length {
                return Err(TextDetoxAcquisitionError::ShortPage {
                    language: language.clone(),
                    offset,
                    expected: expected_length,
                    actual: page.rows.len(),
                });
            }
            for (position, row) in page.rows.iter().enumerate() {
                let expected = offset + position;
                let actual = source_row_index(&row.source_id).unwrap_or(usize::MAX);
                if actual != expected {
                    return Err(TextDetoxAcquisitionError::NonContiguousRow {
                        language: language.clone(),
                        expected,
                        actual,
                    });
                }
            }
            offset += page.rows.len();
            rows.extend(page.rows);
            if offset >= target {
                break;
            }
        }
    }

    let final_revision = read_revision(client)?;
    if final_revision != revision {
        return Err(TextDetoxAcquisitionError::FinalRevisionMismatch {
            expected: revision,
            actual: final_revision,
        });
    }

    Ok(AcquiredTextDetox { revision, rows })
}

pub fn fetch_textdetox(
    client: &mut impl TextDetoxHttpClient,
    languages: &[String],
    max_rows: Option<usize>,
    output: &Path,
) -> Result<AcquiredTextDetox, TextDetoxFetchError> {
    if output.exists() {
        return Err(TextDetoxFetchError::ExistingOutput(output.to_owned()));
    }
    let acquired = acquire_textdetox(client, languages, max_rows)?;
    publish_acquired_with(output, &acquired, |staging, acquired| {
        let file = File::options()
            .write(true)
            .create_new(true)
            .open(staging)
            .map_err(|source| TextDetoxFetchError::CreateStaging {
                path: staging.to_owned(),
                source,
            })?;
        write_textdetox_source_tsv(file, &acquired.rows).map_err(TextDetoxFetchError::Write)
    })?;
    Ok(acquired)
}

fn publish_acquired_with(
    output: &Path,
    acquired: &AcquiredTextDetox,
    writer: impl FnOnce(&Path, &AcquiredTextDetox) -> Result<(), TextDetoxFetchError>,
) -> Result<(), TextDetoxFetchError> {
    if output.exists() {
        return Err(TextDetoxFetchError::ExistingOutput(output.to_owned()));
    }
    let staging = acquisition_staging_path(output)?;
    if staging.exists() {
        return Err(TextDetoxFetchError::ExistingStaging(staging));
    }
    let result: Result<(), TextDetoxFetchError> = (|| {
        writer(&staging, acquired)?;
        atomic_publish_noreplace(&staging, output)
            .map_err(|error| map_atomic_fetch_error(error, output))?;
        Ok(())
    })();
    if result.is_err() && staging.exists() {
        let _ = fs::remove_file(&staging);
    }
    result?;
    Ok(())
}

fn map_atomic_fetch_error(error: AtomicPublishError, output: &Path) -> TextDetoxFetchError {
    match error {
        AtomicPublishError::DestinationExists => {
            TextDetoxFetchError::ExistingOutput(output.to_owned())
        }
        AtomicPublishError::Unsupported => TextDetoxFetchError::UnsupportedAtomicPublish,
        AtomicPublishError::Rename(source)
        | AtomicPublishError::StagingSync(source)
        | AtomicPublishError::ParentSync(source)
        | AtomicPublishError::Cleanup(source) => TextDetoxFetchError::Publish {
            path: output.to_owned(),
            source,
        },
    }
}

fn acquisition_staging_path(output: &Path) -> Result<PathBuf, TextDetoxFetchError> {
    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| TextDetoxFetchError::InvalidOutput(output.to_owned()))?;
    Ok(parent.join(format!(".{name}.staging-{}", std::process::id())))
}

fn source_row_index(source_id: &str) -> Option<usize> {
    let suffix = source_id.rsplit('/').next()?;
    (suffix.len() == 6 && suffix.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| suffix.parse().ok())
        .flatten()
}

fn read_revision(
    client: &mut impl TextDetoxHttpClient,
) -> Result<String, TextDetoxAcquisitionError> {
    let response = client.get(TEXTDETOX_REVISION_URL)?;
    let document: RevisionDocument =
        serde_json::from_slice(&response.body).map_err(TextDetoxAcquisitionError::RevisionJson)?;
    let revision = document.sha.trim().to_owned();
    if revision.is_empty() {
        return Err(TextDetoxAcquisitionError::BlankRevision);
    }
    Ok(revision)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        AcquiredTextDetox, SourceAcquisitionError, TextDetoxFetchError, publish_acquired_with,
        select_exact_member_index,
    };

    #[test]
    fn archive_extraction_rejects_duplicate_matching_members() {
        let names = vec![(0, "data.tsv".to_owned()), (1, "data.tsv".to_owned())];

        let error = select_exact_member_index("data.tsv", &names).expect_err("duplicate member");

        assert!(matches!(
            error,
            SourceAcquisitionError::DuplicateArchiveMember(member) if member == "data.tsv"
        ));
    }

    #[test]
    fn removes_the_staging_file_after_a_writer_failure() {
        let directory = tempdir().expect("temporary directory");
        let output = directory.path().join("source.tsv");
        let acquired = AcquiredTextDetox {
            revision: "rev-a".to_owned(),
            rows: Vec::new(),
        };

        let error = publish_acquired_with(&output, &acquired, |staging, _| {
            std::fs::write(staging, "partial").expect("write partial staging file");
            Err(TextDetoxFetchError::InjectedWriterFailure)
        })
        .expect_err("writer failure");

        assert!(matches!(error, TextDetoxFetchError::InjectedWriterFailure));
        assert!(!output.exists());
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("read directory")
                .count(),
            0
        );
    }

    #[test]
    fn maps_a_concurrent_file_to_existing_output_without_overwrite() {
        let directory = tempdir().expect("temporary directory");
        let output = directory.path().join("source.tsv");
        let acquired = AcquiredTextDetox {
            revision: "rev-a".to_owned(),
            rows: Vec::new(),
        };

        let error = publish_acquired_with(&output, &acquired, |staging, _| {
            std::fs::write(staging, "staged").expect("write staged file");
            std::fs::write(&output, "concurrent").expect("write concurrent file");
            Ok(())
        })
        .expect_err("existing output");

        assert!(matches!(
            error,
            TextDetoxFetchError::ExistingOutput(path) if path == output
        ));
        assert_eq!(
            std::fs::read_to_string(&output).expect("concurrent output"),
            "concurrent"
        );
        assert_eq!(
            std::fs::read_dir(directory.path())
                .expect("read directory")
                .count(),
            1
        );
    }
}
