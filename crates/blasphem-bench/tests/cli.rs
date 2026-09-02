use std::{fs, path::Path, process::Command};

use toxbench::SizeEvidence;

#[test]
fn size_command_writes_canonical_experimental_evidence() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir().expect("temporary directory");
    let binary = temporary.path().join("toxcheck");
    let output = temporary.path().join("size.json");
    fs::write(&binary, b"shipping binary fixture").expect("binary fixture");

    let status = Command::new(env!("CARGO_BIN_EXE_toxbench"))
        .arg("size")
        .arg("--binary")
        .arg(&binary)
        .arg("--model-manifest")
        .arg(project_root.join("resources/models/multilingual-v2/manifest.json"))
        .arg("--hurtlex-root")
        .arg(project_root.join("data/raw-v1/hurtlex"))
        .arg("--target-triple")
        .arg("aarch64-apple-darwin")
        .arg("--output")
        .arg(&output)
        .status()
        .expect("run size command");

    assert!(status.success());
    let bytes = fs::read(&output).expect("size evidence");
    assert_ne!(bytes.last(), Some(&b'\n'));
    let evidence: SizeEvidence = serde_json::from_slice(&bytes).expect("valid size evidence");
    assert_eq!(evidence.evidence_status, "experimental");
    assert_eq!(evidence.artifacts.len(), 15);
    assert_eq!(evidence.hurtlex.len(), 15);
}

#[test]
fn auto_command_exposes_the_reproducible_evidence_inputs() {
    let output = Command::new(env!("CARGO_BIN_EXE_toxbench"))
        .arg("auto")
        .arg("--help")
        .output()
        .expect("run auto help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    for option in [
        "--texts",
        "--labels",
        "--fixtures",
        "--hurtlex-root",
        "--model-manifest",
        "--native-binary",
        "--eldc-artifact",
        "--browser-report",
        "--project-root",
        "--output",
    ] {
        assert!(stdout.contains(option), "missing {option} in {stdout}");
    }
}
