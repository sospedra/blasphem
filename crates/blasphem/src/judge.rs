//! A high-level client over locale selection, scoring, and masking.

#[cfg(feature = "embedded")]
use crate::embedded::embedded_detector;
use crate::grawlix::{apply_grawlix, masked_spans};
use crate::language::Language;
use crate::pack::{PackError, PackSource, detect_file_name, pack_file_name, verify_digest};
use crate::policy::{PolicyResult, ReplyTarget};
use crate::runtime::{NudgeDetector, RuntimeInitError};

#[cfg(feature = "language-detection")]
use crate::language_detection::{
    LanguageDetector, LanguageDetectorError, LanguageIdentifier, LanguageResolution,
};

/// Options for one judge. Every field has a working default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeOptions {
    /// Locales to load. Empty loads every compiled language.
    pub locales: Vec<Language>,
    /// Route by detected language instead of scoring every loaded locale.
    pub detect_language: bool,
    /// Populate [`Judgement::grawlix`] for unsafe verdicts.
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
    /// The masked text when [`JudgeOptions::grawlix`] is set and the verdict is unsafe.
    /// Safe verdicts return `None`.
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

/// Anything that stops a judge from being built. Every message starts with
/// the error code the JavaScript contract exposes.
#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error("BLASPHEM_PACK_INVALID: {0}")]
    Runtime(#[from] RuntimeInitError),
    #[cfg(feature = "language-detection")]
    #[error("BLASPHEM_PACK_INVALID: {0}")]
    LanguageDetector(#[from] LanguageDetectorError),
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error("BLASPHEM_LOCALES_EMPTY: no locale was given")]
    NoLocales,
    #[error("BLASPHEM_PACK_INVALID: {} was given twice", pack_file_name(*.0))]
    DuplicateLocale(Language),
    #[error("BLASPHEM_PACK_INVALID: {} is required when language detection is on", detect_file_name(*.0))]
    MissingDetect(Language),
    #[error("BLASPHEM_PACK_INVALID: this build has no language detection")]
    DetectionUnavailable,
    #[error("BLASPHEM_LOCALE_MISSING: {0} is not compiled into this build")]
    LocaleUnavailable(Language),
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
    /// Builds one detector per requested locale from the embedded data.
    ///
    /// # Errors
    ///
    /// Returns an error when an embedded resource is invalid, or when the
    /// language detector cannot start.
    #[cfg(feature = "embedded")]
    pub fn new(options: JudgeOptions) -> Result<Self, JudgeError> {
        #[cfg(not(feature = "language-detection"))]
        ensure_no_detection(options.detect_language, &[])?;
        let locales = requested_locales(&options.locales)?;
        let mut detectors = Vec::new();
        for &language in &locales {
            detectors.push((language, embedded_detector(language)?));
        }

        Ok(Self {
            detectors,
            #[cfg(feature = "language-detection")]
            identifier: identifier_for(options.detect_language, &locales)?,
            grawlix: options.grawlix,
        })
    }

    /// Builds one detector per pack, verifying each digest the caller supplies.
    ///
    /// With `detect_language` every source needs its detect slice, and the
    /// judge routes by the merged slices. Without it, the judge scores every
    /// loaded locale and reports the highest.
    ///
    /// # Errors
    ///
    /// Returns an error naming the file for a digest mismatch, a foreign
    /// format version, a malformed pack, a repeated locale, or a missing slice.
    pub fn from_packs(
        sources: &[PackSource<'_>],
        detect_language: bool,
        grawlix: bool,
    ) -> Result<Self, JudgeError> {
        if sources.is_empty() {
            return Err(JudgeError::NoLocales);
        }
        let mut detectors: Vec<(Language, NudgeDetector)> = Vec::with_capacity(sources.len());
        let mut slices = Vec::with_capacity(sources.len());
        for source in sources {
            if detectors
                .iter()
                .any(|(language, _)| *language == source.language)
            {
                return Err(JudgeError::DuplicateLocale(source.language));
            }
            verify_digest(
                &pack_file_name(source.language),
                source.pack,
                source.pack_sha256,
            )?;
            let detector =
                NudgeDetector::from_pack(source.language, source.pack).map_err(lift_runtime)?;
            detectors.push((source.language, detector));
            if !detect_language {
                continue;
            }
            let detect = source
                .detect
                .ok_or(JudgeError::MissingDetect(source.language))?;
            verify_digest(
                &detect_file_name(source.language),
                detect,
                source.detect_sha256,
            )?;
            slices.push(detect);
        }
        detectors.sort_by_key(|(language, _)| language.index());
        #[cfg(feature = "language-detection")]
        let identifier = identifier_from_slices(detect_language, &slices)?;
        #[cfg(not(feature = "language-detection"))]
        ensure_no_detection(detect_language, &slices)?;

        Ok(Self {
            detectors,
            #[cfg(feature = "language-detection")]
            identifier,
            grawlix,
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
            grawlix: if nudge.should_nudge {
                self.mask(&result)
            } else {
                None
            },
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

fn lift_runtime(error: RuntimeInitError) -> JudgeError {
    match error {
        RuntimeInitError::Pack(pack) => JudgeError::Pack(pack),
        other => JudgeError::Runtime(other),
    }
}

#[cfg(feature = "language-detection")]
fn identifier_from_slices(
    detect_language: bool,
    slices: &[&[u8]],
) -> Result<Option<LanguageDetector>, JudgeError> {
    if !detect_language {
        return Ok(None);
    }
    Ok(Some(LanguageDetector::from_slices(slices)?))
}

#[cfg(not(feature = "language-detection"))]
fn ensure_no_detection(detect_language: bool, _slices: &[&[u8]]) -> Result<(), JudgeError> {
    if detect_language {
        return Err(JudgeError::DetectionUnavailable);
    }
    Ok(())
}

#[cfg(all(feature = "language-detection", feature = "embedded"))]
fn identifier_for(
    detect_language: bool,
    locales: &[Language],
) -> Result<Option<LanguageDetector>, JudgeError> {
    if !detect_language {
        return Ok(None);
    }
    let slices: Vec<_> = locales
        .iter()
        .copied()
        .map(crate::embedded::embedded_detect_bytes)
        .collect();
    Ok(Some(LanguageDetector::from_slices(&slices)?))
}

#[cfg(feature = "embedded")]
fn requested_locales(locales: &[Language]) -> Result<Vec<Language>, JudgeError> {
    let compiled = crate::embedded::compiled_locales();
    if locales.is_empty() {
        return if compiled.is_empty() {
            Err(JudgeError::NoLocales)
        } else {
            Ok(compiled)
        };
    }
    for &language in locales {
        if !compiled.contains(&language) {
            return Err(JudgeError::LocaleUnavailable(language));
        }
    }
    let mut requested = locales.to_vec();
    requested.sort_by_key(|language| language.index());
    requested.dedup();
    Ok(requested)
}
