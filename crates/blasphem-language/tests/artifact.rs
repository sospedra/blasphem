use std::{fs, path::PathBuf, process::Command};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_committed_artifact_rebuilds_from_the_vendored_tables() {
    let root = project_root();
    let vendor =
        root.join("crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8");
    let output = tempfile::NamedTempFile::new().expect("temporary output");
    let status = Command::new(env!("CARGO_BIN_EXE_blasphem-language-model"))
        .arg(&vendor)
        .arg(output.path())
        .status()
        .expect("runs the language model builder");
    assert!(status.success(), "the builder must succeed");

    let rebuilt = fs::read(output.path()).expect("readable rebuild");
    let committed =
        fs::read(root.join("crates/blasphem-language/data/blasphem-language-15-v2.bin"))
            .expect("readable committed artifact");
    assert_eq!(
        rebuilt, committed,
        "the rebuild must match the committed artifact"
    );
}
