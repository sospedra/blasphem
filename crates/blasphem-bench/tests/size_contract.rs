use std::{fs, path::Path};

use toxbench::{
    ARTIFACT_SIZE_LIMIT_BYTES, BINARY_SIZE_LIMIT_BYTES, check_artifact_size, check_binary_size,
    collect_size_evidence, record_file,
};

#[test]
fn shipping_size_gates_use_the_exact_limits() {
    assert!(check_binary_size(BINARY_SIZE_LIMIT_BYTES).is_ok());
    assert!(check_binary_size(BINARY_SIZE_LIMIT_BYTES + 1).is_err());
    assert!(check_artifact_size(ARTIFACT_SIZE_LIMIT_BYTES - 1).is_ok());
    assert!(check_artifact_size(ARTIFACT_SIZE_LIMIT_BYTES).is_err());
}

#[test]
fn size_evidence_contains_every_language_resource() {
    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir().expect("temporary directory");
    let binary = temporary.path().join("toxcheck");
    fs::write(&binary, b"shipping binary fixture").expect("binary fixture");

    let evidence = collect_size_evidence(
        &binary,
        &project_root.join("resources/models/multilingual-v2/manifest.json"),
        &project_root.join("data/raw-v1/hurtlex"),
        "aarch64-apple-darwin",
    )
    .expect("size evidence");

    assert_eq!(evidence.artifacts.len(), 15);
    assert_eq!(evidence.hurtlex.len(), 15);
    assert!(evidence.all_gates_passed);
}

#[test]
fn file_record_rejects_missing_and_digest_mismatched_files() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let path = temporary.path().join("artifact.bin");

    assert!(record_file(&path, "artifact.bin", None, None).is_err());
    fs::write(&path, b"abc").expect("artifact fixture");
    assert!(
        record_file(
            &path,
            "artifact.bin",
            Some("0000000000000000000000000000000000000000000000000000000000000000"),
            Some(3),
        )
        .is_err()
    );
}
