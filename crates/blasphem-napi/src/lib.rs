//! Node.js bindings for the multilingual nudge detector.
//!
//! The binary carries no language data. The JavaScript core feeds packs and
//! detect slices through [`EngineBuilder`] and reads verdicts from [`Engine`].
//! Every error message starts with the contract code.

use napi::bindgen_prelude::*;
use napi_derive::napi;

/// One verdict as a plain object.
#[napi(object)]
pub struct Judgement {
    pub safe: bool,
    pub score: f64,
    pub locale: Option<String>,
    pub grawlix: Option<String>,
}

/// Collects one locale at a time, then builds the engine.
#[napi]
pub struct EngineBuilder {
    sources: Vec<blasphem::EngineSource>,
    detect_language: bool,
    grawlix: bool,
}

#[napi]
impl EngineBuilder {
    #[napi(constructor)]
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
    #[napi]
    pub fn add(
        &mut self,
        locale: String,
        pack: Uint8Array,
        pack_sha256: Option<String>,
        detect: Option<Uint8Array>,
        detect_sha256: Option<String>,
    ) -> Result<()> {
        let source = blasphem::EngineSource::new(
            &locale,
            pack.to_vec(),
            pack_sha256.as_deref(),
            detect.map(|bytes| bytes.to_vec()),
            detect_sha256.as_deref(),
        )
        .map_err(|error| Error::from_reason(error.to_string()))?;
        self.sources.push(source);
        Ok(())
    }

    /// Verifies every digest, parses every pack, and returns the engine.
    /// The builder is emptied.
    ///
    /// # Errors
    ///
    /// Returns the first failure, message prefixed by its contract code.
    #[napi]
    pub fn build(&mut self) -> Result<Engine> {
        let sources = std::mem::take(&mut self.sources);
        blasphem::Engine::build(&sources, self.detect_language, self.grawlix)
            .map(|engine| Engine {
                engine: Some(engine),
            })
            .map_err(|error| Error::from_reason(error.to_string()))
    }
}

/// The Node-facing engine.
#[napi]
pub struct Engine {
    engine: Option<blasphem::Engine>,
}

#[napi]
impl Engine {
    /// The loaded locales as lowercase codes.
    #[napi(getter)]
    pub fn locales(&self) -> Result<Vec<String>> {
        Ok(self.open()?.locales())
    }

    /// Scores one message.
    ///
    /// # Errors
    ///
    /// Returns `BLASPHEM_CLOSED` after `close`.
    #[napi]
    pub fn judge(&self, text: String) -> Result<Judgement> {
        let verdict = self.open()?.judge(&text);
        Ok(Judgement {
            safe: verdict.safe,
            score: verdict.score,
            locale: verdict.locale,
            grawlix: verdict.grawlix,
        })
    }

    /// Releases the packs. Later calls fail with `BLASPHEM_CLOSED`.
    #[napi]
    pub fn close(&mut self) {
        self.engine = None;
    }

    fn open(&self) -> Result<&blasphem::Engine> {
        self.engine
            .as_ref()
            .ok_or_else(|| Error::from_reason("BLASPHEM_CLOSED: the judge was closed"))
    }
}
