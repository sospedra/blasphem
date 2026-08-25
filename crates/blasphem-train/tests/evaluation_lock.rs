use std::{fs, path::PathBuf};

use blasphem_train::evaluation_lock::{
    EvaluationLockError, parse_evaluation_lock, verify_sealed_partitions,
};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed_lock() -> blasphem_train::evaluation_lock::EvaluationLock {
    let path = project_root().join("resources/datasets/evaluation-lock-v1.json");
    let file = fs::File::open(path).expect("readable evaluation lock");
    parse_evaluation_lock(file).expect("valid evaluation lock")
}

#[test]
fn the_lock_seals_fifteen_languages() {
    let lock = committed_lock();
    assert_eq!(lock.languages.len(), 15);
    assert!(lock.languages.contains_key("ID"), "Malay seals under its storage code");
    assert!(lock.languages.contains_key("ES"));
}

#[test]
fn a_moved_test_row_fails_verification() {
    let source = project_root().join("data/prepared-v1");
    if !source.exists() {
        eprintln!("skipped: data/prepared-v1 is derived and not committed");
        return;
    }
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().join("prepared");
    copy_tree(&source, &root);

    let target = root.join("EN/test.tsv");
    let mut text = fs::read_to_string(&target).expect("readable test split");
    text.push_str("EN\tclean\tinjected@0/row/000000\tinjected row\n");
    fs::write(&target, text).expect("writable test split");

    let error = verify_sealed_partitions(&root, &committed_lock())
        .expect_err("a changed sealed file must fail");
    assert!(matches!(error, EvaluationLockError::SealedHashChanged { .. }));
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to).expect("creatable directory");
    for entry in fs::read_dir(from).expect("readable directory") {
        let entry = entry.expect("readable entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copyable file");
        }
    }
}
