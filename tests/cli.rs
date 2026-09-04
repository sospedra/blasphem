use std::io::Write;
use std::process::{Command, Output, Stdio};

use tempfile::tempdir;

fn run(language: &str, text: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_blasphem"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["check", "--language", language, "--text", text])
        .output()
        .expect("run blasphem")
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
        let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
            .args(&arguments)
            .output()
            .expect("run blasphem");

        assert!(!output.status.success(), "accepted {arguments:?}");
    }
}

#[test]
fn check_help_exposes_only_the_product_resource_controls() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
        .args(["check", "--help"])
        .output()
        .expect("run blasphem help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("--language <LANGUAGE>"));
    assert!(!stdout.contains("default: auto"));
    assert!(stdout.contains("default: data/clean-room-v1"));
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
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
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
        .expect("run blasphem");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("EN"));
    assert!(stderr.contains("EN.tsv"));
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

    assert!(stdout.contains("sparse_score=20"));
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
            "ok=true score=30 threshold=50 should_nudge=false",
        ),
        (
            "Te voy a matar de risa",
            "ok=true score=30 threshold=50 should_nudge=false",
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
            "ok=true score=48 threshold=50 should_nudge=false",
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
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
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
        .expect("run blasphem");
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
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
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
        .expect("run blasphem");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("unsupported language"));
    assert!(!stderr.contains("cannot read required"));
}

#[test]
fn check_uses_the_typed_reply_target() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
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
        .expect("run blasphem");
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

fn judge(arguments: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_blasphem"));
    command.arg("judge").args(arguments);
    let Some(text) = stdin else {
        return command
            .stdin(Stdio::null())
            .output()
            .expect("run blasphem judge");
    };
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn blasphem judge");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(text.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("run blasphem judge")
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn judge_prints_a_safe_verdict_and_exits_zero() {
    let output = judge(
        &[
            "--no-detect",
            "--locales",
            "en",
            "I hope you have a wonderful day",
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(0), "{}", lossy(&output.stderr));
    assert_eq!(lossy(&output.stdout), "safe=true score=0.42 locale=en\n");
}

#[test]
fn judge_exits_one_for_a_nudge() {
    let output = judge(&["--no-detect", "--locales", "en", "I will kill you"], None);

    assert_eq!(output.status.code(), Some(1), "{}", lossy(&output.stderr));
    assert_eq!(lossy(&output.stdout), "safe=false score=0.95 locale=en\n");
}

#[test]
fn judge_json_matches_the_javascript_contract() {
    let output = judge(
        &["--json", "--no-detect", "--locales", "es", "Te voy a matar"],
        None,
    );

    assert_eq!(output.status.code(), Some(1), "{}", lossy(&output.stderr));
    assert_eq!(
        lossy(&output.stdout),
        "{\"safe\":false,\"score\":0.95,\"locale\":\"es\",\"grawlix\":null}\n"
    );
}

#[test]
fn judge_masks_the_text_when_asked() {
    let output = judge(
        &[
            "--grawlix",
            "--no-detect",
            "--locales",
            "en",
            "you are an idiot",
        ],
        None,
    );

    assert_eq!(output.status.code(), Some(1), "{}", lossy(&output.stderr));
    let stdout = lossy(&output.stdout);
    assert!(stdout.contains(" grawlix=\""), "{stdout}");
    assert!(!stdout.contains("idiot"), "{stdout}");
}

#[test]
fn judge_reads_one_message_per_stdin_line() {
    let output = judge(&["--locales", "es"], Some("hello\nTe voy a matar\n"));

    assert_eq!(output.status.code(), Some(1), "{}", lossy(&output.stderr));
    let stdout = lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].starts_with("safe=true "), "{stdout}");
    assert_eq!(lines[1], "safe=false score=0.95 locale=es");
}

#[test]
fn judge_with_empty_stdin_prints_nothing_and_exits_zero() {
    let output = judge(&["--locales", "en"], None);

    assert_eq!(output.status.code(), Some(0), "{}", lossy(&output.stderr));
    assert_eq!(lossy(&output.stdout), "");
}

#[test]
fn judge_rejects_an_unknown_locale() {
    let output = judge(&["--locales", "en,xx", "hello"], None);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        lossy(&output.stderr).contains("xx"),
        "{}",
        lossy(&output.stderr)
    );
}

#[test]
fn top_level_help_shows_judge_and_hides_check() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
        .arg("--help")
        .output()
        .expect("run blasphem --help");

    assert!(output.status.success());
    let stdout = lossy(&output.stdout);
    assert!(stdout.contains("judge"), "{stdout}");
    assert!(!stdout.contains("check"), "{stdout}");
}
