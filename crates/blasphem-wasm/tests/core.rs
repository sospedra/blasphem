//! Exercises the engine the wasm classes wrap, with packs built from the
//! repository's committed artifacts.

use std::path::PathBuf;

use blasphem::{Engine, EngineSource, Language, PackInput};
use sha2::Digest as _;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> Vec<u8> {
    let path = root().join(relative);
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn rule_pack_version(language: Language) -> u16 {
    let manifest: serde_json::Value =
        serde_json::from_slice(&read("resources/models/multilingual-v2/manifest.json"))
            .expect("valid model manifest");
    let entry = manifest["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["language"] == language.code())
        .expect("manifest entry");
    u16::try_from(entry["rule_pack_version"].as_u64().expect("version")).expect("u16")
}

fn pack(language: Language) -> Vec<u8> {
    let storage = language.storage_code();
    let artifact_name = if language == Language::Es {
        "es-chargram-v1.bin".to_owned()
    } else {
        format!("{}-sparse-v2.bin", storage.to_ascii_lowercase())
    };
    let artifact = read(&format!("resources/models/multilingual-v2/{artifact_name}"));
    let lexicon = read(&format!(
        "data/raw-v1/hurtlex/{storage}/1.2/hurtlex_{storage}.tsv"
    ));
    blasphem::encode_pack(&PackInput {
        language,
        rule_pack_version: rule_pack_version(language),
        artifact: &artifact,
        lexicon: &lexicon,
    })
}

fn detect(language: Language) -> Vec<u8> {
    let model = read("crates/blasphem-language/data/blasphem-language-15-v2.bin");
    let code = language.code().to_ascii_lowercase();
    blasphem_language::slice::write_slices(&model)
        .expect("slices")
        .into_iter()
        .find(|(slice_language, _)| slice_language.code() == code)
        .map(|(_, bytes)| bytes)
        .expect("slice for language")
}

fn hex(bytes: &[u8]) -> String {
    format!("{:x}", sha2::Sha256::digest(bytes))
}

fn source(language: Language, with_detect: bool) -> EngineSource {
    let pack = pack(language);
    let detect = with_detect.then(|| detect(language));
    EngineSource::new(
        &language.code().to_ascii_lowercase(),
        pack.clone(),
        Some(&hex(&pack)),
        detect.clone(),
        detect.as_deref().map(hex).as_deref(),
    )
    .expect("valid source")
}

#[test]
fn engine_scores_english_and_masks_on_request() {
    let engine = Engine::build(
        &[source(Language::En, true), source(Language::Es, true)],
        true,
        true,
    )
    .expect("engine builds");
    let verdict = engine.judge("you are a stupid loser");

    assert_eq!(engine.locales(), vec!["en".to_owned(), "es".to_owned()]);
    assert!(!verdict.safe);
    assert_eq!(verdict.score, 0.64);
    assert_eq!(verdict.locale.as_deref(), Some("en"));
    assert_eq!(verdict.grawlix.as_deref(), Some("you are a @#$%&! loser"));
}

#[test]
fn engine_omits_grawlix_when_not_requested_and_accepts_the_id_alias() {
    let engine = Engine::build(&[source(Language::Ms, false)], false, false).expect("engine");
    let aliased = EngineSource::new("id", pack(Language::Ms), None, None, None).expect("alias");
    let alias_engine = Engine::build(&[aliased], false, false).expect("alias engine");
    let text = "Dia memberitahu saya yang dia benar-benar letih.";

    assert_eq!(engine.locales(), vec!["ms".to_owned()]);
    assert_eq!(alias_engine.locales(), vec!["ms".to_owned()]);
    assert_eq!(engine.judge(text), alias_engine.judge(text));
    assert_eq!(engine.judge(text).grawlix, None);
}

#[test]
fn engine_fails_open_for_text_no_loaded_locale_routes() {
    let engine = Engine::build(&[source(Language::En, true)], true, false).expect("engine");

    for text in ["물이 별로 없다.", "", "!@#$%^&*()"] {
        let verdict = engine.judge(text);
        assert!(verdict.safe, "{text:?}");
        assert_eq!(verdict.score, 0.0, "{text:?}");
        assert_eq!(verdict.locale, None, "{text:?}");
    }
}

#[test]
fn engine_source_reports_an_unknown_locale_with_its_code() {
    let error = EngineSource::new("xx", pack(Language::En), None, None, None).expect_err("xx");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_LOCALE_UNSUPPORTED: unsupported locale \"xx\""
    );
}

#[test]
fn engine_source_rejects_a_short_digest_string() {
    let error =
        EngineSource::new("en", pack(Language::En), Some("abc"), None, None).expect_err("digest");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_PACK_INVALID: en.pack digest is not 64 hexadecimal characters"
    );
}

#[test]
fn engine_reports_a_digest_mismatch_with_its_code() {
    let zeroes = "0".repeat(64);
    let source =
        EngineSource::new("en", pack(Language::En), Some(&zeroes), None, None).expect("source");
    let error = Engine::build(&[source], false, false).expect_err("mismatch");

    assert!(
        error
            .to_string()
            .starts_with("BLASPHEM_DIGEST_MISMATCH: en.pack expected sha256 0000"),
        "got {error}"
    );
}

#[test]
fn engine_requires_a_detect_slice_when_detection_is_on() {
    let error = Engine::build(&[source(Language::En, false)], true, false).expect_err("no slice");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_PACK_INVALID: en.detect is required when language detection is on"
    );
}

#[test]
fn engine_rejects_no_sources() {
    let error = Engine::build(&[], false, false).expect_err("empty");

    assert_eq!(
        error.to_string(),
        "BLASPHEM_LOCALES_EMPTY: no locale was given"
    );
}
