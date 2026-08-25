use std::io::Read;

use blasphem::Language;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    datasets::{DatasetId, LineageStatus},
    evidence::Sha256Digest,
    source_role::SourceRole,
};

pub const SOURCE_CATALOG_SCHEMA_VERSION: &str = "source-catalog-v1";
pub const SOURCE_OBSERVATION_SCHEMA_VERSION: &str = "source-observation-v1";
pub const SOURCE_LOCK_SCHEMA_VERSION: &str = "source-lock-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRequest {
    pub dataset: DatasetId,
    pub detector_language: Language,
    pub source_role: SourceRole,
    pub source_file_id: String,
    pub requested_url: String,
    pub revision_url: Option<String>,
    pub requested_revision: Option<String>,
    pub archive_member: Option<String>,
    pub file_path: String,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenSource {
    pub dataset: DatasetId,
    pub detector_language: Language,
    pub source_role: SourceRole,
    pub source_file_id: String,
    pub immutable_source_url: String,
    pub archive_member: Option<String>,
    pub revision: Option<String>,
    pub file_path: String,
    pub file_sha256: Sha256Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_sha256: Option<Sha256Digest>,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub dataset: DatasetId,
    #[serde(serialize_with = "serialize_storage_code")]
    pub detector_language: Language,
    pub source_role: SourceRole,
    pub source_file_id: String,
    pub immutable_source_url: String,
    pub archive_member: Option<String>,
    pub revision: Option<String>,
    pub file_path: String,
    pub file_sha256: Sha256Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub download_sha256: Option<Sha256Digest>,
    pub acquired_at_unix_seconds: u64,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
}

fn serialize_storage_code<S>(language: &Language, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(language.storage_code())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCatalog {
    pub schema_version: String,
    pub sources: Vec<SourceRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObservation {
    pub schema_version: String,
    pub sources: Vec<SourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenSourceLock {
    pub schema_version: String,
    pub sources: Vec<FrozenSource>,
}

#[derive(Debug, Error)]
pub enum SourceManifestError {
    #[error("cannot parse source manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid source manifest schema version: expected {expected}, got {actual}")]
    InvalidSchemaVersion {
        expected: &'static str,
        actual: String,
    },
}

pub fn parse_source_catalog(reader: impl Read) -> Result<SourceCatalog, SourceManifestError> {
    let catalog: SourceCatalog = serde_json::from_reader(reader)?;
    validate_schema_version(&catalog.schema_version, SOURCE_CATALOG_SCHEMA_VERSION)?;
    Ok(catalog)
}

pub fn parse_source_observation(
    reader: impl Read,
) -> Result<SourceObservation, SourceManifestError> {
    let observation: SourceObservation = serde_json::from_reader(reader)?;
    validate_schema_version(
        &observation.schema_version,
        SOURCE_OBSERVATION_SCHEMA_VERSION,
    )?;
    Ok(observation)
}

pub fn parse_frozen_source_lock(
    reader: impl Read,
) -> Result<FrozenSourceLock, SourceManifestError> {
    let source_lock: FrozenSourceLock = serde_json::from_reader(reader)?;
    validate_schema_version(&source_lock.schema_version, SOURCE_LOCK_SCHEMA_VERSION)?;
    Ok(source_lock)
}

fn validate_schema_version(
    actual: &str,
    expected: &'static str,
) -> Result<(), SourceManifestError> {
    if actual == expected {
        Ok(())
    } else {
        Err(SourceManifestError::InvalidSchemaVersion {
            expected,
            actual: actual.to_owned(),
        })
    }
}
