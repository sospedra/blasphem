//! A C ABI over the bytes-in engine, for Nitro Modules and other native hosts.
//!
//! Ownership: the caller owns a builder until `blasphem_builder_build`
//! consumes it, owns an engine until `blasphem_engine_free`, and owns every
//! `blasphem_judgement` until `blasphem_judgement_free`. Error text lives in a
//! thread-local buffer that `blasphem_last_error` exposes until the next
//! failing call on the same thread.

// C callers see these names, so they follow C conventions.
#![allow(non_camel_case_types)]

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::ptr;

use blasphem::{Engine, EngineSource};

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

/// An opaque builder. Create with `blasphem_builder_new`.
pub struct blasphem_builder {
    sources: Vec<EngineSource>,
    detect_language: bool,
    grawlix: bool,
    /// The last failure of `add` or `build` on this builder. Bindings whose
    /// threads migrate between calls read this instead of the thread-local.
    error: Option<CString>,
}

// Every binding may call `judge` from several threads at once.
const _: () = {
    const fn assert_sync<T: Sync + Send>() {}
    assert_sync::<Engine>();
};

/// An opaque engine. Create with `blasphem_builder_build`.
pub struct blasphem_engine {
    engine: Engine,
}

/// One verdict. `locale` and `grawlix` are NUL-terminated UTF-8 or null.
#[repr(C)]
pub struct blasphem_judgement {
    pub safe: bool,
    pub score: f64,
    pub locale: *mut c_char,
    pub grawlix: *mut c_char,
}

fn c_text(message: String) -> CString {
    CString::new(message.replace('\0', " ")).expect("NUL bytes replaced")
}

fn set_error(message: String) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(c_text(message)));
}

/// Records a failure on the builder and in the thread-local slot.
fn set_builder_error(builder: &mut blasphem_builder, message: String) {
    builder.error = Some(c_text(message.clone()));
    set_error(message);
}

/// # Safety
///
/// `pointer` must be null or a valid NUL-terminated string.
unsafe fn text<'a>(pointer: *const c_char) -> Option<&'a str> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: the caller promises a valid NUL-terminated string.
    unsafe { CStr::from_ptr(pointer) }.to_str().ok()
}

/// # Safety
///
/// `pointer` must be null with `length == 0`, or valid for `length` bytes.
unsafe fn bytes<'a>(pointer: *const u8, length: usize) -> Option<&'a [u8]> {
    if pointer.is_null() {
        return None;
    }
    // SAFETY: the caller promises `length` readable bytes.
    Some(unsafe { std::slice::from_raw_parts(pointer, length) })
}

fn owned_text(value: Option<String>) -> *mut c_char {
    value
        .and_then(|text| CString::new(text).ok())
        .map_or(ptr::null_mut(), CString::into_raw)
}

/// Starts a builder. Never fails.
#[unsafe(no_mangle)]
pub extern "C" fn blasphem_builder_new(
    detect_language: bool,
    grawlix: bool,
) -> *mut blasphem_builder {
    Box::into_raw(Box::new(blasphem_builder {
        sources: Vec::new(),
        detect_language,
        grawlix,
        error: None,
    }))
}

/// The last failure of `blasphem_builder_add` or `blasphem_builder_build` on
/// this builder, or null. Valid until the next call on the builder or its free.
///
/// # Safety
///
/// `builder` must be live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_builder_error(builder: *const blasphem_builder) -> *const c_char {
    // SAFETY: the caller promises a live builder or null.
    unsafe { builder.as_ref() }
        .and_then(|builder| builder.error.as_ref())
        .map_or(ptr::null(), |text| text.as_ptr())
}

/// Adds one locale. Returns 0 on success, 1 on failure with the message in
/// `blasphem_last_error`. Digest strings are optional hexadecimal.
///
/// # Safety
///
/// `builder` must come from `blasphem_builder_new` and not yet be built or
/// freed. String and byte pointers follow the rules of `text` and `bytes`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_builder_add(
    builder: *mut blasphem_builder,
    locale: *const c_char,
    pack: *const u8,
    pack_len: usize,
    pack_sha256: *const c_char,
    detect: *const u8,
    detect_len: usize,
    detect_sha256: *const c_char,
) -> i32 {
    // SAFETY: the caller promises a live builder pointer.
    let Some(builder) = (unsafe { builder.as_mut() }) else {
        set_error("BLASPHEM_PACK_INVALID: null builder".to_owned());
        return 1;
    };
    // SAFETY: pointer contracts are documented on this function.
    let (locale, pack, pack_digest, detect, detect_digest) = unsafe {
        (
            text(locale),
            bytes(pack, pack_len),
            text(pack_sha256),
            bytes(detect, detect_len),
            text(detect_sha256),
        )
    };
    let (Some(locale), Some(pack)) = (locale, pack) else {
        set_builder_error(
            builder,
            "BLASPHEM_PACK_INVALID: locale and pack bytes are required".to_owned(),
        );
        return 1;
    };
    match EngineSource::new(
        locale,
        pack.to_vec(),
        pack_digest,
        detect.map(<[u8]>::to_vec),
        detect_digest,
    ) {
        Ok(source) => {
            builder.error = None;
            builder.sources.push(source);
            0
        }
        Err(error) => {
            set_builder_error(builder, error.to_string());
            1
        }
    }
}

/// Builds the engine. On success the builder is consumed and must not be
/// touched again. On failure it returns null and the builder stays alive with
/// the message in `blasphem_builder_error`; the caller frees it with
/// `blasphem_builder_free`.
///
/// # Safety
///
/// `builder` must come from `blasphem_builder_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_builder_build(
    builder: *mut blasphem_builder,
) -> *mut blasphem_engine {
    // SAFETY: the caller promises a live builder pointer.
    let Some(live) = (unsafe { builder.as_mut() }) else {
        set_error("BLASPHEM_PACK_INVALID: null builder".to_owned());
        return ptr::null_mut();
    };
    match Engine::build(&live.sources, live.detect_language, live.grawlix) {
        Ok(engine) => {
            // SAFETY: success hands ownership of the builder to this function.
            drop(unsafe { Box::from_raw(builder) });
            Box::into_raw(Box::new(blasphem_engine { engine }))
        }
        Err(error) => {
            set_builder_error(live, error.to_string());
            ptr::null_mut()
        }
    }
}

/// Frees a builder that was never built, or whose build failed.
///
/// # Safety
///
/// `builder` must come from `blasphem_builder_new`, not yet built successfully,
/// and is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_builder_free(builder: *mut blasphem_builder) {
    if !builder.is_null() {
        // SAFETY: the caller hands over ownership.
        drop(unsafe { Box::from_raw(builder) });
    }
}

/// Scores one message. Never fails; unroutable text is safe. A null engine or
/// text yields the fail-open verdict.
///
/// # Safety
///
/// `engine` must be live and `text` null or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_engine_judge(
    engine: *const blasphem_engine,
    text: *const c_char,
) -> blasphem_judgement {
    let fail_open = blasphem_judgement {
        safe: true,
        score: 0.0,
        locale: ptr::null_mut(),
        grawlix: ptr::null_mut(),
    };
    // SAFETY: the caller promises a live engine.
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return fail_open;
    };
    // SAFETY: the caller promises a NUL-terminated string or null.
    let Some(text) = (unsafe { self::text(text) }) else {
        return fail_open;
    };
    let verdict = engine.engine.judge(text);
    blasphem_judgement {
        safe: verdict.safe,
        score: verdict.score,
        locale: owned_text(verdict.locale),
        grawlix: owned_text(verdict.grawlix),
    }
}

/// Number of loaded locales.
///
/// # Safety
///
/// `engine` must be live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_engine_locale_count(engine: *const blasphem_engine) -> usize {
    // SAFETY: the caller promises a live engine or null.
    unsafe { engine.as_ref() }.map_or(0, |engine| engine.engine.locales().len())
}

/// The lowercase code of the locale at `index`, or null. Free with `blasphem_text_free`.
///
/// # Safety
///
/// `engine` must be live or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_engine_locale(
    engine: *const blasphem_engine,
    index: usize,
) -> *mut c_char {
    // SAFETY: the caller promises a live engine or null.
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return ptr::null_mut();
    };
    owned_text(engine.engine.locales().get(index).cloned())
}

/// Frees the strings inside a judgement.
#[unsafe(no_mangle)]
pub extern "C" fn blasphem_judgement_free(judgement: blasphem_judgement) {
    for pointer in [judgement.locale, judgement.grawlix] {
        if !pointer.is_null() {
            // SAFETY: both pointers came from `CString::into_raw` in this crate.
            drop(unsafe { CString::from_raw(pointer) });
        }
    }
}

/// Frees a string this library returned.
///
/// # Safety
///
/// `text` must come from this library and is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_text_free(text: *mut c_char) {
    if !text.is_null() {
        // SAFETY: the pointer came from `CString::into_raw`.
        drop(unsafe { CString::from_raw(text) });
    }
}

/// Frees an engine.
///
/// # Safety
///
/// `engine` must come from `blasphem_builder_build` and is invalid afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn blasphem_engine_free(engine: *mut blasphem_engine) {
    if !engine.is_null() {
        // SAFETY: the caller hands over ownership.
        drop(unsafe { Box::from_raw(engine) });
    }
}

/// The last error on this thread, or null. Valid until the next failing call.
#[unsafe(no_mangle)]
pub extern "C" fn blasphem_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(ptr::null(), |text| text.as_ptr())
    })
}
