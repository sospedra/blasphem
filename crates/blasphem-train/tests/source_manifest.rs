use std::{collections::BTreeSet, fs};

use blasphem::Language;
use blasphem_train::source_role::SourceRole;
use blasphem_train::{
    acquisition::{
        freeze_observation, sha256_digest, source_record_from_request,
        source_record_from_request_with_download, validate_source_download,
        validate_source_lock_for_acquisition,
    },
    datasets::{DatasetId, SourceSplit, source_id},
    source_manifest::{parse_frozen_source_lock, parse_source_catalog, parse_source_observation},
};
use serde_json::{Value, json};

#[test]
fn every_current_source_declares_the_baseline_role() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/blasphem-train/metadata/source-lock-v1.json");
    let file = std::fs::File::open(path).expect("readable source lock");
    let lock =
        blasphem_train::source_manifest::parse_frozen_source_lock(file).expect("valid source lock");
    assert_eq!(lock.sources.len(), 88);
    for source in &lock.sources {
        assert_eq!(
            source.source_role,
            SourceRole::Baseline,
            "{} must keep its frozen partition",
            source.source_file_id
        );
    }
}

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn frozen_source_lock_rejects_a_missing_hash_and_unknown_dataset() {
    let mut missing_hash = valid_frozen_lock_json();
    missing_hash["sources"][0]
        .as_object_mut()
        .expect("source object")
        .remove("file_sha256");
    let missing_hash = serde_json::to_vec(&missing_hash).expect("JSON");
    assert!(parse_frozen_source_lock(&missing_hash[..]).is_err());

    let mut unknown = valid_frozen_lock_json();
    unknown["sources"][0]["dataset"] = json!("other");
    let unknown = serde_json::to_vec(&unknown).expect("JSON");
    assert!(parse_frozen_source_lock(&unknown[..]).is_err());
}

#[test]
fn source_observation_and_frozen_lock_are_distinct_schemas() {
    let observation = valid_source_observation_json();
    let bytes = serde_json::to_vec(&observation).expect("JSON");
    assert!(parse_frozen_source_lock(&bytes[..]).is_err());
}

#[test]
fn old_observation_parses_but_textdetox_without_download_hash_cannot_freeze() {
    let without_download_hash = parse_source_observation(
        &serde_json::to_vec(&valid_source_observation_json()).expect("JSON")[..],
    )
    .expect("backward-compatible observation");
    assert_eq!(without_download_hash.sources[0].download_sha256, None);
    let error = freeze_observation(without_download_hash).expect_err("missing download hash");
    assert!(error.to_string().contains("download digest"));
}

#[test]
fn parquet_download_hash_survives_freezing() {
    let mut with_download_hash = valid_source_observation_json();
    with_download_hash["sources"][0]["download_sha256"] = json!(HASH);
    let observation =
        parse_source_observation(&serde_json::to_vec(&with_download_hash).expect("JSON")[..])
            .expect("observation with download hash");
    let source_lock = freeze_observation(observation).expect("frozen source lock");

    assert_eq!(
        source_lock.sources[0]
            .download_sha256
            .as_ref()
            .map(ToString::to_string),
        Some(HASH.to_owned())
    );
}

#[test]
fn acquisition_validation_requires_the_digest_before_any_request() {
    let source_lock =
        parse_frozen_source_lock(&serde_json::to_vec(&valid_frozen_lock_json()).expect("JSON")[..])
            .expect("legacy source lock parses");

    let error = validate_source_lock_for_acquisition(&source_lock).expect_err("missing digest");

    assert!(error.to_string().contains("download digest"));
}

#[test]
fn acquisition_validation_rejects_a_parquet_digest_mismatch() {
    let mut source_lock = valid_frozen_lock_json();
    source_lock["sources"][0]["download_sha256"] = json!(HASH);
    let source_lock =
        parse_frozen_source_lock(&serde_json::to_vec(&source_lock).expect("JSON")[..])
            .expect("source lock");

    let error = validate_source_download(&source_lock.sources[0], b"different Parquet bytes")
        .expect_err("digest mismatch");

    assert!(error.to_string().contains("download digest"));
}

#[test]
fn source_record_keeps_download_and_canonical_hashes_separate() {
    let request =
        parse_source_catalog(&serde_json::to_vec(&valid_source_catalog_json()).expect("JSON")[..])
            .expect("source catalog")
            .sources
            .remove(0);
    let record = source_record_from_request_with_download(
        &request,
        request.requested_url.clone(),
        request.requested_revision.clone(),
        b"Parquet bytes",
        b"source_id\tlanguage\ttoxic\ttext\n",
        1,
    )
    .expect("source record");

    assert_eq!(
        record.download_sha256,
        Some(sha256_digest(b"Parquet bytes"))
    );
    assert_eq!(
        record.file_sha256,
        sha256_digest(b"source_id\tlanguage\ttoxic\ttext\n")
    );
    assert_ne!(record.download_sha256.as_ref(), Some(&record.file_sha256));
}

#[test]
fn legacy_source_record_path_rejects_textdetox_bytes() {
    let request =
        parse_source_catalog(&serde_json::to_vec(&valid_source_catalog_json()).expect("JSON")[..])
            .expect("source catalog")
            .sources
            .remove(0);

    let error = source_record_from_request(
        &request,
        request.requested_url.clone(),
        request.requested_revision.clone(),
        b"canonical TSV bytes",
        1,
    )
    .expect_err("legacy TextDetox record path");

    assert!(error.to_string().contains("separate Parquet bytes"));
}

#[test]
fn textdetox_lock_accepts_only_the_exact_pinned_one_file_url() {
    let revision = blasphem_train::TEXTDETOX_REVISION;
    let mut source_lock = valid_frozen_lock_json();
    source_lock["sources"][0]["revision"] = json!(revision);
    source_lock["sources"][0]["download_sha256"] = json!(HASH);
    source_lock["sources"][0]["immutable_source_url"] = json!(format!(
        "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/{revision}/data/en-00000-of-00001.parquet"
    ));
    let accepted = parse_frozen_source_lock(&serde_json::to_vec(&source_lock).expect("JSON")[..])
        .expect("source lock");
    validate_source_lock_for_acquisition(&accepted).expect("exact pinned URL");

    source_lock["sources"][0]["immutable_source_url"] = json!(format!(
        "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/{revision}/data/fr-00000-of-00001.parquet"
    ));
    let rejected = parse_frozen_source_lock(&serde_json::to_vec(&source_lock).expect("JSON")[..])
        .expect("source lock");

    assert!(validate_source_lock_for_acquisition(&rejected).is_err());
}

#[test]
fn source_manifest_parsers_require_exact_schema_versions_and_lowercase_hashes() {
    let mut catalog = valid_source_catalog_json();
    catalog["schema_version"] = json!("source-catalog-v2");
    assert!(parse_source_catalog(&serde_json::to_vec(&catalog).expect("JSON")[..]).is_err());

    let mut observation = valid_source_observation_json();
    observation["schema_version"] = json!("source-observation-v2");
    assert!(
        parse_source_observation(&serde_json::to_vec(&observation).expect("JSON")[..]).is_err()
    );

    let mut lock = valid_frozen_lock_json();
    lock["sources"][0]["file_sha256"] = json!(HASH.to_ascii_uppercase());
    assert!(parse_frozen_source_lock(&serde_json::to_vec(&lock).expect("JSON")[..]).is_err());
}

#[test]
fn catalog_has_unique_exact_source_identities_and_paths() {
    let path = format!(
        "{}/../../crates/blasphem-train/metadata/source-catalog-v1.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let catalog =
        parse_source_catalog(fs::File::open(path).expect("catalog")).expect("catalog parses");

    assert_eq!(catalog.sources.len(), 23);
    let unique_ids: BTreeSet<_> = catalog
        .sources
        .iter()
        .map(|source| source.source_file_id.as_str())
        .collect();
    assert_eq!(unique_ids.len(), catalog.sources.len());

    let expected = [
        (
            "textdetox-en",
            DatasetId::TextDetox,
            Language::En,
            "textdetox/en.tsv",
        ),
        (
            "textdetox-zh",
            DatasetId::TextDetox,
            Language::Zh,
            "textdetox/zh.tsv",
        ),
        (
            "textdetox-ar",
            DatasetId::TextDetox,
            Language::Ar,
            "textdetox/ar.tsv",
        ),
        (
            "textdetox-fr",
            DatasetId::TextDetox,
            Language::Fr,
            "textdetox/fr.tsv",
        ),
        (
            "textdetox-hi",
            DatasetId::TextDetox,
            Language::Hi,
            "textdetox/hi.tsv",
        ),
        (
            "textdetox-ru",
            DatasetId::TextDetox,
            Language::Ru,
            "textdetox/ru.tsv",
        ),
        (
            "textdetox-ja",
            DatasetId::TextDetox,
            Language::Ja,
            "textdetox/ja.tsv",
        ),
        (
            "textdetox-de",
            DatasetId::TextDetox,
            Language::De,
            "textdetox/de.tsv",
        ),
        (
            "textdetox-it",
            DatasetId::TextDetox,
            Language::It,
            "textdetox/it.tsv",
        ),
        (
            "textdetox-es",
            DatasetId::TextDetox,
            Language::Es,
            "textdetox/es.tsv",
        ),
        (
            "ibrohim-budi-re-dataset",
            DatasetId::IbrohimBudi,
            Language::Ms,
            "datasets/ibrohim-budi-re-dataset/re_dataset.csv",
        ),
        (
            "told-br-alpha",
            DatasetId::ToldBr,
            Language::Pt,
            "datasets/told-br-alpha/ToLD-BR_alpha.csv",
        ),
        (
            "offenseval-tr-training",
            DatasetId::OffensEvalTr,
            Language::Tr,
            "datasets/offenseval-tr-training/offenseval-tr-training-v1.tsv",
        ),
        (
            "offenseval-tr-test",
            DatasetId::OffensEvalTr,
            Language::Tr,
            "datasets/offenseval-tr-test/offenseval-tr-testset-v1.tsv",
        ),
        (
            "offenseval-tr-test-labels",
            DatasetId::OffensEvalTr,
            Language::Tr,
            "datasets/offenseval-tr-test-labels/offenseval-tr-labela-v1.tsv",
        ),
        (
            "vihos-train",
            DatasetId::ViHos,
            Language::Vi,
            "datasets/vihos-train/train.csv",
        ),
        (
            "vihos-development",
            DatasetId::ViHos,
            Language::Vi,
            "datasets/vihos-development/dev.csv",
        ),
        (
            "vihos-test",
            DatasetId::ViHos,
            Language::Vi,
            "datasets/vihos-test/test.csv",
        ),
        (
            "kmhas-train",
            DatasetId::KMHas,
            Language::Ko,
            "datasets/kmhas-train/kmhas_train.txt",
        ),
        (
            "kmhas-validation",
            DatasetId::KMHas,
            Language::Ko,
            "datasets/kmhas-validation/kmhas_valid.txt",
        ),
        (
            "kmhas-test",
            DatasetId::KMHas,
            Language::Ko,
            "datasets/kmhas-test/kmhas_test.txt",
        ),
        (
            "germeval-2018-training",
            DatasetId::GermEval2018,
            Language::De,
            "datasets/germeval-2018-training/germeval2018.training.txt",
        ),
        (
            "germeval-2018-test",
            DatasetId::GermEval2018,
            Language::De,
            "datasets/germeval-2018-test/germeval2018.test.txt",
        ),
    ];

    for (source_file_id, dataset, language, file_path) in expected {
        let source = catalog
            .sources
            .iter()
            .find(|source| source.source_file_id == source_file_id)
            .expect("expected source");
        assert_eq!(source.dataset, dataset, "{source_file_id}");
        assert_eq!(source.detector_language, language, "{source_file_id}");
        assert_eq!(source.file_path, file_path, "{source_file_id}");
    }

    let textdetox = catalog
        .sources
        .iter()
        .filter(|source| source.dataset == DatasetId::TextDetox)
        .count();
    let lexicon = catalog
        .sources
        .iter()
        .filter(|source| source.dataset == DatasetId::Lexicon)
        .count();
    assert_eq!(textdetox, 10);
    assert_eq!(lexicon, 0);

    let textdetox_revision = "01907546324b0330d2d8b7669648cc18823323e5";
    for source in catalog
        .sources
        .iter()
        .filter(|source| source.dataset == DatasetId::TextDetox)
    {
        let source_code = source
            .source_file_id
            .strip_prefix("textdetox-")
            .expect("TextDetox source code");
        assert_eq!(
            source.requested_revision.as_deref(),
            Some(textdetox_revision)
        );
        assert_eq!(source.revision_url, None);
        assert_eq!(
            source.requested_url,
            format!(
                "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/{textdetox_revision}/data/{source_code}-00000-of-00001.parquet"
            )
        );
    }
    assert_eq!(
        catalog
            .sources
            .iter()
            .filter(|source| source.dataset == DatasetId::IbrohimBudi)
            .count(),
        1
    );
    assert_eq!(
        catalog
            .sources
            .iter()
            .filter(|source| source.dataset == DatasetId::ToldBr)
            .count(),
        1
    );
    assert_eq!(
        catalog
            .sources
            .iter()
            .filter(|source| source.dataset == DatasetId::OffensEvalTr)
            .count(),
        3
    );
    assert_eq!(
        catalog
            .sources
            .iter()
            .filter(|source| source.dataset == DatasetId::ViHos)
            .count(),
        3
    );
    assert_eq!(
        catalog
            .sources
            .iter()
            .filter(|source| source.dataset == DatasetId::KMHas)
            .count(),
        3
    );
    let germeval_revision = "9877472d39523effd54cd079b4c61157ed141508";
    let germeval = catalog
        .sources
        .iter()
        .filter(|source| source.dataset == DatasetId::GermEval2018)
        .collect::<Vec<_>>();
    assert_eq!(germeval.len(), 2);
    for source in germeval {
        assert_eq!(
            source.requested_revision.as_deref(),
            Some(germeval_revision)
        );
        assert_eq!(source.revision_url, None);
        assert_eq!(source.archive_member, None);
        let source_name = match source.source_file_id.as_str() {
            "germeval-2018-training" => "germeval2018.training.txt",
            "germeval-2018-test" => "germeval2018.test.txt",
            other => panic!("unexpected GermEval source: {other}"),
        };
        assert_eq!(
            source.requested_url,
            format!(
                "https://raw.githubusercontent.com/uds-lsv/GermEval-2018-Data/{germeval_revision}/{source_name}"
            )
        );
    }
}

#[test]
fn source_identifiers_use_the_dataset_revision_split_and_native_id() {
    assert_eq!(
        source_id(DatasetId::TextDetox, "019075", SourceSplit::Unsplit, "42"),
        "textdetox@019075/unsplit/42"
    );
}

#[test]
fn the_source_lock_registers_the_spanish_textdetox_input() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/blasphem-train/metadata/source-lock-v1.json");
    let file = std::fs::File::open(path).expect("readable source lock");
    let lock =
        blasphem_train::source_manifest::parse_frozen_source_lock(file).expect("valid source lock");
    assert_eq!(lock.sources.len(), 88);
    let spanish = lock
        .sources
        .iter()
        .find(|source| source.source_file_id == "textdetox-es")
        .expect("Spanish source entry");
    assert_eq!(spanish.file_path, "textdetox/es.tsv");
    assert_eq!(spanish.detector_language, blasphem::Language::Es);
}

fn valid_source_catalog_json() -> Value {
    json!({
        "schema_version": "source-catalog-v1",
        "sources": [source_request_json()],
    })
}

fn valid_source_observation_json() -> Value {
    json!({
        "schema_version": "source-observation-v1",
        "sources": [source_record_json()],
    })
}

fn valid_frozen_lock_json() -> Value {
    json!({
        "schema_version": "source-lock-v1",
        "sources": [frozen_source_json()],
    })
}

fn source_request_json() -> Value {
    json!({
        "dataset": "textdetox",
        "detector_language": "EN",
        "source_role": "baseline",
        "source_file_id": "textdetox-en",
        "requested_url": "https://example.test/rows",
        "revision_url": "https://example.test/revision",
        "requested_revision": "abc123",
        "archive_member": null,
        "file_path": "textdetox/en.tsv",
        "license_id": "CC-BY-4.0",
        "license_url": "https://creativecommons.org/licenses/by/4.0/",
        "license_year": 2024,
        "citation": "Example citation",
        "upstream_lineage": ["example"],
        "lineage_status": "resolved",
    })
}

fn source_record_json() -> Value {
    json!({
        "dataset": "textdetox",
        "detector_language": "EN",
        "source_role": "baseline",
        "source_file_id": "textdetox-en",
        "immutable_source_url": "https://example.test/rows",
        "archive_member": null,
        "revision": "abc123",
        "file_path": "textdetox/en.tsv",
        "file_sha256": HASH,
        "acquired_at_unix_seconds": 1,
        "license_id": "CC-BY-4.0",
        "license_url": "https://creativecommons.org/licenses/by/4.0/",
        "license_year": 2024,
        "citation": "Example citation",
        "upstream_lineage": ["example"],
        "lineage_status": "resolved",
    })
}

fn frozen_source_json() -> Value {
    json!({
        "dataset": "textdetox",
        "detector_language": "EN",
        "source_role": "baseline",
        "source_file_id": "textdetox-en",
        "immutable_source_url": "https://example.test/rows",
        "archive_member": null,
        "revision": "abc123",
        "file_path": "textdetox/en.tsv",
        "file_sha256": HASH,
        "license_id": "CC-BY-4.0",
        "license_url": "https://creativecommons.org/licenses/by/4.0/",
        "license_year": 2024,
        "citation": "Example citation",
        "upstream_lineage": ["example"],
        "lineage_status": "resolved",
    })
}

#[test]
fn every_frozen_source_states_the_upstream_license_year() {
    let bytes = std::fs::read("../../crates/blasphem-train/metadata/source-lock-v1.json").unwrap();
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(bytes.as_slice()).unwrap();

    assert_eq!(lock.sources.len(), 88);
    for source in &lock.sources {
        assert!(
            (1990..=2026).contains(&source.license_year),
            "{} has an implausible license year {}",
            source.source_file_id,
            source.license_year
        );
    }
}

#[test]
fn every_textdetox_lock_entry_carries_a_download_digest() {
    let bytes = std::fs::read("../../crates/blasphem-train/metadata/source-lock-v1.json").unwrap();
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(bytes.as_slice()).unwrap();

    let missing: Vec<&str> = lock
        .sources
        .iter()
        .filter(|source| source.dataset == blasphem_train::datasets::DatasetId::TextDetox)
        .filter(|source| source.download_sha256.is_none())
        .map(|source| source.source_file_id.as_str())
        .collect();

    assert_eq!(missing, Vec::<&str>::new());
}
