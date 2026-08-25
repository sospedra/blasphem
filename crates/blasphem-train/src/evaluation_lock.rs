use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::evidence::{Sha256Digest, sha256_digest};

pub const EVALUATION_LOCK_SCHEMA_VERSION: &str = "evaluation-lock-v1";

/// The sealed validation and test digests for one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedLanguage {
    pub validation_sha256: Sha256Digest,
    pub test_sha256: Sha256Digest,
}

/// The sealed evaluation partitions, keyed by storage code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationLock {
    pub schema_version: String,
    pub languages: BTreeMap<String, SealedLanguage>,
}

#[derive(Debug, Error)]
pub enum EvaluationLockError {
    #[error("cannot parse the evaluation lock: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid evaluation lock schema version: expected {expected}, got {actual}")]
    InvalidSchemaVersion {
        expected: &'static str,
        actual: String,
    },
    #[error("cannot read the sealed partition {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the sealed partition {relative_path} changed: expected {expected}, got {actual}")]
    SealedHashChanged {
        relative_path: String,
        expected: String,
        actual: String,
    },
}

/// Parses one evaluation lock document.
///
/// # Errors
///
/// Returns an error when the JSON or the schema version is invalid.
pub fn parse_evaluation_lock(reader: impl Read) -> Result<EvaluationLock, EvaluationLockError> {
    let lock: EvaluationLock = serde_json::from_reader(reader)?;
    if lock.schema_version != EVALUATION_LOCK_SCHEMA_VERSION {
        return Err(EvaluationLockError::InvalidSchemaVersion {
            expected: EVALUATION_LOCK_SCHEMA_VERSION,
            actual: lock.schema_version,
        });
    }
    Ok(lock)
}

/// Returns the SHA-256 digest of one prepared split file.
///
/// # Errors
///
/// Returns an error when the file is unreadable.
pub fn sealed_partition_digest(path: &Path) -> Result<Sha256Digest, EvaluationLockError> {
    let bytes = fs::read(path).map_err(|source| EvaluationLockError::Io {
        path: path.to_owned(),
        source,
    })?;
    Ok(sha256_digest(&bytes))
}

/// Computes the sealed digests for every language directory under one prepared root.
///
/// # Errors
///
/// Returns an error when a sealed file is unreadable.
pub fn compute_sealed_partitions(
    prepared_root: &Path,
    storage_codes: &[&str],
) -> Result<BTreeMap<String, SealedLanguage>, EvaluationLockError> {
    let mut languages = BTreeMap::new();
    for code in storage_codes {
        languages.insert(
            (*code).to_owned(),
            SealedLanguage {
                validation_sha256: sealed_partition_digest(
                    &prepared_root.join(code).join("validation.tsv"),
                )?,
                test_sha256: sealed_partition_digest(&prepared_root.join(code).join("test.tsv"))?,
            },
        );
    }
    Ok(languages)
}

/// Rejects any change to a sealed validation or test partition.
///
/// # Errors
///
/// Returns an error on the first missing or changed sealed file.
pub fn verify_sealed_partitions(
    prepared_root: &Path,
    lock: &EvaluationLock,
) -> Result<(), EvaluationLockError> {
    for (code, sealed) in &lock.languages {
        for (name, expected) in [
            ("validation.tsv", &sealed.validation_sha256),
            ("test.tsv", &sealed.test_sha256),
        ] {
            let path = prepared_root.join(code).join(name);
            let actual = sealed_partition_digest(&path)?;
            if &actual != expected {
                return Err(EvaluationLockError::SealedHashChanged {
                    relative_path: format!("{code}/{name}"),
                    expected: expected.to_string(),
                    actual: actual.to_string(),
                });
            }
        }
    }
    Ok(())
}
