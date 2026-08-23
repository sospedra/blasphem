use std::{fmt, fs, path::Path};

use serde::{Deserialize, Serialize as SerializeDerive};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, SerializeDerive, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Digest(String);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("SHA-256 digest must contain exactly 64 lowercase hexadecimal characters: {0}")]
pub struct InvalidSha256Digest(String);

impl Sha256Digest {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sha256Digest {
    type Error = InvalidSha256Digest;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            Ok(Self(value))
        } else {
            Err(InvalidSha256Digest(value))
        }
    }
}

impl From<Sha256Digest> for String {
    fn from(value: Sha256Digest) -> Self {
        value.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(&self.0)
    }
}

#[derive(Debug, Error)]
pub enum CanonicalEvidenceError {
    #[error("cannot parse or serialize canonical JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the JSON input is not canonical")]
    NonCanonical,
    #[error("cannot write canonical evidence: {0}")]
    Io(#[from] std::io::Error),
}

/// Serializes one evidence value with RFC 8785 JSON canonicalization.
///
/// # Errors
///
/// Returns an error when the value cannot be represented as JSON.
pub fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CanonicalEvidenceError> {
    serde_jcs::to_vec(value).map_err(CanonicalEvidenceError::Json)
}

/// Parses one typed canonical evidence value.
///
/// # Errors
///
/// Returns an error for invalid JSON or any noncanonical byte sequence.
pub fn parse_canonical_json<T>(bytes: &[u8]) -> Result<T, CanonicalEvidenceError>
where
    T: DeserializeOwned + Serialize,
{
    let value = serde_json::from_slice(bytes)?;
    if canonical_json_bytes(&value)? != bytes {
        return Err(CanonicalEvidenceError::NonCanonical);
    }
    Ok(value)
}

#[must_use]
pub fn sha256_digest(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("SHA-256 output is a valid digest")
}

/// Writes one canonical evidence file without a trailing newline.
///
/// # Errors
///
/// Returns an error when serialization or a file operation fails.
pub fn write_canonical_json<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), CanonicalEvidenceError> {
    let bytes = canonical_json_bytes(value)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}
