use std::{fs, process::Command};

use tempfile::tempdir;

#[test]
fn verification_commands_are_typed_cli_subcommands() {
    let output = blasphem_train_command(&["--help"]);

    assert!(output.status.success(), "{:?}", output.stderr);
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    for command in ["evaluate", "behavior", "cli-smoke"] {
        assert!(stdout.contains(command), "missing {command} in:\n{stdout}");
    }
}

#[test]
fn evaluate_rejects_test_before_it_opens_an_input() {
    let directory = tempdir().expect("temporary directory");
    let report = directory.path().join("test.json");

    let output = blasphem_train_command(&[
        "evaluate",
        "--split",
        "test",
        "--prepared-root",
        "/missing/prepared",
        "--model-manifest",
        "/missing/model.json",
        "--hurtlex-root",
        "/missing/hurtlex",
        "--output",
        report.to_str().expect("UTF-8 path"),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("invalid value 'test'"), "{stderr}");
    assert!(!report.exists());
}

#[test]
fn cli_smoke_writes_canonical_60_case_evidence() {
    let project = project_root();
    let directory = tempdir().expect("temporary directory");
    let report = directory.path().join("nested/smoke.json");
    let model_manifest = project.join("resources/models/multilingual-v2/manifest.json");
    let hurtlex_root = project.join("data/raw-v1/hurtlex");

    let output = blasphem_train_command(&[
        "cli-smoke",
        "--model-manifest",
        model_manifest.to_str().expect("UTF-8 model path"),
        "--hurtlex-root",
        hurtlex_root.to_str().expect("UTF-8 HurtLex path"),
        "--output",
        report.to_str().expect("UTF-8 path"),
    ]);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(&report).expect("smoke evidence");
    let evidence = blasphem_train::evidence::parse_canonical_json::<
        blasphem_train::verification::CliSmokeEvidence,
    >(&bytes)
    .expect("canonical smoke evidence");
    assert!(evidence.passed());
    assert_eq!(
        evidence
            .languages
            .values()
            .map(|language| language.cases.len())
            .sum::<usize>(),
        60,
    );
}

fn blasphem_train_command(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(arguments)
        .output()
        .expect("run blasphem-train")
}

fn project_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("project root")
}
