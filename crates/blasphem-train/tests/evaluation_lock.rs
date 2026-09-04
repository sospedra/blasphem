use std::{fs, path::PathBuf};

use blasphem_train::evaluation_lock::parse_evaluation_lock;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed_lock() -> blasphem_train::evaluation_lock::EvaluationLock {
    let path = project_root().join("crates/blasphem-train/metadata/evaluation-lock-v1.json");
    let file = fs::File::open(path).expect("readable evaluation lock");
    parse_evaluation_lock(file).expect("valid evaluation lock")
}

#[test]
fn the_lock_seals_fifteen_languages() {
    let lock = committed_lock();
    assert_eq!(lock.languages.len(), 15);
    assert!(
        lock.languages.contains_key("ID"),
        "Malay seals under its storage code"
    );
    assert!(lock.languages.contains_key("ES"));
}

#[test]
fn a_moved_test_row_fails_verification() {
    use blasphem::Language;
    use blasphem_train::corpus::{parse_corpus, split_digest};
    use blasphem_train::datasets::DatasetSplit;

    let path = project_root().join("corpus/EN.tsv");
    let mut rows = parse_corpus(fs::File::open(&path).expect("English corpus")).unwrap();
    let sealed = split_digest(&rows, DatasetSplit::Test);

    let injected = rows
        .iter()
        .find(|row| row.split == DatasetSplit::Development)
        .cloned()
        .expect("a development row");
    rows.push(blasphem_train::corpus::CorpusRow {
        split: DatasetSplit::Test,
        ..injected
    });

    assert_ne!(
        split_digest(&rows, DatasetSplit::Test),
        sealed,
        "moving a row into the sealed test split must change its digest"
    );
    let _ = Language::En;
}

#[test]
fn the_evaluation_lock_seals_the_committed_corpus_rows() {
    use blasphem::Language;
    use blasphem_train::corpus::{parse_corpus, split_digest};
    use blasphem_train::datasets::DatasetSplit;

    let bytes =
        std::fs::read("../../crates/blasphem-train/metadata/evaluation-lock-v1.json").unwrap();
    let lock = blasphem_train::evaluation_lock::parse_evaluation_lock(bytes.as_slice()).unwrap();

    assert_eq!(lock.languages.len(), 15);
    for (code, sealed) in &lock.languages {
        let path = format!("../../corpus/{code}.tsv");
        let file = std::fs::File::open(&path).unwrap_or_else(|_| panic!("missing {path}"));
        let rows = parse_corpus(file).unwrap();

        assert_eq!(
            split_digest(&rows, DatasetSplit::Validation),
            sealed.validation_sha256
        );
        assert_eq!(split_digest(&rows, DatasetSplit::Test), sealed.test_sha256);
    }
    let _ = Language::En;
}
