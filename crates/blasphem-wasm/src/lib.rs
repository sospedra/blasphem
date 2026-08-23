//! Browser bindings for the multilingual nudge detector.

use std::str::FromStr;

use blasphem::{Language, LanguageSelection, NudgeDetector, ReplyTarget};
#[cfg(feature = "language-detection")]
use blasphem::{LanguageDetector, LanguageIdentifier, LanguageResolution};
use wasm_bindgen::prelude::*;

/// A platform-neutral result used by the browser binding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoreResult {
    pub ok: bool,
    pub score: u8,
    pub threshold: u8,
    pub should_nudge: bool,
    pub evaluated: bool,
    pub resolved_language: &'static str,
    pub language_reliable: bool,
    pub language_score: Option<f32>,
}

#[cfg(feature = "language-detection")]
#[derive(Debug)]
struct DetectorSlots {
    slots: Box<[Option<NudgeDetector>; 15]>,
}

#[cfg(feature = "language-detection")]
impl DetectorSlots {
    fn from_languages(languages: impl IntoIterator<Item = Language>) -> Result<Self, String> {
        let mut slots = Box::new(std::array::from_fn(|_| None));
        for language in languages {
            slots[language.index()] = Some(embedded_detector(language)?);
        }
        Ok(Self { slots })
    }

    fn get(&self, language: Language) -> Option<&NudgeDetector> {
        self.slots[language.index()].as_ref()
    }
}

#[derive(Debug)]
enum DetectorMode {
    Explicit(NudgeDetector),
    #[cfg(feature = "language-detection")]
    Automatic {
        identifier: LanguageDetector,
        detectors: DetectorSlots,
    },
}

/// A detector with embedded data for explicit or automatic language selection.
#[derive(Debug)]
pub struct DetectorCore {
    selection_code: &'static str,
    mode: DetectorMode,
}

impl DetectorCore {
    /// Builds a detector from the embedded model and HurtLex data.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported language or invalid embedded data.
    pub fn new(language: &str) -> Result<Self, String> {
        match LanguageSelection::from_str(language).map_err(|error| error.to_string())? {
            LanguageSelection::Explicit(language) => Ok(Self {
                selection_code: language.code(),
                mode: DetectorMode::Explicit(embedded_detector(language)?),
            }),
            LanguageSelection::Auto => Self::new_automatic(),
        }
    }

    #[must_use]
    pub const fn language(&self) -> &'static str {
        self.selection_code
    }

    #[must_use]
    pub fn check(&self, text: &str) -> CoreResult {
        match &self.mode {
            DetectorMode::Explicit(detector) => {
                evaluated_result(detector, text, detector.language(), None)
            }
            #[cfg(feature = "language-detection")]
            DetectorMode::Automatic {
                identifier,
                detectors,
            } => {
                let detection = identifier.identify(text);
                let (LanguageResolution::Known(language), true, Some(score)) =
                    (detection.resolution, detection.reliable, detection.score)
                else {
                    return unknown_result();
                };
                let Some(detector) = detectors.get(language) else {
                    return unknown_result();
                };
                evaluated_result(detector, text, language, Some(score))
            }
        }
    }

    #[cfg(feature = "language-detection")]
    fn new_automatic() -> Result<Self, String> {
        let identifier = LanguageDetector::new().map_err(|error| error.to_string())?;
        let detectors = DetectorSlots::from_languages(Language::ALL)?;

        Ok(Self {
            selection_code: "AUTO",
            mode: DetectorMode::Automatic {
                identifier,
                detectors,
            },
        })
    }

    #[cfg(not(feature = "language-detection"))]
    fn new_automatic() -> Result<Self, String> {
        Err("AUTO requires the language-detection feature".to_owned())
    }
}

fn embedded_detector(language: Language) -> Result<NudgeDetector, String> {
    NudgeDetector::from_hurtlex_bytes(language, Some(embedded_hurtlex_bytes(language)))
        .map_err(|error| error.to_string())
}

fn evaluated_result(
    detector: &NudgeDetector,
    text: &str,
    language: Language,
    language_score: Option<f32>,
) -> CoreResult {
    let result = detector.check(text, ReplyTarget::Unknown);
    CoreResult {
        ok: !result.should_nudge,
        score: result.score,
        threshold: result.threshold,
        should_nudge: result.should_nudge,
        evaluated: true,
        resolved_language: language.code(),
        language_reliable: true,
        language_score,
    }
}

#[cfg(feature = "language-detection")]
const fn unknown_result() -> CoreResult {
    CoreResult {
        ok: true,
        score: 0,
        threshold: 50,
        should_nudge: false,
        evaluated: false,
        resolved_language: "unknown",
        language_reliable: false,
        language_score: None,
    }
}

/// The browser-facing detector.
#[wasm_bindgen(js_name = BlasphemDetector)]
pub struct WasmDetector {
    core: DetectorCore,
}

#[wasm_bindgen(js_class = "BlasphemDetector")]
impl WasmDetector {
    /// Builds one detector for an explicit language code or `AUTO`.
    #[wasm_bindgen(constructor)]
    pub fn new(language: &str) -> Result<WasmDetector, JsValue> {
        DetectorCore::new(language)
            .map(|core| Self { core })
            .map_err(|error| JsValue::from_str(&error))
    }

    #[wasm_bindgen(getter)]
    pub fn language(&self) -> String {
        self.core.language().to_owned()
    }

    #[must_use]
    pub fn check(&self, text: &str) -> WasmCheckResult {
        WasmCheckResult::from(self.core.check(text))
    }
}

/// The small browser result for the pre-send nudge.
#[wasm_bindgen(js_name = BlasphemResult)]
pub struct WasmCheckResult {
    inner: CoreResult,
}

impl From<CoreResult> for WasmCheckResult {
    fn from(inner: CoreResult) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = "BlasphemResult")]
impl WasmCheckResult {
    #[wasm_bindgen(getter)]
    pub fn ok(&self) -> bool {
        self.inner.ok
    }

    #[wasm_bindgen(getter)]
    pub fn score(&self) -> u8 {
        self.inner.score
    }

    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> u8 {
        self.inner.threshold
    }

    #[wasm_bindgen(getter, js_name = shouldNudge)]
    pub fn should_nudge(&self) -> bool {
        self.inner.should_nudge
    }

    #[wasm_bindgen(getter)]
    pub fn evaluated(&self) -> bool {
        self.inner.evaluated
    }

    #[wasm_bindgen(getter, js_name = resolvedLanguage)]
    pub fn resolved_language(&self) -> String {
        self.inner.resolved_language.to_owned()
    }

    #[wasm_bindgen(getter, js_name = languageReliable)]
    pub fn language_reliable(&self) -> bool {
        self.inner.language_reliable
    }

    #[wasm_bindgen(getter, js_name = languageScore)]
    pub fn language_score(&self) -> Option<f32> {
        self.inner.language_score
    }
}

const fn embedded_hurtlex_bytes(language: Language) -> &'static [u8] {
    match language {
        Language::En => include_bytes!("../../../data/raw-v1/hurtlex/EN/1.2/hurtlex_EN.tsv"),
        Language::Zh => include_bytes!("../../../data/raw-v1/hurtlex/ZH/1.2/hurtlex_ZH.tsv"),
        Language::Es => include_bytes!("../../../data/raw-v1/hurtlex/ES/1.2/hurtlex_ES.tsv"),
        Language::Ar => include_bytes!("../../../data/raw-v1/hurtlex/AR/1.2/hurtlex_AR.tsv"),
        Language::Ms => include_bytes!("../../../data/raw-v1/hurtlex/ID/1.2/hurtlex_ID.tsv"),
        Language::Pt => include_bytes!("../../../data/raw-v1/hurtlex/PT/1.2/hurtlex_PT.tsv"),
        Language::Fr => include_bytes!("../../../data/raw-v1/hurtlex/FR/1.2/hurtlex_FR.tsv"),
        Language::Hi => include_bytes!("../../../data/raw-v1/hurtlex/HI/1.2/hurtlex_HI.tsv"),
        Language::Ru => include_bytes!("../../../data/raw-v1/hurtlex/RU/1.2/hurtlex_RU.tsv"),
        Language::Ja => include_bytes!("../../../data/raw-v1/hurtlex/JA/1.2/hurtlex_JA.tsv"),
        Language::De => include_bytes!("../../../data/raw-v1/hurtlex/DE/1.2/hurtlex_DE.tsv"),
        Language::Tr => include_bytes!("../../../data/raw-v1/hurtlex/TR/1.2/hurtlex_TR.tsv"),
        Language::Vi => include_bytes!("../../../data/raw-v1/hurtlex/VI/1.2/hurtlex_VI.tsv"),
        Language::Ko => include_bytes!("../../../data/raw-v1/hurtlex/KO/1.2/hurtlex_KO.tsv"),
        Language::It => include_bytes!("../../../data/raw-v1/hurtlex/IT/1.2/hurtlex_IT.tsv"),
    }
}

#[cfg(all(test, feature = "language-detection"))]
mod tests {
    use super::*;

    #[test]
    fn a_partial_detector_set_keeps_language_keys_and_reports_missing_packs() {
        let detectors =
            DetectorSlots::from_languages([Language::En, Language::Es]).expect("detector slots");

        assert_eq!(
            detectors.get(Language::En).map(NudgeDetector::language),
            Some(Language::En),
        );
        assert_eq!(
            detectors.get(Language::Es).map(NudgeDetector::language),
            Some(Language::Es),
        );
        assert!(detectors.get(Language::Zh).is_none());
    }
}
