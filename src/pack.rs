//! The `.pack` container: one language's sparse artifact and lexicon.
//!
//! Packs travel outside the binary. A pack names its language and the
//! rule-pack version it was built for, so the loader can refuse a pack that
//! does not match the compiled rules. Digests come from the manifest that
//! ships beside the packs and are verified here, never pinned in the core.

use std::str::FromStr;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::Language;

pub const PACK_MAGIC: &[u8; 8] = b"BLSPHPCK";
pub const PACK_FORMAT_VERSION: u32 = 1;
pub const PACK_HEADER_LEN: usize = 24;

/// Everything a pack stores.
#[derive(Debug, Clone, Copy)]
pub struct PackInput<'a> {
    pub language: Language,
    pub rule_pack_version: u16,
    pub artifact: &'a [u8],
    pub lexicon: &'a [u8],
}

/// A pack's fields, borrowed from its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedPack<'a> {
    pub language: Language,
    pub rule_pack_version: u16,
    pub artifact: &'a [u8],
    pub lexicon: &'a [u8],
}

/// One language's files as the loader hands them to the judge.
#[derive(Debug, Clone, Copy)]
pub struct PackSource<'a> {
    pub language: Language,
    pub pack: &'a [u8],
    pub pack_sha256: Option<[u8; 32]>,
    pub detect: Option<&'a [u8]>,
    pub detect_sha256: Option<[u8; 32]>,
}

/// Anything that stops a pack or a detect slice from loading. Every message
/// starts with the error code the JavaScript contract exposes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PackError {
    #[error("BLASPHEM_DIGEST_MISMATCH: {file} expected sha256 {expected}, actual {actual}")]
    DigestMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error(
        "BLASPHEM_FORMAT_VERSION: {file} has format version {found}, this build accepts {accepted}"
    )]
    FormatVersion {
        file: String,
        found: u32,
        accepted: u32,
    },
    #[error("BLASPHEM_PACK_INVALID: {file} {reason}")]
    Invalid { file: String, reason: String },
}

/// The file name a language's pack carries in every runtime.
#[must_use]
pub fn pack_file_name(language: Language) -> String {
    format!("{}.pack", language.code().to_ascii_lowercase())
}

/// The file name a language's detect slice carries in every runtime.
#[must_use]
pub fn detect_file_name(language: Language) -> String {
    format!("{}.detect", language.code().to_ascii_lowercase())
}

/// Serializes one pack.
#[must_use]
pub fn encode_pack(input: &PackInput<'_>) -> Vec<u8> {
    let mut bytes =
        Vec::with_capacity(PACK_HEADER_LEN + input.artifact.len() + input.lexicon.len());
    bytes.extend_from_slice(PACK_MAGIC);
    bytes.extend_from_slice(&PACK_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(input.language.code().to_ascii_lowercase().as_bytes());
    bytes.extend_from_slice(&input.rule_pack_version.to_le_bytes());
    bytes.extend_from_slice(&(input.artifact.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(input.lexicon.len() as u32).to_le_bytes());
    bytes.extend_from_slice(input.artifact);
    bytes.extend_from_slice(input.lexicon);
    bytes
}

/// Parses a pack and checks that it declares the requested language.
///
/// # Errors
///
/// Returns an error for a wrong magic, an unknown version, a truncated or
/// padded file, or a pack for another language.
pub fn decode_pack(language: Language, bytes: &[u8]) -> Result<DecodedPack<'_>, PackError> {
    let file = pack_file_name(language);
    let invalid = |reason: &str| PackError::Invalid {
        file: file.clone(),
        reason: reason.to_owned(),
    };
    if bytes.len() < PACK_HEADER_LEN {
        return Err(invalid("is shorter than its header"));
    }
    if &bytes[..8] != PACK_MAGIC {
        return Err(invalid("has an invalid magic"));
    }
    let found = read_u32(bytes, 8);
    if found != PACK_FORMAT_VERSION {
        return Err(PackError::FormatVersion {
            file,
            found,
            accepted: PACK_FORMAT_VERSION,
        });
    }
    let declared = std::str::from_utf8(&bytes[12..14])
        .ok()
        .and_then(|code| Language::from_str(code).ok())
        .ok_or_else(|| invalid("declares an unknown language"))?;
    if declared != language {
        return Err(invalid(&format!(
            "declares {}",
            declared.code().to_ascii_lowercase()
        )));
    }
    let rule_pack_version = u16::from_le_bytes([bytes[14], bytes[15]]);
    let artifact_len = read_u32(bytes, 16) as usize;
    let lexicon_len = read_u32(bytes, 20) as usize;
    let expected_len = PACK_HEADER_LEN
        .checked_add(artifact_len)
        .and_then(|length| length.checked_add(lexicon_len))
        .ok_or_else(|| invalid("has overflowing section lengths"))?;
    if bytes.len() != expected_len {
        return Err(invalid(&format!(
            "has {} bytes, its header promises {expected_len}",
            bytes.len()
        )));
    }
    let artifact = &bytes[PACK_HEADER_LEN..PACK_HEADER_LEN + artifact_len];
    let lexicon = &bytes[PACK_HEADER_LEN + artifact_len..];
    Ok(DecodedPack {
        language,
        rule_pack_version,
        artifact,
        lexicon,
    })
}

/// Checks bytes against the digest the manifest promised, when one was given.
///
/// # Errors
///
/// Returns [`PackError::DigestMismatch`] naming the file and both digests.
pub fn verify_digest(
    file: &str,
    bytes: &[u8],
    expected: Option<[u8; 32]>,
) -> Result<(), PackError> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual: [u8; 32] = Sha256::digest(bytes).into();
    if actual == expected {
        return Ok(());
    }
    Err(PackError::DigestMismatch {
        file: file.to_owned(),
        expected: hex(&expected),
        actual: hex(&actual),
    })
}

/// Parses 64 hexadecimal characters into a digest.
#[must_use]
pub fn parse_sha256(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut digest = [0_u8; 32];
    for (index, chunk) in text.as_bytes().chunks_exact(2).enumerate() {
        let pair = std::str::from_utf8(chunk).ok()?;
        digest[index] = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(digest)
}

fn hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"))
}

#[cfg(test)]
mod tests {
    use super::{
        PACK_FORMAT_VERSION, PackError, PackInput, decode_pack, encode_pack, parse_sha256,
        verify_digest,
    };
    use crate::Language;

    fn sample() -> Vec<u8> {
        encode_pack(&PackInput {
            language: Language::Ms,
            rule_pack_version: 1,
            artifact: b"artifact",
            lexicon: b"lexicon",
        })
    }

    #[test]
    fn a_pack_round_trips_its_fields() {
        let bytes = sample();
        let decoded = decode_pack(Language::Ms, &bytes).expect("valid pack");

        assert_eq!(&bytes[12..14], b"ms");
        assert_eq!(decoded.language, Language::Ms);
        assert_eq!(decoded.rule_pack_version, 1);
        assert_eq!(decoded.artifact, b"artifact");
        assert_eq!(decoded.lexicon, b"lexicon");
    }

    #[test]
    fn a_pack_for_another_language_is_rejected_by_name() {
        let error = decode_pack(Language::En, &sample()).expect_err("wrong language");

        assert_eq!(
            error.to_string(),
            "BLASPHEM_PACK_INVALID: en.pack declares ms"
        );
    }

    #[test]
    fn a_foreign_format_version_names_the_accepted_one() {
        let mut bytes = sample();
        bytes[8..12].copy_from_slice(&7_u32.to_le_bytes());

        assert_eq!(
            decode_pack(Language::Ms, &bytes).expect_err("wrong version"),
            PackError::FormatVersion {
                file: "ms.pack".to_owned(),
                found: 7,
                accepted: PACK_FORMAT_VERSION,
            }
        );
    }

    #[test]
    fn truncated_and_padded_packs_are_rejected() {
        let bytes = sample();
        let mut padded = bytes.clone();
        padded.push(0);

        assert!(decode_pack(Language::Ms, &bytes[..bytes.len() - 1]).is_err());
        assert!(decode_pack(Language::Ms, &padded).is_err());
        assert!(decode_pack(Language::Ms, &bytes[..10]).is_err());
    }

    #[test]
    fn digests_verify_only_when_given() {
        let bytes = sample();
        let digest = parse_sha256(&format!("{:x}", sha2::Sha256::digest(&bytes))).expect("hex");

        verify_digest("ms.pack", &bytes, None).expect("no digest, no check");
        verify_digest("ms.pack", &bytes, Some(digest)).expect("matching digest");
        let error = verify_digest("ms.pack", &bytes, Some([0; 32])).expect_err("mismatch");
        assert!(
            error
                .to_string()
                .starts_with("BLASPHEM_DIGEST_MISMATCH: ms.pack expected sha256 0000")
        );
        assert_eq!(parse_sha256("abc"), None);
        assert_eq!(parse_sha256(&"zz".repeat(32)), None);
    }

    use sha2::Digest;
}
