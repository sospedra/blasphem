use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use blasphem::{EvalLabel, Language};
use blasphem_train::{
    datasets::{
        DatasetId, DatasetSplit, InclusionStatus, LineageStatus, PreparedCounts, PreparedLanguage,
        PreparedRow, ProvenanceRow, SourceSplit,
    },
    evidence::Sha256Digest,
    publication::publish_prepared,
    source_manifest::{SOURCE_OBSERVATION_SCHEMA_VERSION, SourceObservation, SourceRecord},
    source_role::SourceRole,
};
use tempfile::tempdir;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn publication_writes_one_provenance_row_per_source_row() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let published = publish_fixture(&output).expect("publish");

    let provenance = read_tsv(output.join("provenance.tsv"));

    assert_eq!(provenance.len() - 1, published.manifest.source_rows);
    assert_eq!(provenance[0].len(), 27);
    assert_eq!(
        provenance[0],
        vec![
            "dataset",
            "source_file_id",
            "source_id",
            "immutable_source_url",
            "archive_member",
            "revision",
            "file_path",
            "file_sha256",
            "acquired_at_unix_seconds",
            "license_id",
            "license_url",
            "citation",
            "upstream_lineage",
            "lineage_status",
            "source_language_code",
            "detector_language_code",
            "source_label",
            "detector_label",
            "label_conversion_version",
            "split_version",
            "normalization_version",
            "canonical_group_id",
            "representative_source_id",
            "source_split",
            "detector_split",
            "inclusion_status",
            "exclusion_reason",
        ]
    );
    assert!(provenance.iter().skip(1).all(|row| {
        row[3] == "https://example.invalid/source"
            && row[7] == HASH
            && row[18] == "textdetox-binary-v1"
    }));
}

#[test]
fn prepared_rows_round_trip_source_ids_and_text() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");

    publish_fixture(&output).expect("publish");

    let rows = read_tsv(output.join("EN/development.tsv"));
    assert_eq!(rows[0], ["detector_language", "label", "source_id", "text"]);
    assert!(
        rows.iter()
            .any(|row| row[2] == "EN/development/clean" && row[3] == "tab\tquote \"é\"\nline")
    );
}

#[test]
fn publication_builds_complete_stable_manifest() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let published = publish_fixture(&output).expect("publish");

    assert_eq!(
        published.manifest.language_sources.len(),
        Language::ALL.len()
    );
    assert_eq!(
        published.manifest.language_counts.len(),
        Language::ALL.len()
    );
    assert_eq!(published.manifest.prepared_files.len(), 45);
    assert_eq!(
        published.manifest.language_sources["ES"],
        vec!["fixture-es"]
    );
    assert!(output.join("ES").exists());
    assert_eq!(
        published.manifest.prepared_files["EN/development.tsv"].rows,
        2
    );
    assert_eq!(
        published.manifest.prepared_files["EN/development.tsv"].clean_rows,
        1
    );
    assert_eq!(
        published.manifest.prepared_files["EN/development.tsv"].toxic_rows,
        1
    );
    assert_eq!(
        published.manifest.source_label_counts["textdetox/en/0"],
        601
    );
    assert_eq!(published.manifest.detector_label_counts["EN/clean"], 601);
    assert_eq!(
        published.manifest.source_split_counts["textdetox/unsplit"],
        18_030
    );
    assert_eq!(
        published.manifest.detector_split_counts["EN/validation"],
        600
    );
    assert_eq!(
        published.manifest.inclusion_status_counts["included"],
        18_030
    );
    assert!(published.manifest.exclusion_reason_counts.is_empty());
}

#[test]
fn publication_excludes_audit_only_rows_from_split_counts() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut languages = fixture_languages();
    let language = languages
        .iter_mut()
        .find(|language| language.language == Language::En)
        .expect("English fixture");
    let mut audit_only = language.provenance[0].clone();
    audit_only.source_id = "EN/development/audit-only".to_owned();
    audit_only.detector_split = Some(DatasetSplit::Development);
    audit_only.inclusion_status = InclusionStatus::Excluded;
    audit_only.exclusion_reason = Some(blasphem_train::datasets::ExclusionReason::AuditOnly);
    audit_only.canonical_group_id = None;
    audit_only.representative_source_id = None;
    language.provenance.push(audit_only);

    let published = publish_prepared(&output, &languages, &fixture_observation())
        .expect("publish with audit-only row");
    let counts = &published.manifest.language_counts["EN"];

    assert_eq!(counts.development, 2);
    assert_eq!(counts.excluded, 1);
    assert_eq!(
        counts.development,
        published.manifest.prepared_files["EN/development.tsv"].rows
    );
}

#[test]
fn publication_writes_the_complete_nested_language_tree() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");

    publish_fixture(&output).expect("publish");

    for language in Language::ALL {
        for split in ["development", "validation", "test"] {
            let path = output
                .join(language.storage_code())
                .join(format!("{split}.tsv"));
            assert!(path.is_file(), "missing {}", path.display());
            assert!(fs::metadata(path).expect("prepared file metadata").len() > 0);
        }
    }
}

#[test]
fn publication_rejects_an_empty_development_class() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut languages = fixture_languages();
    languages[0]
        .development
        .retain(|row| row.label == EvalLabel::Toxic);
    languages[0]
        .provenance
        .retain(|row| row.source_id != "EN/development/clean");

    let error = publish_prepared(&output, &languages, &fixture_observation())
        .expect_err("empty clean development");

    assert!(error.to_string().contains("development"));
    assert!(!output.exists());
}

#[test]
fn publication_rejects_duplicate_prepared_source_ids_in_one_split() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut languages = fixture_languages();
    let duplicate = languages[0].development[0].clone();
    languages[0].development.push(duplicate);

    let error = publish_prepared(&output, &languages, &fixture_observation())
        .expect_err("duplicate source identifier");

    assert!(matches!(
        error,
        blasphem_train::PreparedPublicationError::DuplicatePreparedSourceId(source_id)
            if source_id == "EN/development/clean"
    ));
    assert!(!output.exists());
}

#[test]
fn publication_rejects_duplicate_prepared_source_ids_across_splits() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut languages = fixture_languages();
    let duplicate = languages[0].development[0].clone();
    languages[0].validation.push(duplicate);

    let error = publish_prepared(&output, &languages, &fixture_observation())
        .expect_err("duplicate source identifier");

    assert!(matches!(
        error,
        blasphem_train::PreparedPublicationError::DuplicatePreparedSourceId(source_id)
            if source_id == "EN/development/clean"
    ));
    assert!(!output.exists());
}

#[test]
fn publication_rejects_a_prepared_row_for_another_language() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut languages = fixture_languages();
    languages[0].development[0].detector_language = Language::Es;

    let error = publish_prepared(&output, &languages, &fixture_observation())
        .expect_err("mismatched row language");

    assert!(matches!(
        error,
        blasphem_train::PreparedPublicationError::PreparedRowLanguageMismatch {
            source_id,
            actual: Language::Es,
            expected: Language::En,
        } if source_id == "EN/development/clean"
    ));
    assert!(!output.exists());
}

#[test]
fn publication_rejects_blank_required_provenance_strings() {
    for field in [
        "source_id",
        "source_language_code",
        "source_label",
        "split_version",
        "normalization_version",
    ] {
        let directory = tempdir().expect("temporary directory");
        let output = directory.path().join("prepared-v1");
        let mut languages = fixture_languages();
        let row = &mut languages[0].provenance[0];
        match field {
            "source_id" => row.source_id.clear(),
            "source_language_code" => row.source_language_code.clear(),
            "source_label" => row.source_label.clear(),
            "split_version" => row.split_version.clear(),
            "normalization_version" => row.normalization_version.clear(),
            _ => unreachable!("known field"),
        }

        let error = publish_prepared(&output, &languages, &fixture_observation())
            .expect_err("blank required provenance field");

        assert!(matches!(
            error,
            blasphem_train::PreparedPublicationError::BlankRequiredProvenanceField(actual)
                if actual == field
        ));
        assert!(!output.exists());
    }
}

#[test]
fn publication_sorts_each_prepared_split_by_source_id() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut languages = fixture_languages();
    languages[0].development.reverse();

    publish_prepared(&output, &languages, &fixture_observation()).expect("publish");

    let rows = read_tsv(output.join("EN/development.tsv"));
    assert_eq!(rows[1][2], "EN/development/clean");
    assert_eq!(rows[2][2], "EN/development/toxic");
}

#[test]
fn publication_rejects_unknown_or_mismatched_source_records() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    let mut observation = fixture_observation();
    observation.sources[0].detector_language = Language::Es;

    assert!(publish_prepared(&output, &fixture_languages(), &observation).is_err());
    assert!(!output.exists());
}

#[test]
fn existing_destination_survives_failed_publication() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("prepared-v1");
    fs::create_dir(&output).expect("existing destination");
    fs::write(output.join("owner.txt"), "owner").expect("owner marker");

    assert!(publish_prepared(&output, &fixture_languages(), &fixture_observation()).is_err());
    assert_eq!(
        fs::read_to_string(output.join("owner.txt")).expect("owner marker"),
        "owner"
    );
}

fn publish_fixture(output: &Path) -> blasphem_train::publication::PreparedPublicationResult {
    publish_prepared(output, &fixture_languages(), &fixture_observation())
}

fn fixture_languages() -> Vec<PreparedLanguage> {
    Language::ALL.into_iter().map(fixture_language).collect()
}

fn fixture_language(language: Language) -> PreparedLanguage {
    let code = language.code();
    let source_file_id = format!("fixture-{}", code.to_ascii_lowercase());
    let mut rows = BTreeMap::new();
    rows.insert(
        DatasetSplit::Development,
        vec![
            fixture_row(
                language,
                "development/clean",
                EvalLabel::Clean,
                "tab\tquote \"é\"\nline",
            ),
            fixture_row(language, "development/toxic", EvalLabel::Toxic, "toxic"),
        ],
    );
    for split in [DatasetSplit::Validation, DatasetSplit::Test] {
        let mut split_rows = Vec::new();
        for label in [EvalLabel::Clean, EvalLabel::Toxic] {
            for index in 0..300 {
                split_rows.push(fixture_row(
                    language,
                    &format!("{split:?}/{label:?}/{index:03}"),
                    label,
                    "fixture",
                ));
            }
        }
        rows.insert(split, split_rows);
    }
    let provenance = rows
        .iter()
        .flat_map(|(split, prepared)| {
            prepared.iter().map(|row| ProvenanceRow {
                dataset: DatasetId::TextDetox,
                source_file_id: source_file_id.clone(),
                source_id: row.source_id.clone(),
                immutable_source_url: String::new(),
                archive_member: None,
                revision: None,
                file_path: String::new(),
                file_sha256: digest(),
                acquired_at_unix_seconds: 0,
                license_id: String::new(),
                license_url: String::new(),
                citation: String::new(),
                upstream_lineage: Vec::new(),
                lineage_status: LineageStatus::Unresolved,
                source_language_code: code.to_ascii_lowercase(),
                detector_language_code: Some(language.storage_code().to_owned()),
                source_label: match row.label {
                    EvalLabel::Clean => "0",
                    EvalLabel::Toxic => "1",
                }
                .to_owned(),
                detector_label: Some(row.label),
                label_conversion_version: String::new(),
                split_version: "split-v1".to_owned(),
                normalization_version: "normalization-v1".to_owned(),
                canonical_group_id: Some(row.source_id.clone()),
                representative_source_id: Some(row.source_id.clone()),
                source_split: SourceSplit::Unsplit,
                detector_split: Some(*split),
                inclusion_status: InclusionStatus::Included,
                exclusion_reason: None,
            })
        })
        .collect::<Vec<_>>();
    PreparedLanguage {
        language,
        development: rows
            .remove(&DatasetSplit::Development)
            .expect("development"),
        validation: rows.remove(&DatasetSplit::Validation).expect("validation"),
        test: rows.remove(&DatasetSplit::Test).expect("test"),
        provenance,
        counts: PreparedCounts {
            development: 2,
            validation: 600,
            test: 600,
            duplicates: 0,
            conflicts: 0,
            excluded: 0,
        },
    }
}

fn fixture_row(language: Language, suffix: &str, label: EvalLabel, text: &str) -> PreparedRow {
    PreparedRow {
        detector_language: language,
        label,
        source_id: format!("{}/{suffix}", language.code()),
        text: text.to_owned(),
    }
}

fn fixture_observation() -> SourceObservation {
    SourceObservation {
        schema_version: SOURCE_OBSERVATION_SCHEMA_VERSION.to_owned(),
        sources: Language::ALL.into_iter().map(fixture_source).collect(),
    }
}

fn fixture_source(language: Language) -> SourceRecord {
    let code = language.code().to_ascii_lowercase();
    SourceRecord {
        dataset: DatasetId::TextDetox,
        detector_language: language,
        source_role: SourceRole::Baseline,
        source_file_id: format!("fixture-{code}"),
        immutable_source_url: "https://example.invalid/source".to_owned(),
        archive_member: None,
        revision: Some("revision".to_owned()),
        file_path: format!("fixture/{code}.tsv"),
        file_sha256: digest(),
        download_sha256: None,
        acquired_at_unix_seconds: 1_700_000_000,
        license_id: "CC-BY-4.0".to_owned(),
        license_url: "https://example.invalid/license".to_owned(),
        license_year: 2024,
        citation: "Fixture citation".to_owned(),
        upstream_lineage: vec!["fixture-lineage".to_owned()],
        lineage_status: LineageStatus::Resolved,
    }
}

fn digest() -> Sha256Digest {
    HASH.to_owned().try_into().expect("digest")
}

fn read_tsv(path: PathBuf) -> Vec<Vec<String>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .from_path(path)
        .expect("open TSV");
    reader
        .records()
        .map(|record| record.expect("TSV row").iter().map(str::to_owned).collect())
        .collect()
}
