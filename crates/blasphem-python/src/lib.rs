//! Python bindings for the multilingual nudge detector.
//!
//! The extension exposes one class, `Engine`, over the bytes-in core. The
//! pure-Python package `blasphem` reads the packs, builds engines, and keeps
//! the module-level judge; every error message starts with the contract code.

use std::sync::RwLock;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

type Entry = (String, Vec<u8>, Option<String>, Option<Vec<u8>>, Option<String>);

/// One judge over a fixed set of locales. Safe to share between threads.
#[pyclass(name = "Engine")]
struct Engine {
    inner: RwLock<Option<blasphem::Engine>>,
}

#[pymethods]
impl Engine {
    /// `entries` are `(locale, pack, pack_sha256, detect, detect_sha256)` tuples.
    #[new]
    #[pyo3(signature = (entries, detect_language = true, grawlix = false))]
    fn new(entries: Vec<Entry>, detect_language: bool, grawlix: bool) -> PyResult<Self> {
        let mut sources = Vec::with_capacity(entries.len());
        for (locale, pack, pack_sha256, detect, detect_sha256) in entries {
            let source = blasphem::EngineSource::new(
                &locale,
                pack,
                pack_sha256.as_deref(),
                detect,
                detect_sha256.as_deref(),
            )
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
            sources.push(source);
        }
        let engine = blasphem::Engine::build(&sources, detect_language, grawlix)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self {
            inner: RwLock::new(Some(engine)),
        })
    }

    /// The loaded locales as lowercase codes, in registry order.
    #[getter]
    fn locales(&self) -> PyResult<Vec<String>> {
        let guard = self.inner.read().expect("engine lock");
        guard
            .as_ref()
            .map(blasphem::Engine::locales)
            .ok_or_else(closed)
    }

    /// Scores one message as `(safe, score, locale, grawlix)`.
    fn judge(&self, text: &str) -> PyResult<(bool, f64, Option<String>, Option<String>)> {
        let guard = self.inner.read().expect("engine lock");
        let engine = guard.as_ref().ok_or_else(closed)?;
        let verdict = engine.judge(text);
        Ok((verdict.safe, verdict.score, verdict.locale, verdict.grawlix))
    }

    /// Releases the packs. Later calls fail with `BLASPHEM_CLOSED`.
    fn close(&self) {
        *self.inner.write().expect("engine lock") = None;
    }
}

fn closed() -> PyErr {
    PyValueError::new_err("BLASPHEM_CLOSED: the judge was closed")
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<Engine>()
}
