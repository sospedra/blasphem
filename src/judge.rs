//! A high-level client over locale selection, scoring, and masking.

use crate::embedded::embedded_detector;
use crate::grawlix::{apply_grawlix, masked_spans};
use crate::language::Language;
use crate::policy::{PolicyResult, ReplyTarget};
use crate::runtime::{NudgeDetector, RuntimeInitError};

#[cfg(feature = "language-detection")]
use crate::language_detection::{
    LanguageDetector, LanguageDetectorError, LanguageIdentifier, LanguageResolution,
};

/// Options for one judge. Every field has a working default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeOptions {
    /// Locales to load. Empty loads every supported language.
    pub locales: Vec<Language>,
    /// Route by detected language instead of scoring every loaded locale.
    pub detect_language: bool,
    /// Populate [`Judgement::grawlix`].
    pub grawlix: bool,
}

impl Default for JudgeOptions {
    fn default() -> Self {
        Self {
            locales: Vec::new(),
            detect_language: true,
            grawlix: false,
        }
    }
}

/// One verdict for one message.
#[derive(Debug, Clone, PartialEq)]
pub struct Judgement {
    /// True when no nudge is due. Unroutable text is safe; the nudge fails open.
    pub safe: bool,
    /// Ordinal risk from 0.0 through 1.0. Not a probability.
    pub score: f64,
    /// The locale that produced the score.
    pub locale: Option<Language>,
    /// The masked text when [`JudgeOptions::grawlix`] is set.
    pub grawlix: Option<String>,
}

impl Judgement {
    const fn fails_open() -> Self {
        Self {
            safe: true,
            score: 0.0,
            locale: None,
            grawlix: None,
        }
    }
}

/// Anything that stops a judge from being built.
#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error(transparent)]
    Runtime(#[from] RuntimeInitError),
    #[cfg(feature = "language-detection")]
    #[error(transparent)]
    LanguageDetector(#[from] LanguageDetectorError),
}

/// A reusable judge holding one detector per loaded locale.
///
/// Build it once and call [`Judge::judge`] many times. Each locale
/// carries its own lexicon and sparse table, so construction is the
/// expensive step.
#[derive(Debug)]
pub struct Judge {
    detectors: Vec<(Language, NudgeDetector)>,
    #[cfg(feature = "language-detection")]
    identifier: Option<LanguageDetector>,
    grawlix: bool,
}

impl Judge {
    /// Builds one detector per requested locale.
    ///
    /// # Errors
    ///
    /// Returns an error when an embedded resource is invalid, or when the
    /// language detector cannot start.
    pub fn new(options: JudgeOptions) -> Result<Self, JudgeError> {
        let mut detectors = Vec::new();
        for language in requested_locales(&options.locales) {
            detectors.push((language, embedded_detector(language)?));
        }

        Ok(Self {
            detectors,
            #[cfg(feature = "language-detection")]
            identifier: identifier_for(options.detect_language)?,
            grawlix: options.grawlix,
        })
    }

    /// Scores one message.
    #[must_use]
    pub fn judge(&self, text: &str) -> Judgement {
        let Some((language, detector)) = self.select(text) else {
            return Judgement::fails_open();
        };
        let result = detector.analyze(text, ReplyTarget::Unknown);
        let nudge = result.nudge();

        Judgement {
            safe: !nudge.should_nudge,
            score: f64::from(nudge.score) / 100.0,
            locale: Some(language),
            grawlix: self.mask(&result),
        }
    }

    /// The locales this judge loaded, in registry order.
    #[must_use]
    pub fn locales(&self) -> Vec<Language> {
        self.detectors
            .iter()
            .map(|(language, _)| *language)
            .collect()
    }

    fn mask(&self, result: &PolicyResult) -> Option<String> {
        if !self.grawlix {
            return None;
        }
        Some(apply_grawlix(&result.original_text, &masked_spans(result)))
    }

    #[cfg(feature = "language-detection")]
    fn find(&self, language: Language) -> Option<(Language, &NudgeDetector)> {
        self.detectors
            .iter()
            .find(|(candidate, _)| *candidate == language)
            .map(|(candidate, detector)| (*candidate, detector))
    }

    fn highest_scoring(&self, text: &str) -> Option<(Language, &NudgeDetector)> {
        self.detectors
            .iter()
            .max_by_key(|(_, detector)| detector.check(text, ReplyTarget::Unknown).score)
            .map(|(language, detector)| (*language, detector))
    }

    #[cfg(feature = "language-detection")]
    fn select(&self, text: &str) -> Option<(Language, &NudgeDetector)> {
        let Some(identifier) = self.identifier.as_ref() else {
            return self.highest_scoring(text);
        };
        let detection = identifier.identify(text);
        let (LanguageResolution::Known(language), true) =
            (detection.resolution, detection.reliable)
        else {
            return None;
        };
        self.find(language)
    }

    #[cfg(not(feature = "language-detection"))]
    fn select(&self, text: &str) -> Option<(Language, &NudgeDetector)> {
        self.highest_scoring(text)
    }
}

#[cfg(feature = "language-detection")]
fn identifier_for(detect_language: bool) -> Result<Option<LanguageDetector>, JudgeError> {
    if !detect_language {
        return Ok(None);
    }
    Ok(Some(LanguageDetector::new()?))
}

fn requested_locales(locales: &[Language]) -> Vec<Language> {
    if locales.is_empty() {
        return Language::ALL.to_vec();
    }
    let mut requested = locales.to_vec();
    requested.sort_by_key(|language| language.index());
    requested.dedup();
    requested
}
