//! The bytes-in engine every binding wraps.
//!
//! A binding collects [`EngineSource`] values from strings and byte buffers,
//! then builds one [`Engine`]. Every error message starts with the code the
//! JavaScript contract exposes, so a binding can pass the text through.

use std::str::FromStr;

use thiserror::Error;

use crate::judge::{Judge, JudgeError};
use crate::language::Language;
use crate::pack::{PackSource, detect_file_name, pack_file_name, parse_sha256};

/// One locale's files, owned until the engine is built.
#[derive(Debug, Clone)]
pub struct EngineSource {
    language: Language,
    pack: Vec<u8>,
    pack_sha256: Option<[u8; 32]>,
    detect: Option<Vec<u8>>,
    detect_sha256: Option<[u8; 32]>,
}

/// One verdict in the shape every binding returns.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineJudgement {
    pub safe: bool,
    pub score: f64,
    pub locale: Option<String>,
    pub grawlix: Option<String>,
}

/// Anything a binding can fail on. Messages start with the contract code.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("BLASPHEM_LOCALE_UNSUPPORTED: unsupported locale {0:?}")]
    UnsupportedLocale(String),
    #[error("BLASPHEM_PACK_INVALID: {file} digest is not 64 hexadecimal characters")]
    BadDigest { file: String },
    #[error(transparent)]
    Judge(#[from] JudgeError),
}

impl EngineSource {
    /// Parses the strings a binding receives. Digests are hexadecimal or absent.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown locale or a malformed digest string.
    pub fn new(
        locale: &str,
        pack: Vec<u8>,
        pack_sha256: Option<&str>,
        detect: Option<Vec<u8>>,
        detect_sha256: Option<&str>,
    ) -> Result<Self, EngineError> {
        let language = Language::from_str(locale)
            .map_err(|_| EngineError::UnsupportedLocale(locale.to_owned()))?;
        Ok(Self {
            language,
            pack,
            pack_sha256: parse_digest(&pack_file_name(language), pack_sha256)?,
            detect,
            detect_sha256: parse_digest(&detect_file_name(language), detect_sha256)?,
        })
    }

    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    fn as_pack_source(&self) -> PackSource<'_> {
        PackSource {
            language: self.language,
            pack: &self.pack,
            pack_sha256: self.pack_sha256,
            detect: self.detect.as_deref(),
            detect_sha256: self.detect_sha256,
        }
    }
}

fn parse_digest(file: &str, text: Option<&str>) -> Result<Option<[u8; 32]>, EngineError> {
    let Some(text) = text else {
        return Ok(None);
    };
    parse_sha256(text)
        .map(Some)
        .ok_or_else(|| EngineError::BadDigest {
            file: file.to_owned(),
        })
}

/// A judge built from packs, with string locales on both sides.
#[derive(Debug)]
pub struct Engine {
    judge: Judge,
}

impl Engine {
    /// Verifies the digests, parses the packs, and builds the judge.
    ///
    /// # Errors
    ///
    /// Returns the first pack, slice, or option error, message prefixed by its code.
    pub fn build(
        sources: &[EngineSource],
        detect_language: bool,
        grawlix: bool,
    ) -> Result<Self, EngineError> {
        let borrowed: Vec<PackSource<'_>> =
            sources.iter().map(EngineSource::as_pack_source).collect();
        Ok(Self {
            judge: Judge::from_packs(&borrowed, detect_language, grawlix)?,
        })
    }

    /// The loaded locales as lowercase codes, in registry order.
    #[must_use]
    pub fn locales(&self) -> Vec<String> {
        self.judge
            .locales()
            .into_iter()
            .map(|language| language.code().to_ascii_lowercase())
            .collect()
    }

    /// Scores one message. Never fails; unroutable text is safe.
    #[must_use]
    pub fn judge(&self, text: &str) -> EngineJudgement {
        let verdict = self.judge.judge(text);
        EngineJudgement {
            safe: verdict.safe,
            score: verdict.score,
            locale: verdict
                .locale
                .map(|language| language.code().to_ascii_lowercase()),
            grawlix: verdict.grawlix,
        }
    }
}
