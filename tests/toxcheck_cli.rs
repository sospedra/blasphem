use std::process::{Command, Output};

use tempfile::tempdir;

fn run(language: &str, text: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["check", "--language", language, "--text", text])
        .output()
        .expect("run toxcheck")
}

fn stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

#[test]
fn check_requires_one_supported_language() {
    for arguments in [
        vec!["check", "--text", "hello"],
        vec!["check", "--language", "all", "--text", "hello"],
        vec!["check", "--language", "EN,ES", "--text", "hello"],
        vec!["check", "--language", "XX", "--text", "hello"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
            .args(&arguments)
            .output()
            .expect("run toxcheck");

        assert!(!output.status.success(), "accepted {arguments:?}");
    }
}

#[test]
fn check_help_exposes_only_the_product_resource_controls() {
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .args(["check", "--help"])
        .output()
        .expect("run toxcheck help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("--language <LANGUAGE>"));
    assert!(!stdout.contains("default: auto"));
    assert!(stdout.contains("default: data/raw-v1/hurtlex"));
    assert!(!stdout.contains("--include-inclusive"));
}

#[test]
fn check_prints_only_stable_product_fields_on_the_first_line() {
    let stdout = stdout(run("ES", "Te voy a matar"));

    assert_eq!(
        stdout.lines().next(),
        Some("ok=false score=95 threshold=50 should_nudge=true"),
    );
}

#[test]
fn check_rejects_a_missing_required_hurtlex_file() {
    let directory = tempdir().expect("temporary directory");
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .args([
            "check",
            "--data-dir",
            directory.path().to_str().expect("UTF-8 path"),
            "--language",
            "EN",
            "--text",
            "hello",
        ])
        .output()
        .expect("run toxcheck");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("EN"));
    assert!(stderr.contains("EN/1.2/hurtlex_EN.tsv"));
}

#[test]
fn check_prints_the_prediction_and_match() {
    let stdout = stdout(run("EN", "I will kill you"));

    assert_eq!(
        stdout.lines().next(),
        Some("ok=false score=95 threshold=50 should_nudge=true"),
    );
    assert!(stdout.contains("lexical_score="));
    assert!(stdout.contains("category=threat_language points=95"));
    assert!(stdout.contains("rule=direct_threat"));
}

#[test]
fn check_prints_the_sparse_channel_status() {
    let stdout = stdout(run("ES", "La gente del pueblo habló con la policía."));

    assert!(stdout.contains("sparse_score=15"));
}

#[test]
fn explicit_supported_language_uses_the_nested_hurtlex_file() {
    let stdout = stdout(run("ZH", "普通消息"));

    assert!(stdout.starts_with("ok=true"));
}

#[test]
fn spanish_check_nudges_harm_and_preserves_safe_context() {
    for (text, expected_line) in [
        (
            "Te voy a matar",
            "ok=false score=95 threshold=50 should_nudge=true",
        ),
        (
            "ojala se muera toda tu familia",
            "ok=false score=85 threshold=50 should_nudge=true",
        ),
        (
            "No te voy a matar",
            "ok=true score=24 threshold=50 should_nudge=false",
        ),
        (
            "Te voy a matar de risa",
            "ok=true score=24 threshold=50 should_nudge=false",
        ),
    ] {
        let stdout = stdout(run("ES", text));
        assert_eq!(stdout.lines().next(), Some(expected_line), "{text}");
    }
}

#[test]
fn check_blocks_a_direct_threat_without_a_lexical_match() {
    let stdout = stdout(run("EN", "I will kill you"));

    assert_eq!(
        stdout.lines().next(),
        Some("ok=false score=95 threshold=50 should_nudge=true"),
    );
    assert!(stdout.contains("lexical_score=0.000"));
}

#[test]
fn check_rejects_language_lists_and_all() {
    for language in ["EN,ES", "all"] {
        let output = run(language, "hello");

        assert!(!output.status.success(), "accepted {language}");
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
        assert!(stderr.contains("unsupported language"));
    }
}

#[test]
fn check_escapes_control_characters_in_one_line_fields() {
    let stdout = stdout(run("EN", "hello\nworld"));

    assert!(!stdout.contains("normalized=\"hello\nworld\""));
    assert!(stdout.contains("normalized=\"hello world\""));
}

#[test]
#[cfg(feature = "language-detection")]
fn check_accepts_uppercase_and_lowercase_auto() {
    for language in ["AUTO", "auto"] {
        let stdout = stdout(run(language, "I never should've bought that."));
        assert_known_auto_output(
            &stdout,
            "ok=true score=42 threshold=50 should_nudge=false",
            "EN",
        );
    }
}

#[cfg(not(feature = "language-detection"))]
#[test]
fn automatic_cli_requires_the_optional_language_detection_feature() {
    let output = run("AUTO", "I never should've bought that.");

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).expect("UTF-8 error"),
        "Error: AUTO requires the language-detection feature\n",
    );
}

#[test]
#[cfg(feature = "language-detection")]
fn automatic_unknown_skips_hurtlex_when_the_data_directory_is_missing() {
    let directory = tempdir().expect("temporary directory");
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .args([
            "check",
            "--data-dir",
            directory.path().to_str().expect("UTF-8 path"),
            "--language",
            "AUTO",
            "--text",
            "!@#$%^&*()",
        ])
        .output()
        .expect("run toxcheck");
    let stdout = stdout(output);

    assert_eq!(
        stdout,
        "ok=true score=0 threshold=50 should_nudge=false\nlanguage_mode=auto route=unknown detected_language=unknown reliable=false language_score=none evaluated=false\n"
    );
}

#[test]
fn explicit_ms_and_id_have_the_same_decision_and_canonical_route() {
    let ms = stdout(run("MS", "aku akan membunuhmu"));
    let id = stdout(run("ID", "aku akan membunuhmu"));

    assert_eq!(ms.lines().next(), id.lines().next());
    assert_eq!(
        ms.lines().nth(1),
        Some(
            "language_mode=explicit route=known detected_language=MS reliable=true language_score=none evaluated=true"
        )
    );
    assert_eq!(ms.lines().nth(1), id.lines().nth(1));
}

#[test]
fn check_rejects_unknown_languages_before_resource_access() {
    let directory = tempdir().expect("temporary directory");
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .args([
            "check",
            "--data-dir",
            directory.path().to_str().expect("UTF-8 path"),
            "--language",
            "XX",
            "--text",
            "hello",
        ])
        .output()
        .expect("run toxcheck");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("unsupported language"));
    assert!(!stderr.contains("cannot read required"));
}

#[test]
fn check_uses_the_typed_reply_target() {
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "check",
            "--language",
            "EN",
            "--reply-target",
            "person",
            "--text",
            "moron",
        ])
        .output()
        .expect("run toxcheck");
    let stdout = stdout(output);

    assert!(stdout.lines().next().is_some_and(|line| {
        line.starts_with("ok=false score=") && line.ends_with("threshold=50 should_nudge=true")
    }));
    assert!(stdout.contains("rule=semantic_directed_hostility"));
}

#[cfg(feature = "language-detection")]
fn assert_known_auto_output(stdout: &str, expected_first_line: &str, language: &str) {
    assert_eq!(stdout.lines().next(), Some(expected_first_line));

    let route = stdout.lines().nth(1).expect("routing line");
    let prefix = format!(
        "language_mode=auto route=known detected_language={language} reliable=true language_score="
    );
    let score = route
        .strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix(" evaluated=true"))
        .expect("exact automatic known routing fields");
    let (whole, fractional) = score.split_once('.').expect("decimal language score");

    assert!(!whole.is_empty() && whole.chars().all(|character| character.is_ascii_digit()));
    assert_eq!(
        fractional.len(),
        4,
        "language score requires four decimal digits"
    );
    assert!(
        fractional
            .chars()
            .all(|character| character.is_ascii_digit()),
        "language score requires decimal digits"
    );
}
