use std::str::FromStr;

#[cfg(feature = "language-detection")]
use thiserror::Error;

use crate::{Language, UnsupportedLanguage};

/// An explicit language or automatic language selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageSelection {
    Explicit(Language),
    Auto,
}

impl FromStr for LanguageSelection {
    type Err = UnsupportedLanguage;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.trim().eq_ignore_ascii_case("AUTO") {
            return Ok(Self::Auto);
        }
        Language::from_str(value).map(Self::Explicit)
    }
}

/// The source of a language resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageSource {
    Explicit,
    Automatic,
}

/// A supported language resolution or an unknown result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageResolution {
    Known(Language),
    Unknown,
}

/// The product language routing result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LanguageDetection {
    pub source: LanguageSource,
    pub resolution: LanguageResolution,
    pub reliable: bool,
    pub score: Option<f32>,
    pub feature_count: Option<usize>,
}

/// Identifies a supported language from text.
pub trait LanguageIdentifier {
    /// Identifies one text without changing the identifier.
    #[must_use]
    fn identify(&self, text: &str) -> LanguageDetection;
}

/// An embedded language detector.
#[cfg(feature = "language-detection")]
#[derive(Debug)]
pub struct LanguageDetector {
    detector: blasphem_language::Detector,
}

/// An error from embedded language model initialization.
#[cfg(feature = "language-detection")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("cannot initialize the language model: {0}")]
pub struct LanguageDetectorError(#[from] blasphem_language::ModelError);

#[cfg(feature = "language-detection")]
impl LanguageDetector {
    /// Loads and validates the embedded language model.
    ///
    /// # Errors
    ///
    /// Returns an error when the embedded model is invalid.
    pub fn new() -> Result<Self, LanguageDetectorError> {
        Ok(Self {
            detector: blasphem_language::Detector::new()?,
        })
    }
}

#[cfg(feature = "language-detection")]
impl LanguageIdentifier for LanguageDetector {
    fn identify(&self, text: &str) -> LanguageDetection {
        let detection = self.detector.detect(text);
        let resolution = match (detection.reliable, detection.language) {
            (true, Some(language)) => LanguageResolution::Known(map_language(language)),
            _ => LanguageResolution::Unknown,
        };

        LanguageDetection {
            source: LanguageSource::Automatic,
            resolution,
            reliable: detection.reliable,
            score: detection.language.map(|_| detection.top_score),
            feature_count: Some(detection.feature_count),
        }
    }
}

/// Resolves an explicit or automatic language selection.
#[must_use]
pub fn resolve_language<I: LanguageIdentifier + ?Sized>(
    selection: LanguageSelection,
    text: &str,
    identifier: &I,
) -> LanguageDetection {
    match selection {
        LanguageSelection::Explicit(language) => LanguageDetection {
            source: LanguageSource::Explicit,
            resolution: LanguageResolution::Known(language),
            reliable: true,
            score: None,
            feature_count: None,
        },
        LanguageSelection::Auto => identifier.identify(text),
    }
}

#[cfg(feature = "language-detection")]
const fn map_language(language: blasphem_language::Language) -> Language {
    match language {
        blasphem_language::Language::Arabic => Language::Ar,
        blasphem_language::Language::German => Language::De,
        blasphem_language::Language::English => Language::En,
        blasphem_language::Language::Spanish => Language::Es,
        blasphem_language::Language::French => Language::Fr,
        blasphem_language::Language::Hindi => Language::Hi,
        blasphem_language::Language::Italian => Language::It,
        blasphem_language::Language::Japanese => Language::Ja,
        blasphem_language::Language::Korean => Language::Ko,
        blasphem_language::Language::Malay => Language::Ms,
        blasphem_language::Language::Portuguese => Language::Pt,
        blasphem_language::Language::Russian => Language::Ru,
        blasphem_language::Language::Turkish => Language::Tr,
        blasphem_language::Language::Vietnamese => Language::Vi,
        blasphem_language::Language::Chinese => Language::Zh,
    }
}
