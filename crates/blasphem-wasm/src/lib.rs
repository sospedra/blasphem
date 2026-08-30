//! Browser bindings for the multilingual nudge detector.
//!
//! The wasm carries no language data. The JavaScript core feeds packs and
//! detect slices through [`WasmEngineBuilder`] and reads verdicts from
//! [`WasmEngine`]. Every error is a string that starts with the contract code.

use blasphem::{Engine, EngineSource};
use wasm_bindgen::prelude::*;

/// Collects one locale at a time, then builds the engine.
#[wasm_bindgen(js_name = BlasphemEngineBuilder)]
pub struct WasmEngineBuilder {
    sources: Vec<EngineSource>,
    detect_language: bool,
    grawlix: bool,
}

#[wasm_bindgen(js_class = "BlasphemEngineBuilder")]
impl WasmEngineBuilder {
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(detect_language: bool, grawlix: bool) -> Self {
        Self {
            sources: Vec::new(),
            detect_language,
            grawlix,
        }
    }

    /// Adds one locale's pack, and its detect slice when detection is on.
    ///
    /// # Errors
    ///
    /// Returns `BLASPHEM_LOCALE_UNSUPPORTED` or `BLASPHEM_PACK_INVALID` text.
    pub fn add(
        &mut self,
        locale: &str,
        pack: Vec<u8>,
        pack_sha256: Option<String>,
        detect: Option<Vec<u8>>,
        detect_sha256: Option<String>,
    ) -> Result<(), JsValue> {
        let source = EngineSource::new(
            locale,
            pack,
            pack_sha256.as_deref(),
            detect,
            detect_sha256.as_deref(),
        )
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
        self.sources.push(source);
        Ok(())
    }

    /// Verifies every digest, parses every pack, and returns the engine.
    /// The builder is consumed.
    ///
    /// # Errors
    ///
    /// Returns the first failure, message prefixed by its contract code.
    pub fn build(self) -> Result<WasmEngine, JsValue> {
        Engine::build(&self.sources, self.detect_language, self.grawlix)
            .map(|engine| WasmEngine { engine })
            .map_err(|error| JsValue::from_str(&error.to_string()))
    }
}

/// The browser-facing engine. `judge` returns a plain object, so callers never free it.
#[wasm_bindgen(js_name = BlasphemEngine)]
pub struct WasmEngine {
    engine: Engine,
}

#[wasm_bindgen(js_class = "BlasphemEngine")]
impl WasmEngine {
    /// The loaded locales as lowercase codes.
    #[wasm_bindgen(getter)]
    #[must_use]
    pub fn locales(&self) -> Vec<String> {
        self.engine.locales()
    }

    /// Scores one message and returns `{ safe, score, locale, grawlix }`.
    ///
    /// # Errors
    ///
    /// Returns an error only when the host rejects a property write.
    pub fn judge(&self, text: &str) -> Result<JsValue, JsValue> {
        let verdict = self.engine.judge(text);
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
