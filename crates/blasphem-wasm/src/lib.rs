//! Browser bindings for the multilingual nudge detector.

use std::str::FromStr;

use blasphem::{Judge, JudgeOptions, Language, LanguageSelection, NudgeDetector, ReplyTarget};
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
    blasphem::embedded_detector(language).map_err(|error| error.to_string())
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

/// A platform-neutral verdict used by the browser binding.
#[derive(Debug, Clone, PartialEq)]
pub struct CoreJudgement {
    pub safe: bool,
    pub score: f64,
    pub locale: Option<String>,
    pub grawlix: Option<String>,
}

/// A reusable judge behind the wasm boundary.
#[derive(Debug)]
pub struct JudgeCore {
    inner: Judge,
}

impl JudgeCore {
    /// Builds a judge for the requested lowercase locale codes.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported locale or invalid embedded data.
    pub fn new(locales: &[String], detect_language: bool, grawlix: bool) -> Result<Self, String> {
        let options = JudgeOptions {
            locales: parse_locales(locales)?,
            detect_language,
            grawlix,
        };
        Judge::new(options)
            .map(|inner| Self { inner })
            .map_err(|error| error.to_string())
    }

    #[must_use]
    pub fn judge(&self, text: &str) -> CoreJudgement {
        let verdict = self.inner.judge(text);
        CoreJudgement {
            safe: verdict.safe,
            score: verdict.score,
            locale: verdict
                .locale
                .map(|language| language.code().to_ascii_lowercase()),
            grawlix: verdict.grawlix,
        }
    }
}

fn parse_locales(codes: &[String]) -> Result<Vec<Language>, String> {
    codes
        .iter()
        .map(|code| Language::from_str(code).map_err(|_| format!("unsupported locale \"{code}\"")))
        .collect()
}

/// The browser-facing judge. Returns a plain object, so callers never free it.
#[wasm_bindgen(js_name = BlasphemJudge)]
pub struct WasmJudge {
    core: JudgeCore,
}

#[wasm_bindgen(js_class = "BlasphemJudge")]
impl WasmJudge {
    /// Builds one judge for the requested locales.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported locale or invalid embedded data.
    #[wasm_bindgen(constructor)]
    pub fn new(
        locales: Vec<String>,
        detect_language: bool,
        grawlix: bool,
    ) -> Result<WasmJudge, JsValue> {
        JudgeCore::new(&locales, detect_language, grawlix)
            .map(|core| Self { core })
            .map_err(|error| JsValue::from_str(&error))
    }

    /// Scores one message and returns `{ safe, score, locale, grawlix }`.
    ///
    /// # Errors
    ///
    /// Returns an error when the host rejects a property write.
    pub fn judge(&self, text: &str) -> Result<JsValue, JsValue> {
        let verdict = self.core.judge(text);
        let object = js_sys::Object::new();
        set(&object, "safe", JsValue::from_bool(verdict.safe))?;
        set(&object, "score", JsValue::from_f64(verdict.score))?;
        set(&object, "locale", optional_text(verdict.locale))?;
        set(&object, "grawlix", optional_text(verdict.grawlix))?;
        Ok(object.into())
    }
}

fn optional_text(value: Option<String>) -> JsValue {
    value.map_or(JsValue::NULL, |text| JsValue::from_str(&text))
}

fn set(object: &js_sys::Object, key: &str, value: JsValue) -> Result<(), JsValue> {
    js_sys::Reflect::set(object, &JsValue::from_str(key), &value).map(|_| ())
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
