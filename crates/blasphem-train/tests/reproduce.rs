use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use blasphem_train::reproduce::REENTRY_GUARD_VARIABLE;

const COPIED_ENTRIES: [&str; 10] = [
    "Cargo.toml",
    "Cargo.lock",
    "src",
    "crates",
    "corpus",
    "data/raw-v1",
    "data/hurtlex",
    "resources",
    "tests",
    "samples",
];

#[test]
fn reproduce_rejects_one_changed_raw_byte() {
    if std::env::var_os(REENTRY_GUARD_VARIABLE).is_some() {
        return;
    }
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().join("clone");
    copy_project(&root);

    let target = root.join("data/raw-v1/hurtlex/EN/1.2/hurtlex_EN.tsv");
    let mut bytes = std::fs::read(&target).expect("readable raw source");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x20;
    std::fs::write(&target, bytes).expect("writable raw source");

    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .current_dir(&root)
        .args(["reproduce", "--skip-browser"])
        .output()
        .expect("runs reproduce");
    assert!(!output.status.success(), "a changed raw byte must fail");
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("hurtlex/EN/1.2/hurtlex_EN.tsv"),
        "{message}"
    );
}

fn copy_project(root: &Path) {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for entry in COPIED_ENTRIES {
        copy_entry(&source.join(entry), &root.join(entry));
    }
}

fn copy_entry(source: &Path, destination: &Path) {
    let metadata = fs::symlink_metadata(source).expect("readable project entry");
    let parent = destination.parent().expect("destination has a parent");
    fs::create_dir_all(parent).expect("creatable destination parent");
    if metadata.is_dir() {
        copy_directory(source, destination);
        return;
    }
    fs::copy(source, destination).expect("copyable project file");
}

fn copy_directory(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("creatable destination directory");
    for entry in fs::read_dir(source).expect("readable project directory") {
        let entry = entry.expect("readable directory entry");
        let name = entry.file_name();
        if name == "target" || name == ".git" {
            continue;
        }
        copy_entry(&entry.path(), &destination.join(name));
    }
}

#[test]
fn reproduction_verifies_the_corpus_instead_of_generating_it() {
    assert_eq!(blasphem_train::reproduce::STEP_NAMES.len(), 8);
    assert_eq!(blasphem_train::reproduce::STEP_NAMES[0], "verify-corpus");
    assert!(!blasphem_train::reproduce::STEP_NAMES.contains(&"generate-prepared-data"));
    assert_eq!(blasphem_train::reproduce::GENERATION_STEPS, 4);
}
