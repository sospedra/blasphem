use std::{collections::BTreeMap, fs, path::Path};

use blasphem::{EvalLabel, Language};
use blasphem_train::{
    datasets::{DatasetId, LineageStatus, PreparedCounts, PreparedFileIdentity, PreparedManifest},
    evidence::Sha256Digest,
    model_manifest::ModelSetError,
    prepared_input::{load_prepared_language, parse_prepared_manifest},
    publication::PREPARED_MANIFEST_SCHEMA_VERSION,
    source_manifest::SourceRecord,
    source_role::SourceRole,
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HEADER: &str = "detector_language\tlabel\tsource_id\ttext\n";

#[test]
fn prepared_manifest_rejects_a_missing_or_unknown_schema() {
    let missing = parse_prepared_manifest(b"{}".as_slice()).expect_err("missing schema");
    assert!(matches!(missing, ModelSetError::PreparedManifestJson(_)));

    let mut manifest = fixture_manifest();
    manifest.schema_version = "prepared-v0".to_owned();
    let bytes = serde_json::to_vec(&manifest).expect("serialize fixture");
    let unknown = parse_prepared_manifest(bytes.as_slice()).expect_err("unknown schema");
    assert!(matches!(
        unknown,
        ModelSetError::InvalidPreparedManifestSchema { actual }
            if actual == "prepared-v0"
    ));
}

#[test]
fn prepared_manifest_rejects_an_unknown_root_field() {
    let mut value = serde_json::to_value(fixture_manifest()).expect("serialize fixture");
    value
        .as_object_mut()
        .expect("manifest object")
        .insert("extra".to_owned(), serde_json::Value::Bool(true));
    let bytes = serde_json::to_vec(&value).expect("serialize changed fixture");

    let error = parse_prepared_manifest(bytes.as_slice()).expect_err("unknown field");

    assert!(matches!(error, ModelSetError::PreparedManifestJson(_)));
}

#[test]
fn prepared_input_rejects_every_noncanonical_tsv_header() {
    let cases = [
        "detector_language\tlabel\tsource_id\ttext\textra\n",
        "detector_language\tlabel\tsource_id\n",
        "label\tdetector_language\tsource_id\ttext\n",
        "detector_language\tlabel\tlabel\ttext\n",
    ];
    for header in cases {
        let directory = tempdir().expect("temporary directory");
        let root = directory.path().join("prepared-v1");
        let mut manifest = write_fixture(&root);
        write_target_split(
            &root,
            &mut manifest,
            "development",
            concat!(
                "EN\tclean\tdevelopment/clean\tclean message\n",
                "EN\ttoxic\tdevelopment/toxic\ttoxic message\n",
            ),
            header,
        );
        write_manifest(&root, &manifest);

        let error =
            load_prepared_language(&root, Language::En).expect_err("noncanonical prepared header");

        assert!(matches!(
            error,
            ModelSetError::PreparedHeaderMismatch { .. }
        ));
    }
}

#[test]
fn prepared_input_rejects_duplicate_source_ids_across_splits() {
    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("prepared-v1");
    let mut manifest = write_fixture(&root);
    write_target_split(
        &root,
        &mut manifest,
        "validation",
        concat!(
            "EN\tclean\tdevelopment/clean\tclean validation\n",
            "EN\ttoxic\tvalidation/toxic\ttoxic validation\n",
        ),
        HEADER,
    );
    write_manifest(&root, &manifest);

    let error = load_prepared_language(&root, Language::En)
        .expect_err("duplicate prepared source identifier");

    assert!(matches!(
        error,
        ModelSetError::DuplicatePreparedSourceId(source_id)
            if source_id == "development/clean"
    ));
}

#[test]
fn prepared_input_rejects_duplicate_language_source_identifiers() {
    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("prepared-v1");
    let mut manifest = write_fixture(&root);
    manifest
        .language_sources
        .get_mut("EN")
        .expect("English sources")
        .push("source-en".to_owned());
    write_manifest(&root, &manifest);

    let error = load_prepared_language(&root, Language::En)
        .expect_err("duplicate language source identifier");

    assert!(matches!(
        error,
        ModelSetError::DuplicateLanguageSourceId {
            language: Language::En,
            source_id,
        } if source_id == "source-en"
    ));
}

#[test]
fn prepared_input_rejects_duplicate_identities_in_manifest_sources() {
    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("prepared-v1");
    let mut manifest = write_fixture(&root);
    manifest.sources.push(manifest.sources[0].clone());
    write_manifest(&root, &manifest);

    let error = load_prepared_language(&root, Language::En)
        .expect_err("duplicate prepared source identity");

    assert!(matches!(
        error,
        ModelSetError::DuplicateSourceRecord(source_id) if source_id == "source-en"
    ));
}

#[test]
fn prepared_input_rejects_unknown_and_wrong_language_source_records() {
    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("unknown");
    let mut manifest = write_fixture(&root);
    manifest
        .language_sources
        .insert("EN".to_owned(), vec!["missing".to_owned()]);
    write_manifest(&root, &manifest);
    let unknown = load_prepared_language(&root, Language::En).expect_err("unknown source");
    assert!(matches!(
        unknown,
        ModelSetError::UnknownLanguageSource {
            language: Language::En,
            source_id,
        } if source_id == "missing"
    ));

    let root = directory.path().join("wrong-language");
    let mut manifest = write_fixture(&root);
    manifest
        .sources
        .iter_mut()
        .find(|source| source.source_file_id == "source-en")
        .expect("English source")
        .detector_language = Language::Pt;
    write_manifest(&root, &manifest);
    let wrong = load_prepared_language(&root, Language::En).expect_err("wrong language source");
    assert!(matches!(
        wrong,
        ModelSetError::WrongLanguageSource {
            expected: Language::En,
            actual: Language::Pt,
            source_id,
        } if source_id == "source-en"
    ));
}

#[test]
fn prepared_input_rejects_a_split_count_identity_mismatch() {
    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("prepared-v1");
    let mut manifest = write_fixture(&root);
    manifest
        .language_counts
        .get_mut("EN")
        .expect("English counts")
        .development = 3;
    write_manifest(&root, &manifest);

    let error = load_prepared_language(&root, Language::En).expect_err("count mismatch");

    assert!(matches!(
        error,
        ModelSetError::PreparedSplitCountMismatch {
            language: Language::En,
            split: "development",
            declared: 3,
            file_rows: 2,
        }
    ));
}

#[test]
fn prepared_input_reads_test_count_without_opening_the_test_file() {
    let directory = tempdir().expect("temporary directory");
    let root = directory.path().join("prepared-v1");
    write_fixture(&root);
    assert!(!root.join("EN/test.tsv").exists());

    let input = load_prepared_language(&root, Language::En).expect("prepared English input");

    assert_eq!(input.development.len(), 2);
    assert_eq!(input.validation.len(), 2);
    assert_eq!(input.counts.test, 2);
    assert_eq!(input.sources.len(), 1);
    assert_eq!(input.development[0].label, EvalLabel::Clean);
}

fn write_fixture(root: &Path) -> PreparedManifest {
    fs::create_dir_all(root.join("EN")).expect("create fixture root");
    let mut manifest = fixture_manifest();
    write_target_split(
        root,
        &mut manifest,
        "development",
        concat!(
            "EN\tclean\tdevelopment/clean\tclean message\n",
            "EN\ttoxic\tdevelopment/toxic\ttoxic message\n",
        ),
        HEADER,
    );
    write_target_split(
        root,
        &mut manifest,
        "validation",
        concat!(
            "EN\tclean\tvalidation/clean\tclean validation\n",
            "EN\ttoxic\tvalidation/toxic\ttoxic validation\n",
        ),
        HEADER,
    );
    write_manifest(root, &manifest);
    manifest
}

fn write_target_split(
    root: &Path,
    manifest: &mut PreparedManifest,
    split: &'static str,
    rows: &str,
    header: &str,
) {
    let relative_path = format!("EN/{split}.tsv");
    let bytes = format!("{header}{rows}").into_bytes();
    fs::write(root.join(&relative_path), &bytes).expect("write prepared split");
    manifest.prepared_files.insert(
        relative_path.clone(),
        PreparedFileIdentity {
            relative_path,
            sha256: sha256(&bytes),
            rows: 2,
            clean_rows: 1,
            toxic_rows: 1,
        },
    );
}

fn write_manifest(root: &Path, manifest: &PreparedManifest) {
    fs::create_dir_all(root).expect("create manifest root");
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(manifest).expect("serialize manifest"),
    )
    .expect("write manifest");
}

fn fixture_manifest() -> PreparedManifest {
    let mut language_sources = BTreeMap::new();
    let mut language_counts = BTreeMap::new();
    let mut prepared_files = BTreeMap::new();
    let mut sources = Vec::new();
    for language in Language::ALL {
        let code = language.storage_code();
        let lower = code.to_ascii_lowercase();
        let source_file_id = format!("source-{lower}");
        language_sources.insert(code.to_owned(), vec![source_file_id.clone()]);
        language_counts.insert(
            code.to_owned(),
            PreparedCounts {
                development: 2,
                validation: 2,
                test: 2,
                duplicates: 0,
                conflicts: 0,
                excluded: 0,
            },
        );
        for split in ["development", "validation", "test"] {
            let relative_path = format!("{code}/{split}.tsv");
            prepared_files.insert(
                relative_path.clone(),
                PreparedFileIdentity {
                    relative_path,
                    sha256: digest(),
                    rows: 2,
                    clean_rows: 1,
                    toxic_rows: 1,
                },
            );
        }
        sources.push(SourceRecord {
            dataset: DatasetId::TextDetox,
            detector_language: language,
            source_role: SourceRole::Baseline,
            source_file_id,
            immutable_source_url: "https://example.invalid/source".to_owned(),
            archive_member: None,
            revision: Some("revision".to_owned()),
            file_path: format!("fixture/{lower}.tsv"),
            file_sha256: digest(),
            download_sha256: None,
            acquired_at_unix_seconds: 1_700_000_000,
            license_id: "CC-BY-4.0".to_owned(),
            license_url: "https://example.invalid/license".to_owned(),
            citation: "Fixture citation".to_owned(),
            upstream_lineage: vec!["fixture".to_owned()],
            lineage_status: LineageStatus::Resolved,
        });
    }
    PreparedManifest {
        schema_version: PREPARED_MANIFEST_SCHEMA_VERSION.to_owned(),
        sources,
        language_sources,
        language_counts,
        source_rows: 0,
        source_label_counts: BTreeMap::new(),
        detector_label_counts: BTreeMap::new(),
        source_split_counts: BTreeMap::new(),
        detector_split_counts: BTreeMap::new(),
        inclusion_status_counts: BTreeMap::new(),
        exclusion_reason_counts: BTreeMap::new(),
        prepared_files,
    }
}

fn sha256(bytes: &[u8]) -> Sha256Digest {
    format!("{:x}", Sha256::digest(bytes))
        .try_into()
        .expect("SHA-256 digest")
}

fn digest() -> Sha256Digest {
    HASH.to_owned().try_into().expect("fixture digest")
}
