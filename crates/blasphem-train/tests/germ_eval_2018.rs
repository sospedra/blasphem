use std::io::Cursor;

use blasphem::{EvalLabel, Language};
use blasphem_train::datasets::{
    DatasetAdapter, DatasetId, RowDisposition, SourceInput, SourceSplit,
    germ_eval_2018::GermEval2018Adapter,
};

const REVISION: &str = "9877472d39523effd54cd079b4c61157ed141508";

#[test]
fn importer_maps_both_pinned_files_as_one_unsplit_corpus() {
    let mut training = Cursor::new(include_bytes!(
        "fixtures/germ_eval_2018/germeval2018.training.txt"
    ));
    let mut test = Cursor::new(include_bytes!(
        "fixtures/germ_eval_2018/germeval2018.test.txt"
    ));
    let mut inputs = [
        SourceInput {
            source_file_id: "germeval-2018-training",
            source_split: SourceSplit::Unsplit,
            reader: &mut training,
        },
        SourceInput {
            source_file_id: "germeval-2018-test",
            source_split: SourceSplit::Unsplit,
            reader: &mut test,
        },
    ];

    let rows = GermEval2018Adapter.import(&mut inputs).expect("import");

    assert_eq!(GermEval2018Adapter.dataset_id(), DatasetId::GermEval2018);
    assert_eq!(
        GermEval2018Adapter.label_conversion_version(),
        "germeval-2018-coarse-v1"
    );
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows[0].source_id,
        format!("germeval-2018@{REVISION}/unsplit/training-000000")
    );
    assert_eq!(rows[0].source_file_id, "germeval-2018-training");
    assert_eq!(rows[0].source_language_code, "de");
    assert_eq!(rows[0].detector_language, Some(Language::De));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("DE"));
    assert_eq!(rows[0].source_label, "OTHER/OTHER");
    assert_eq!(rows[0].text, "\"Das Anführungszeichen bleibt wörtlich");
    assert_eq!(rows[0].source_split, SourceSplit::Unsplit);
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
    assert_eq!(
        rows[1].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert_eq!(rows[1].source_label, "OFFENSE/INSULT");
    assert_eq!(
        rows[2].source_id,
        format!("germeval-2018@{REVISION}/unsplit/test-000000")
    );
    assert_eq!(rows[2].source_file_id, "germeval-2018-test");
    assert_eq!(rows[2].source_label, "OFFENSE/PROFANITY");
    assert_eq!(rows[3].source_label, "OFFENSE/ABUSE");
}

#[test]
fn importer_rejects_unknown_labels_and_invalid_coarse_fine_pairs() {
    for row in [
        "Text\tOTHER\tINSULT\n",
        "Text\tOFFENSE\tOTHER\n",
        "Text\tOFFENSE\tHATE\n",
        "Text\tUNKNOWN\tOTHER\n",
    ] {
        assert!(import_training(row.as_bytes()).is_err(), "accepted {row:?}");
    }
}

#[test]
fn importer_requires_exactly_three_literal_tab_fields() {
    for row in [
        "Text\tOTHER\n",
        "Text\tOTHER\tOTHER\textra\n",
        "Text\twith\ttab\tOTHER\tOTHER\n",
    ] {
        assert!(import_training(row.as_bytes()).is_err(), "accepted {row:?}");
    }
}

#[test]
fn importer_rejects_blank_required_fields() {
    for row in [
        "\tOTHER\tOTHER\n",
        "   \tOTHER\tOTHER\n",
        "Text\t\tOTHER\n",
        "Text\tOTHER\t\n",
    ] {
        assert!(import_training(row.as_bytes()).is_err(), "accepted {row:?}");
    }
}

#[test]
fn importer_requires_both_named_unsplit_inputs() {
    let mut training = Cursor::new(b"Text\tOTHER\tOTHER\n");
    let mut test = Cursor::new(b"Text\tOTHER\tOTHER\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "germeval-2018-training",
            source_split: SourceSplit::Train,
            reader: &mut training,
        },
        SourceInput {
            source_file_id: "germeval-2018-test",
            source_split: SourceSplit::Unsplit,
            reader: &mut test,
        },
    ];

    assert!(GermEval2018Adapter.import(&mut inputs).is_err());
}

#[test]
fn importer_resolves_reversed_inputs_by_source_file_id() {
    let mut test = Cursor::new(b"Test text\tOFFENSE\tABUSE\n");
    let mut training = Cursor::new(b"Training text\tOTHER\tOTHER\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "germeval-2018-test",
            source_split: SourceSplit::Unsplit,
            reader: &mut test,
        },
        SourceInput {
            source_file_id: "germeval-2018-training",
            source_split: SourceSplit::Unsplit,
            reader: &mut training,
        },
    ];

    let rows = GermEval2018Adapter
        .import(&mut inputs)
        .expect("import reversed inputs");

    assert_eq!(
        rows.iter()
            .map(|row| row.source_file_id.as_str())
            .collect::<Vec<_>>(),
        ["germeval-2018-training", "germeval-2018-test"]
    );
}

#[test]
fn importer_rejects_an_empty_named_source() {
    for empty_source in ["germeval-2018-training", "germeval-2018-test"] {
        let mut training = Cursor::new(if empty_source == "germeval-2018-training" {
            b"".as_slice()
        } else {
            b"Training text\tOTHER\tOTHER\n".as_slice()
        });
        let mut test = Cursor::new(if empty_source == "germeval-2018-test" {
            b"".as_slice()
        } else {
            b"Test text\tOTHER\tOTHER\n".as_slice()
        });
        let mut inputs = [
            SourceInput {
                source_file_id: "germeval-2018-training",
                source_split: SourceSplit::Unsplit,
                reader: &mut training,
            },
            SourceInput {
                source_file_id: "germeval-2018-test",
                source_split: SourceSplit::Unsplit,
                reader: &mut test,
            },
        ];

        let error = GermEval2018Adapter
            .import(&mut inputs)
            .expect_err("empty source");

        assert!(error.to_string().contains(empty_source));
    }
}

#[test]
fn invalid_label_error_names_the_source_and_one_based_row() {
    let error = import_training(
        concat!(
            "Valid text\tOTHER\tOTHER\n",
            "Invalid text\tOFFENSE\tOTHER\n",
        )
        .as_bytes(),
    )
    .expect_err("invalid pair");

    assert!(error.to_string().contains("germeval-2018-training row 2"));
}

fn import_training(
    training: &[u8],
) -> Result<Vec<blasphem_train::datasets::ImportedRow>, blasphem_train::datasets::ImportError> {
    let mut training = Cursor::new(training);
    let mut test = Cursor::new(b"Test\tOTHER\tOTHER\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "germeval-2018-training",
            source_split: SourceSplit::Unsplit,
            reader: &mut training,
        },
        SourceInput {
            source_file_id: "germeval-2018-test",
            source_split: SourceSplit::Unsplit,
            reader: &mut test,
        },
    ];
    GermEval2018Adapter.import(&mut inputs)
}
