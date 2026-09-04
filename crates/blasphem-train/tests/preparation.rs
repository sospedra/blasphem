use std::collections::{BTreeMap, BTreeSet};

use blasphem::{EvalLabel, Language};
use blasphem_train::datasets::{
    DatasetId, DatasetSplit, ExclusionReason, ImportedRow, PreparationPolicy, RowDisposition,
    SourceSplit, SplitPolicy, prepare_language, split_for_key, split_hash,
};
use blasphem_train::source_role::SourceRole;
use blasphem_train::{
    ProvenanceStatus, TextDetoxLanguage, TextDetoxSourceRow, prepare_textdetox,
    split_for_key as textdetox_split_for_key,
};

#[test]
fn hash_policies_use_the_exact_uppercase_byte_contract() {
    assert_eq!(
        split_for_key(Language::En, "you are an idiot"),
        DatasetSplit::Development
    );
    assert_eq!(
        split_for_key(Language::En, "message 1"),
        DatasetSplit::Validation
    );
    assert_eq!(
        split_for_key(Language::En, "message 14"),
        DatasetSplit::Test
    );
}

#[test]
fn locks_the_fnv_split_contract() {
    assert_eq!(
        textdetox_split_for_key("EN", "you are an idiot"),
        blasphem_train::DatasetSplit::Development
    );
    assert_eq!(
        textdetox_split_for_key("EN", "message 1"),
        blasphem_train::DatasetSplit::Validation
    );
    assert_eq!(
        textdetox_split_for_key("EN", "message 14"),
        blasphem_train::DatasetSplit::Test
    );
}

#[test]
fn deduplicates_normalized_text_and_preserves_source_ids() {
    let rows = vec![
        textdetox_source_row("b", 1, "You are an IDIOT!"),
        textdetox_source_row("a", 1, "you are an idiot"),
    ];

    let prepared =
        prepare_textdetox(&rows, &BTreeSet::from(["EN".to_owned()])).expect("prepared rows");

    assert_eq!(prepared.summary.evaluation_rows, 1);
    assert_eq!(prepared.summary.duplicate_rows, 1);
    assert_eq!(prepared.provenance.len(), 2);
    assert_eq!(
        prepared.provenance[0].group_id.as_deref(),
        Some("v1-a9e8e8eea9fd77d5")
    );
    assert_eq!(
        prepared.provenance[0].canonical_source_id.as_deref(),
        Some("a")
    );
    assert_eq!(
        prepared.provenance[0].status,
        ProvenanceStatus::Representative
    );
    assert_eq!(prepared.provenance[1].status, ProvenanceStatus::Duplicate);
    assert_eq!(prepared.development[0].text, "you are an idiot");
}

#[test]
fn excludes_a_group_with_conflicting_labels() {
    let rows = vec![
        textdetox_source_row("a", 0, "Same text!"),
        textdetox_source_row("b", 1, "same text"),
    ];

    let prepared =
        prepare_textdetox(&rows, &BTreeSet::from(["EN".to_owned()])).expect("prepared rows");

    assert_eq!(prepared.summary.evaluation_rows, 0);
    assert_eq!(prepared.summary.conflicting_groups, 1);
    assert_eq!(prepared.summary.duplicate_rows, 0);
    assert!(prepared.provenance.iter().all(|row| {
        row.status == ProvenanceStatus::LabelConflict
            && row.group_id.is_some()
            && row.split.is_some()
            && row.canonical_source_id.is_none()
    }));
}

#[test]
fn validates_detector_language_filters_case_insensitively() {
    let rows = vec![textdetox_source_row("a", 0, "hello")];

    let prepared = prepare_textdetox(&rows, &BTreeSet::from(["eN".to_owned()]))
        .expect("case-insensitive filter");
    let error =
        prepare_textdetox(&rows, &BTreeSet::from(["PT".to_owned()])).expect_err("unknown filter");

    assert_eq!(prepared.summary.evaluation_rows, 1);
    assert!(matches!(
        error,
        blasphem_train::TextDetoxError::InvalidLanguage(language) if language == "PT"
    ));
}

#[test]
fn gives_empty_text_precedence_over_an_unsupported_language() {
    let rows = vec![
        textdetox_source_row("z-empty", 0, " \t "),
        textdetox_source_row("a-unsupported", 0, "hello"),
    ];

    let prepared =
        prepare_textdetox(&rows, &BTreeSet::from(["ES".to_owned()])).expect("classified rows");

    assert_eq!(prepared.summary.empty_rows, 1);
    assert_eq!(prepared.summary.unsupported_rows, 1);
    assert_eq!(prepared.provenance[0].source_id, "a-unsupported");
    assert_eq!(
        prepared.provenance[0].status,
        ProvenanceStatus::UnsupportedLanguage
    );
    assert_eq!(prepared.provenance[1].status, ProvenanceStatus::EmptyText);
    assert!(prepared.provenance.iter().all(|row| {
        row.group_id.is_none() && row.split.is_none() && row.canonical_source_id.is_none()
    }));
}

#[test]
fn sorts_each_evaluation_split_by_group_id() {
    let rows = vec![
        textdetox_source_row("a", 0, "alpha"),
        textdetox_source_row("w", 0, "world"),
        textdetox_source_row("g", 0, "gamma"),
        textdetox_source_row("d", 0, "delta"),
    ];

    let prepared =
        prepare_textdetox(&rows, &BTreeSet::from(["EN".to_owned()])).expect("prepared rows");

    assert_eq!(
        prepared
            .rows(blasphem_train::DatasetSplit::Development)
            .iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>(),
        ["delta", "gamma", "world", "alpha"]
    );
}

#[test]
fn highest_split_and_smallest_source_id_select_the_representative() {
    let prepared =
        prepare_language(cross_split_duplicates(), &preserve_official()).expect("prepared");

    assert_eq!(prepared.test[0].source_id, "textdetox@v1/test/001");
    assert_eq!(prepared.counts.duplicates, 2);
    assert_eq!(
        prepared
            .provenance
            .iter()
            .filter(|row| row.exclusion_reason == Some(ExclusionReason::Duplicate))
            .count(),
        2
    );
}

#[test]
fn excludes_every_row_in_a_conflicting_label_group() {
    let rows = vec![
        candidate(
            "development/001",
            SourceSplit::Development,
            EvalLabel::Clean,
            "same text",
        ),
        candidate("test/001", SourceSplit::Test, EvalLabel::Toxic, "same text"),
    ];

    let prepared = prepare_language(rows, &preserve_official()).expect("prepared");

    assert!(prepared.development.is_empty());
    assert!(prepared.validation.is_empty());
    assert!(prepared.test.is_empty());
    assert_eq!(prepared.counts.conflicts, 2);
    assert_eq!(prepared.counts.excluded, 2);
    assert!(prepared.provenance.iter().all(|row| {
        row.exclusion_reason == Some(ExclusionReason::LabelConflict)
            && row.inclusion_status == blasphem_train::datasets::InclusionStatus::Excluded
    }));
}

#[test]
fn audit_only_identifiers_accept_only_detector_development_rows() {
    let cases = vec![
        (
            hash_policy(),
            candidate(
                "hash-development",
                SourceSplit::Unsplit,
                EvalLabel::Clean,
                "message 2",
            ),
            true,
        ),
        (
            hash_policy(),
            candidate(
                "hash-validation",
                SourceSplit::Unsplit,
                EvalLabel::Clean,
                "message 1",
            ),
            false,
        ),
        (
            hash_policy(),
            candidate(
                "hash-test",
                SourceSplit::Unsplit,
                EvalLabel::Clean,
                "message 14",
            ),
            false,
        ),
        (
            turkish_policy(),
            candidate_for(
                Language::Tr,
                "turkish-development",
                SourceSplit::Train,
                EvalLabel::Clean,
                "message 0",
            ),
            true,
        ),
        (
            turkish_policy(),
            candidate_for(
                Language::Tr,
                "turkish-validation",
                SourceSplit::Train,
                EvalLabel::Clean,
                "message 3",
            ),
            false,
        ),
        (
            turkish_policy(),
            candidate_for(
                Language::Tr,
                "turkish-test",
                SourceSplit::Test,
                EvalLabel::Clean,
                "message 1",
            ),
            false,
        ),
        (
            preserve_official(),
            candidate(
                "preserve-train",
                SourceSplit::Train,
                EvalLabel::Clean,
                "one",
            ),
            true,
        ),
        (
            preserve_official(),
            candidate(
                "preserve-development",
                SourceSplit::Development,
                EvalLabel::Clean,
                "two",
            ),
            false,
        ),
        (
            preserve_official(),
            candidate(
                "preserve-validation",
                SourceSplit::Validation,
                EvalLabel::Clean,
                "three",
            ),
            false,
        ),
        (
            preserve_official(),
            candidate("preserve-test", SourceSplit::Test, EvalLabel::Clean, "four"),
            false,
        ),
    ];

    for (mut policy, row, accepted) in cases {
        policy.audit_only_source_ids = BTreeSet::from([row.source_id.clone()]);
        let result = prepare_language(vec![row.clone()], &policy);

        if accepted {
            let prepared = result.expect("accepted audit row");
            assert_eq!(prepared.counts.excluded, 1);
            assert_eq!(
                prepared.provenance[0].exclusion_reason,
                Some(ExclusionReason::AuditOnly)
            );
        } else {
            assert!(result.is_err(), "{} must fail", row.source_id);
        }
    }
}

#[test]
fn audit_only_identifiers_reject_unknown_and_duplicate_source_ids() {
    let mut policy = hash_policy();
    policy.audit_only_source_ids = BTreeSet::from(["unknown".to_owned()]);
    assert!(
        prepare_language(
            vec![candidate(
                "known",
                SourceSplit::Unsplit,
                EvalLabel::Clean,
                "message 2",
            )],
            &policy,
        )
        .is_err()
    );

    let mut policy = hash_policy();
    policy.audit_only_source_ids = BTreeSet::from(["duplicate".to_owned()]);
    let duplicated = candidate(
        "duplicate",
        SourceSplit::Unsplit,
        EvalLabel::Clean,
        "message 2",
    );
    assert!(prepare_language(vec![duplicated.clone(), duplicated], &policy).is_err());
}

#[test]
fn audit_only_exclusions_precede_importer_exclusions() {
    let mut policy = preserve_official();
    policy.audit_only_source_ids = BTreeSet::from(["train/004".to_owned()]);
    let row = ImportedRow {
        disposition: RowDisposition::Excluded(ExclusionReason::AmbiguousLabel),
        ..candidate(
            "train/004",
            SourceSplit::Train,
            EvalLabel::Clean,
            "audited row",
        )
    };

    let prepared = prepare_language(vec![row], &policy).expect("prepared");

    assert_eq!(
        prepared.provenance[0].exclusion_reason,
        Some(ExclusionReason::AuditOnly)
    );
}

#[test]
fn hash_policy_assigns_unsplit_rows_to_locked_buckets() {
    let policy = hash_policy();
    let rows = vec![
        candidate(
            "a",
            SourceSplit::Unsplit,
            EvalLabel::Clean,
            "you are an idiot",
        ),
        candidate("b", SourceSplit::Unsplit, EvalLabel::Clean, "message 1"),
        candidate("c", SourceSplit::Unsplit, EvalLabel::Clean, "message 14"),
    ];

    let prepared = prepare_language(rows, &policy).expect("prepared");

    assert_eq!(prepared.counts.development, 1);
    assert_eq!(prepared.counts.validation, 1);
    assert_eq!(prepared.counts.test, 1);
}

#[test]
fn turkish_policy_reserves_official_test_and_hashes_training_rows() {
    let policy = PreparationPolicy {
        language: Language::Tr,
        split_policy: SplitPolicy::TurkishOfficialTest,
        split_version: "split-v1",
        normalization_version: "normalization-v1",
        audit_only_source_ids: BTreeSet::new(),
        source_roles: BTreeMap::new(),
    };
    let rows = vec![
        candidate_for(
            Language::Tr,
            "train",
            SourceSplit::Train,
            EvalLabel::Clean,
            "message 3",
        ),
        candidate_for(
            Language::Tr,
            "test",
            SourceSplit::Test,
            EvalLabel::Toxic,
            "message 1",
        ),
    ];

    let prepared = prepare_language(rows, &policy).expect("prepared");

    assert_eq!(prepared.counts.validation, 1);
    assert_eq!(prepared.counts.test, 1);
    assert_eq!(prepared.test[0].source_id, "test");
}

#[test]
fn preserve_policy_keeps_official_vietnamese_and_korean_splits() {
    for language in [Language::Vi, Language::Ko] {
        let policy = PreparationPolicy {
            language,
            split_policy: SplitPolicy::PreserveOfficial,
            split_version: "split-v1",
            normalization_version: "normalization-v1",
            audit_only_source_ids: BTreeSet::new(),
            source_roles: BTreeMap::new(),
        };
        let rows = vec![
            candidate_for(
                language,
                "train",
                SourceSplit::Train,
                EvalLabel::Clean,
                "one",
            ),
            candidate_for(
                language,
                "development",
                SourceSplit::Development,
                EvalLabel::Clean,
                "two",
            ),
            candidate_for(
                language,
                "test",
                SourceSplit::Test,
                EvalLabel::Toxic,
                "three",
            ),
        ];

        let prepared = prepare_language(rows, &policy).expect("prepared");

        assert_eq!(prepared.counts.development, 1);
        assert_eq!(prepared.counts.validation, 1);
        assert_eq!(prepared.counts.test, 1);
    }
}

#[test]
fn rejects_a_candidate_for_another_detector_language() {
    let error = prepare_language(
        vec![candidate_for(
            Language::Es,
            "es",
            SourceSplit::Unsplit,
            EvalLabel::Clean,
            "hola",
        )],
        &hash_policy(),
    )
    .expect_err("language mismatch");

    assert!(error.to_string().contains("En"));
}

#[test]
fn validates_included_and_excluded_provenance_rows() {
    let rows = vec![
        candidate(
            "included",
            SourceSplit::Unsplit,
            EvalLabel::Clean,
            "included",
        ),
        ImportedRow {
            disposition: RowDisposition::Excluded(ExclusionReason::AmbiguousLabel),
            ..candidate(
                "excluded",
                SourceSplit::Unsplit,
                EvalLabel::Clean,
                "excluded",
            )
        },
    ];

    let prepared = prepare_language(rows, &hash_policy()).expect("prepared");

    assert_eq!(prepared.provenance.len(), 2);
    assert!(prepared.provenance.iter().all(|row| row.validate().is_ok()));
    assert_eq!(prepared.counts.excluded, 1);
}

#[test]
fn a_training_only_source_never_enters_validation_or_test() {
    let mut roles = std::collections::BTreeMap::new();
    roles.insert("community-es-demo".to_owned(), SourceRole::TrainingOnly);
    let policy = PreparationPolicy {
        language: Language::Es,
        split_policy: SplitPolicy::Hash70_15_15,
        split_version: "fnv1a-uppercase-v1",
        normalization_version: "runtime-normalize-v2",
        audit_only_source_ids: Default::default(),
        source_roles: roles,
    };
    let rows = community_rows(40);
    let prepared = prepare_language(rows, &policy).expect("prepares the community rows");
    assert_eq!(prepared.validation.len(), 0);
    assert_eq!(prepared.test.len(), 0);
    assert_eq!(prepared.development.len(), 40);
}

fn community_rows(count: usize) -> Vec<ImportedRow> {
    (0..count)
        .map(|index| {
            let source_id = format!("community-es-demo/row-{index:06}");
            ImportedRow {
                dataset: DatasetId::Community,
                source_file_id: "community-es-demo".to_owned(),
                source_id,
                source_language_code: "es".to_owned(),
                detector_language: Some(Language::Es),
                detector_language_code: Some(Language::Es.code().to_owned()),
                source_label: "clean".to_owned(),
                text: format!("mensaje comunitario numero {index}"),
                source_split: SourceSplit::Unsplit,
                disposition: RowDisposition::Candidate(EvalLabel::Clean),
            }
        })
        .collect()
}

fn hash_policy() -> PreparationPolicy {
    PreparationPolicy {
        language: Language::En,
        split_policy: SplitPolicy::Hash70_15_15,
        split_version: "split-v1",
        normalization_version: "normalization-v1",
        audit_only_source_ids: BTreeSet::new(),
        source_roles: BTreeMap::new(),
    }
}

fn turkish_policy() -> PreparationPolicy {
    PreparationPolicy {
        language: Language::Tr,
        split_policy: SplitPolicy::TurkishOfficialTest,
        split_version: "split-v1",
        normalization_version: "normalization-v1",
        audit_only_source_ids: BTreeSet::new(),
        source_roles: BTreeMap::new(),
    }
}

fn preserve_official() -> PreparationPolicy {
    PreparationPolicy {
        language: Language::En,
        split_policy: SplitPolicy::PreserveOfficial,
        split_version: "split-v1",
        normalization_version: "normalization-v1",
        audit_only_source_ids: BTreeSet::new(),
        source_roles: BTreeMap::new(),
    }
}

fn cross_split_duplicates() -> Vec<ImportedRow> {
    vec![
        candidate(
            "textdetox@v1/development/002",
            SourceSplit::Development,
            EvalLabel::Toxic,
            "same text",
        ),
        candidate(
            "textdetox@v1/validation/001",
            SourceSplit::Validation,
            EvalLabel::Toxic,
            "same text",
        ),
        candidate(
            "textdetox@v1/test/001",
            SourceSplit::Test,
            EvalLabel::Toxic,
            "same text",
        ),
    ]
}

fn candidate(
    source_id: &str,
    source_split: SourceSplit,
    label: EvalLabel,
    text: &str,
) -> ImportedRow {
    candidate_for(Language::En, source_id, source_split, label, text)
}

fn candidate_for(
    language: Language,
    source_id: &str,
    source_split: SourceSplit,
    label: EvalLabel,
    text: &str,
) -> ImportedRow {
    ImportedRow {
        dataset: DatasetId::TextDetox,
        source_file_id: "fixture".to_owned(),
        source_id: source_id.to_owned(),
        source_language_code: language.code().to_ascii_lowercase(),
        detector_language: Some(language),
        detector_language_code: Some(language.code().to_owned()),
        source_label: match label {
            EvalLabel::Clean => "0".to_owned(),
            EvalLabel::Toxic => "1".to_owned(),
        },
        text: text.to_owned(),
        source_split,
        disposition: RowDisposition::Candidate(label),
    }
}

fn textdetox_source_row(source_id: &str, toxic: u8, text: &str) -> TextDetoxSourceRow {
    TextDetoxSourceRow {
        source_id: source_id.to_owned(),
        language: TextDetoxLanguage::English,
        label: if toxic == 0 {
            EvalLabel::Clean
        } else {
            EvalLabel::Toxic
        },
        text: text.to_owned(),
    }
}

#[test]
fn malay_split_hashing_uses_the_frozen_storage_code() {
    let text = "contoh teks untuk pembagian";
    let malay = split_hash(Language::Ms, text);
    let mut expected = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"ID\0".iter().chain(text.as_bytes()) {
        expected = (expected ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    assert_eq!(malay, expected, "Malay must hash the ID storage code");
}

#[test]
fn a_corpus_file_round_trips_through_parse_and_write() {
    use blasphem::EvalLabel;
    use blasphem_train::corpus::{CorpusRow, parse_corpus, write_corpus};
    use blasphem_train::datasets::DatasetSplit;

    let rows = vec![
        CorpusRow {
            split: DatasetSplit::Development,
            label: EvalLabel::Clean,
            text: "una linea\tcon tabulador".to_string(),
        },
        CorpusRow {
            split: DatasetSplit::Test,
            label: EvalLabel::Toxic,
            text: "plain".to_string(),
        },
    ];

    let mut buffer = Vec::new();
    write_corpus(&mut buffer, &rows).unwrap();
    let parsed = parse_corpus(buffer.as_slice()).unwrap();

    assert_eq!(parsed, rows);
    assert_eq!(buffer.iter().filter(|byte| **byte == b'\n').count(), 3);
}

#[test]
fn loading_a_corpus_language_splits_development_from_validation() {
    use blasphem::{EvalLabel, Language};
    use blasphem_train::corpus::{CorpusRow, load_corpus_language, write_corpus};
    use blasphem_train::datasets::DatasetSplit;

    let directory = tempfile::tempdir().unwrap();
    let rows = vec![
        CorpusRow {
            split: DatasetSplit::Development,
            label: EvalLabel::Clean,
            text: "one".to_string(),
        },
        CorpusRow {
            split: DatasetSplit::Validation,
            label: EvalLabel::Toxic,
            text: "two".to_string(),
        },
    ];
    let file = std::fs::File::create(directory.path().join("EN.tsv")).unwrap();
    write_corpus(file, &rows).unwrap();

    let bytes = std::fs::read("../../crates/blasphem-train/metadata/source-lock-v1.json").unwrap();
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(bytes.as_slice()).unwrap();
    let loaded = load_corpus_language(directory.path(), Language::En, &lock).unwrap();

    assert_eq!(loaded.development.len(), 1);
    assert_eq!(loaded.validation.len(), 1);
    assert_eq!(loaded.development[0].text, "one");
}
