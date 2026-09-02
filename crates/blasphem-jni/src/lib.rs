//! A JNI layer over the bytes-in engine for the Android library in
//! `packages/android`. `me.sospedra.blasphem.Native` declares every method.
//!
//! Ownership: Kotlin holds a builder as a `Long` until `builderBuild` consumes
//! it or `builderFree` drops it, and an engine as a `Long` until `engineFree`.
//! Every failure throws `java.lang.RuntimeException` carrying the engine's
//! `CODE: detail` text, which Kotlin turns into `BlasphemException`.

use std::ptr;

use blasphem::{Engine, EngineJudgement, EngineSource};
use jni::JNIEnv;
use jni::errors::Result as JniResult;
use jni::objects::{JByteArray, JObject, JObjectArray, JString, JValue};
use jni::sys::{JNI_TRUE, jboolean, jlong, jobject, jobjectArray, jsize};

const JUDGEMENT_CLASS: &str = "me/sospedra/blasphem/Judgement";
const JUDGEMENT_CONSTRUCTOR: &str = "(ZDLjava/lang/String;Ljava/lang/String;)V";
const RUNTIME_EXCEPTION: &str = "java/lang/RuntimeException";

struct Builder {
    sources: Vec<EngineSource>,
    detect_language: bool,
    grawlix: bool,
}

fn throw(env: &mut JNIEnv<'_>, message: &str) {
    if env.exception_check().unwrap_or(false) {
        return;
    }
    let _ = env.throw_new(RUNTIME_EXCEPTION, message);
}

/// # Safety
///
/// `handle` must be zero or an address from `Box::into_raw` on a `T` that is
/// still owned by the Kotlin side.
unsafe fn borrow<'a, T>(handle: jlong) -> Option<&'a mut T> {
    // SAFETY: the caller promises a live address or zero.
    unsafe { (handle as *mut T).as_mut() }
}

fn read_string(env: &mut JNIEnv<'_>, value: &JString<'_>) -> Option<String> {
    if value.is_null() {
        return None;
    }
    env.get_string(value).ok().map(Into::into)
}

fn read_bytes(env: &mut JNIEnv<'_>, value: &JByteArray<'_>) -> Option<Vec<u8>> {
    if value.is_null() {
        return None;
    }
    env.convert_byte_array(value).ok()
}

fn optional_string<'local>(
    env: &mut JNIEnv<'local>,
    value: Option<&str>,
) -> JniResult<JObject<'local>> {
    match value {
        Some(text) => env.new_string(text).map(JObject::from),
        None => Ok(JObject::null()),
    }
}

fn string_array<'local>(
    env: &mut JNIEnv<'local>,
    values: &[String],
) -> JniResult<JObjectArray<'local>> {
    let array = env.new_object_array(values.len() as jsize, "java/lang/String", JObject::null())?;
    for (index, value) in values.iter().enumerate() {
        let text = env.new_string(value)?;
        env.set_object_array_element(&array, index as jsize, text)?;
    }
    Ok(array)
}

fn judgement<'local>(
    env: &mut JNIEnv<'local>,
    verdict: &EngineJudgement,
) -> JniResult<JObject<'local>> {
    let locale = optional_string(env, verdict.locale.as_deref())?;
    let grawlix = optional_string(env, verdict.grawlix.as_deref())?;
    env.new_object(
        JUDGEMENT_CLASS,
        JUDGEMENT_CONSTRUCTOR,
        &[
            JValue::Bool(u8::from(verdict.safe)),
            JValue::Double(verdict.score),
            JValue::Object(&locale),
            JValue::Object(&grawlix),
        ],
    )
}

/// Boxes a builder and returns its address. Never fails.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_builderNew<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    detect_language: jboolean,
    grawlix: jboolean,
) -> jlong {
    Box::into_raw(Box::new(Builder {
        sources: Vec::new(),
        detect_language: detect_language == JNI_TRUE,
        grawlix: grawlix == JNI_TRUE,
    })) as jlong
}

/// Adds one locale from its pack bytes and optional detect bytes. Digests are
/// not checked: the package manager aligned the versions.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_builderAdd<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    builder: jlong,
    locale: JString<'local>,
    pack: JByteArray<'local>,
    detect: JByteArray<'local>,
) {
    // SAFETY: Kotlin passes the address `builderNew` returned and has not consumed.
    let Some(builder) = (unsafe { borrow::<Builder>(builder) }) else {
        throw(&mut env, "BLASPHEM_PACK_INVALID: null builder");
        return;
    };
    let (Some(locale), Some(pack)) = (read_string(&mut env, &locale), read_bytes(&mut env, &pack))
    else {
        throw(
            &mut env,
            "BLASPHEM_PACK_INVALID: locale and pack bytes are required",
        );
        return;
    };
    let detect = read_bytes(&mut env, &detect);
    match EngineSource::new(&locale, pack, None, detect, None) {
        Ok(source) => builder.sources.push(source),
        Err(error) => throw(&mut env, &error.to_string()),
    }
}

/// Builds the engine. Success consumes the builder and returns the engine's
/// address. Failure throws, returns zero, and keeps the builder alive for
/// `builderFree`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_builderBuild<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    builder: jlong,
) -> jlong {
    // SAFETY: Kotlin passes the address `builderNew` returned and has not consumed.
    let Some(live) = (unsafe { borrow::<Builder>(builder) }) else {
        throw(&mut env, "BLASPHEM_PACK_INVALID: null builder");
        return 0;
    };
    match Engine::build(&live.sources, live.detect_language, live.grawlix) {
        Ok(engine) => {
            // SAFETY: success hands ownership of the builder to this function.
            drop(unsafe { Box::from_raw(builder as *mut Builder) });
            Box::into_raw(Box::new(engine)) as jlong
        }
        Err(error) => {
            throw(&mut env, &error.to_string());
            0
        }
    }
}

/// Drops a builder that was never built, or whose build failed.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_builderFree<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    builder: jlong,
) {
    if builder == 0 {
        return;
    }
    // SAFETY: Kotlin hands over the address `builderNew` returned.
    drop(unsafe { Box::from_raw(builder as *mut Builder) });
}

/// The loaded locales as lowercase codes, in registry order.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_engineLocales<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    engine: jlong,
) -> jobjectArray {
    // SAFETY: Kotlin passes the address `builderBuild` returned and has not freed.
    let Some(engine) = (unsafe { borrow::<Engine>(engine) }) else {
        throw(&mut env, "BLASPHEM_CLOSED: the judge was closed");
        return ptr::null_mut();
    };
    match string_array(&mut env, &engine.locales()) {
        Ok(array) => array.into_raw(),
        Err(error) => {
            throw(&mut env, &format!("BLASPHEM_PACK_INVALID: {error}"));
            ptr::null_mut()
        }
    }
}

/// Scores one message and returns a `me.sospedra.blasphem.Judgement`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_engineJudge<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    engine: jlong,
    text: JString<'local>,
) -> jobject {
    // SAFETY: Kotlin passes the address `builderBuild` returned and has not freed.
    let Some(engine) = (unsafe { borrow::<Engine>(engine) }) else {
        throw(&mut env, "BLASPHEM_CLOSED: the judge was closed");
        return ptr::null_mut();
    };
    let Some(text) = read_string(&mut env, &text) else {
        throw(&mut env, "BLASPHEM_PACK_INVALID: text is required");
        return ptr::null_mut();
    };
    let verdict = engine.judge(&text);
    match judgement(&mut env, &verdict) {
        Ok(object) => object.into_raw(),
        Err(error) => {
            throw(&mut env, &format!("BLASPHEM_PACK_INVALID: {error}"));
            ptr::null_mut()
        }
    }
}

/// Drops an engine.
#[unsafe(no_mangle)]
pub extern "system" fn Java_me_sospedra_blasphem_Native_engineFree<'local>(
    _env: JNIEnv<'local>,
    _this: JObject<'local>,
    engine: jlong,
) {
    if engine == 0 {
        return;
    }
    // SAFETY: Kotlin hands over the address `builderBuild` returned.
    drop(unsafe { Box::from_raw(engine as *mut Engine) });
}
