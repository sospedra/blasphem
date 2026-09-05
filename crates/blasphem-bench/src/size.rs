use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use blasphem::Language;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{check_artifact_size, check_binary_size, sha256_hex};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSizeRecord {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeEvidence {
    pub schema_version: u16,
    pub evidence_status: String,
    pub target_triple: String,
    pub binary: FileSizeRecord,
    pub artifacts: BTreeMap<String, FileSizeRecord>,
    pub lexicon: BTreeMap<String, FileSizeRecord>,
    pub all_gates_passed: bool,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: u16,
    entries: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    language: Language,
    artifact_relative_path: String,
    artifact_bytes: u64,
    artifact_sha256: String,
    lexicon_sha256: Option<String>,
}

#[derive(Debug, Error)]
pub enum SizeError {
    #[error("cannot read size input at {path}: {source}")]
    FileIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse model manifest: {0}")]
    ManifestJson(#[from] serde_json::Error),
    #[error("model manifest schema must be 2, got {0}")]
    ManifestSchema(u16),
    #[error("model manifest repeats language {}", .0.code())]
    DuplicateLanguage(Language),
    #[error("model manifest misses language {}", .0.code())]
    MissingLanguage(Language),
    #[error("unsafe artifact path for {}", .0.code())]
    UnsafeArtifactPath(Language),
    #[error("file size mismatch for {path}: expected {expected}, got {actual}")]
    SizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },
    #[error("file digest mismatch for {0}")]
    DigestMismatch(String),
    #[error("model manifest misses the Lexicon digest for {}", .0.code())]
    MissingLexiconDigest(Language),
    #[error(transparent)]
    Gate(#[from] crate::SizeGateError),
}

/// Reads, hashes, and verifies one file record.
///
/// # Errors
///
/// Returns an error for a missing file or a declared size or digest mismatch.
pub fn record_file(
    path: &Path,
    relative_path: &str,
    expected_sha256: Option<&str>,
    expected_bytes: Option<u64>,
) -> Result<FileSizeRecord, SizeError> {
    let bytes = fs::read(path).map_err(|source| SizeError::FileIo {
        path: path.to_owned(),
        source,
    })?;
    let byte_count = u64::try_from(bytes.len()).map_err(|_| SizeError::SizeMismatch {
        path: relative_path.to_owned(),
        expected: expected_bytes.unwrap_or(u64::MAX),
        actual: u64::MAX,
    })?;
    if let Some(expected) = expected_bytes
        && byte_count != expected
    {
        return Err(SizeError::SizeMismatch {
            path: relative_path.to_owned(),
            expected,
            actual: byte_count,
        });
    }
    let digest = sha256_hex(&bytes);
    if let Some(expected) = expected_sha256
        && digest != expected
    {
        return Err(SizeError::DigestMismatch(relative_path.to_owned()));
    }
    Ok(FileSizeRecord {
        relative_path: relative_path.to_owned(),
        bytes: byte_count,
        sha256: digest,
    })
}

/// Collects size evidence for the binary, 15 artifacts, and 15 Lexicon files.
///
/// # Errors
///
/// Returns an error when a file is missing, changed, duplicated, unsafe, or too large.
pub fn collect_size_evidence(
    binary_path: &Path,
    model_manifest_path: &Path,
    lexicon_root: &Path,
    target_triple: &str,
) -> Result<SizeEvidence, SizeError> {
    let manifest_bytes = fs::read(model_manifest_path).map_err(|source| SizeError::FileIo {
        path: model_manifest_path.to_owned(),
        source,
    })?;
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_version != 2 {
        return Err(SizeError::ManifestSchema(manifest.schema_version));
    }
    let model_root = model_manifest_path
        .parent()
        .and_then(Path::parent)
        .map_or_else(
            || PathBuf::from("resources/models"),
            |resources| resources.join("models"),
        );
    let binary = record_file(binary_path, &binary_path.to_string_lossy(), None, None)?;
    check_binary_size(binary.bytes)?;

    let mut artifacts = BTreeMap::new();
    let mut lexicon = BTreeMap::new();
    for entry in manifest.entries {
        let code = entry.language.code().to_owned();
        if artifacts.contains_key(&code) {
            return Err(SizeError::DuplicateLanguage(entry.language));
        }
        if !safe_relative_file(&entry.artifact_relative_path) {
            return Err(SizeError::UnsafeArtifactPath(entry.language));
        }
        let artifact_path = model_root.join(&entry.artifact_relative_path);
        let artifact_label = artifact_path.to_string_lossy().into_owned();
        let artifact = record_file(
            &artifact_path,
            &artifact_label,
            Some(&entry.artifact_sha256),
            Some(entry.artifact_bytes),
        )?;
        check_artifact_size(artifact.bytes)?;
        artifacts.insert(code.clone(), artifact);

        let expected_lexicon = entry
            .lexicon_sha256
            .ok_or(SizeError::MissingLexiconDigest(entry.language))?;
        let storage_code = entry.language.storage_code();
        let lexicon_path = lexicon_root.join(format!("{storage_code}.tsv"));
        let lexicon_label = lexicon_path.to_string_lossy().into_owned();
        let lexicon_record =
            record_file(&lexicon_path, &lexicon_label, Some(&expected_lexicon), None)?;
        lexicon.insert(code, lexicon_record);
    }
    for language in Language::ALL {
        if !artifacts.contains_key(language.code()) {
            return Err(SizeError::MissingLanguage(language));
        }
    }

    Ok(SizeEvidence {
        schema_version: 1,
        evidence_status: "experimental".to_owned(),
        target_triple: target_triple.to_owned(),
        binary,
        artifacts,
        lexicon,
        all_gates_passed: true,
    })
}

fn safe_relative_file(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}
