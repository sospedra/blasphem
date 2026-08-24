use std::io::Cursor;

use blasphem::{EvalLabel, Language};
use blasphem_train::datasets::{
    DatasetAdapter, ExclusionReason, RowDisposition, SourceInput, SourceSplit,
    ibrohim_budi::import_indonesian,
    kmhas::{KMHasAdapter, kmhas_label},
    offenseval_tr::OffensEvalTrAdapter,
    textdetox::{TEXTDETOX_CODES, TEXTDETOX_REVISION, TextDetoxAdapter},
    told_br::import_told_br,
    vihos::{ViHosAdapter, vihos_label},
};

#[test]
fn textdetox_importer_keeps_pinned_identity_and_raw_labels() {
    let mut reader = Cursor::new(include_bytes!("fixtures/textdetox.json"));
    let mut inputs = [SourceInput {
        source_file_id: "textdetox-en",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];

    let rows = TextDetoxAdapter.import(&mut inputs).expect("import");

    assert_eq!(
        TEXTDETOX_REVISION,
        "01907546324b0330d2d8b7669648cc18823323e5"
    );
    assert_eq!(
        TEXTDETOX_CODES,
        ["en", "zh", "es", "ar", "fr", "hi", "ru", "ja", "de", "it"]
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].source_id,
        "textdetox@01907546324b0330d2d8b7669648cc18823323e5/en/000007"
    );
    assert_eq!(rows[0].source_file_id, "textdetox-en");
    assert_eq!(rows[0].source_language_code, "en");
    assert_eq!(rows[0].detector_language, Some(Language::En));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("EN"));
    assert_eq!(rows[0].source_label, "0");
    assert_eq!(rows[0].text, "  exact text  ");
    assert_eq!(rows[0].source_split, SourceSplit::Unsplit);
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
    assert_eq!(rows[1].source_label, "1");
    assert_eq!(
        rows[1].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
}

#[test]
fn textdetox_rejects_hinglish_for_hindi() {
    let mut reader = Cursor::new(
        br#"{"rows":[{"row_idx":0,"row":{"text":"tu idiot hai","toxic":1}}],"num_rows_total":1}"#,
    );
    let mut inputs = [SourceInput {
        source_file_id: "textdetox-hin",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];

    let error = TextDetoxAdapter
        .import(&mut inputs)
        .expect_err("Hinglish must fail");

    assert!(error.to_string().contains("hin"));
}

#[test]
fn textdetox_importer_accepts_the_acquired_tsv_for_its_catalog_language() {
    let mut reader = Cursor::new(concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "textdetox@01907546324b0330d2d8b7669648cc18823323e5/en/000000\ten\t1\texact text\n",
    ));
    let mut inputs = [SourceInput {
        source_file_id: "textdetox-en",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];

    let rows = TextDetoxAdapter
        .import(&mut inputs)
        .expect("import acquired TSV");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source_language_code, "en");
    assert_eq!(rows[0].source_label, "1");
    assert_eq!(
        rows[0].source_id,
        "textdetox@01907546324b0330d2d8b7669648cc18823323e5/en/000000"
    );
}

#[test]
fn textdetox_importer_rejects_a_tsv_row_for_another_catalog_language() {
    let mut reader = Cursor::new(concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "textdetox@01907546324b0330d2d8b7669648cc18823323e5/fr/000000\tfr\t1\texact text\n",
    ));
    let mut inputs = [SourceInput {
        source_file_id: "textdetox-en",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];

    assert!(TextDetoxAdapter.import(&mut inputs).is_err());
}

#[test]
fn indonesian_uses_hate_or_abusive() {
    let rows = import_indonesian(include_bytes!("fixtures/ibrohim_budi.csv")).expect("import");

    assert_eq!(
        labels(&rows),
        [EvalLabel::Clean, EvalLabel::Toxic, EvalLabel::Toxic]
    );
    assert_eq!(
        rows[0].source_id,
        "ibrohim-budi@be98de98e974b65838d2b5145ee2c89e9bf53a6b/unsplit/000000"
    );
    assert_eq!(rows[0].source_file_id, "ibrohim-budi-re-dataset");
    assert_eq!(rows[0].source_language_code, "id");
    assert_eq!(rows[0].detector_language, Some(Language::Ms));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("ID"));
    assert_eq!(rows[0].source_label, "HS=0;Abusive=0");
    assert_eq!(rows[0].text, "hello");
    assert_eq!(rows[0].source_split, SourceSplit::Unsplit);
}

#[test]
fn indonesian_preserves_valid_utf8_and_replaces_only_damaged_tweet_bytes() {
    let mut csv = concat!(
        "Tweet,HS,Abusive,HS_Individual,HS_Group,HS_Religion,HS_Race,HS_Physical,",
        "HS_Gender,HS_Other,HS_Weak,HS_Moderate,HS_Strong\n",
        "pesan عربي "
    )
    .as_bytes()
    .to_vec();
    csv.extend_from_slice(&[0xf0, b'?', b'?', 0xad]);
    csv.extend_from_slice(" selesai,0,0,0,0,0,0,0,0,0,0,0,0\n".as_bytes());

    let rows = import_indonesian(csv).expect("import damaged tweet bytes");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].text, "pesan عربي �??� selesai");
    assert_eq!(rows[0].source_label, "HS=0;Abusive=0");
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
}

#[test]
fn indonesian_rejects_a_non_binary_label() {
    let csv = concat!(
        "Tweet,HS,Abusive,HS_Individual,HS_Group,HS_Religion,HS_Race,HS_Physical,HS_Gender,HS_Other,HS_Weak,HS_Moderate,HS_Strong\n",
        "bad,2,0,0,0,0,0,0,0,0,0,0,0\n",
    );

    assert!(import_indonesian(csv.as_bytes()).is_err());
}

#[test]
fn indonesian_rejects_non_binary_values_in_every_label_column() {
    let header = concat!(
        "Tweet,HS,Abusive,HS_Individual,HS_Group,HS_Religion,HS_Race,HS_Physical,",
        "HS_Gender,HS_Other,HS_Weak,HS_Moderate,HS_Strong",
    );

    for label_column in 1..=12 {
        let mut fields = vec!["0"; 13];
        fields[0] = "message";
        fields[label_column] = "2";
        let csv = format!("{header}\n{}\n", fields.join(","));

        assert!(
            import_indonesian(csv.as_bytes()).is_err(),
            "label column {label_column} accepted a non-binary value"
        );
    }
}

#[test]
fn portuguese_counts_toxic_annotators_across_categories() {
    let rows = import_told_br(include_bytes!("fixtures/told_br_alpha.csv")).expect("import");

    assert_eq!(
        dispositions(&rows),
        [
            RowDisposition::Candidate(EvalLabel::Clean),
            RowDisposition::Candidate(EvalLabel::Toxic),
            RowDisposition::Excluded(ExclusionReason::AmbiguousLabel),
        ]
    );
    assert_eq!(
        rows[0].source_id,
        "told-br@6b325d26a9d25b321a3e9ba98ef98832b56729f5/unsplit/000000"
    );
    assert_eq!(rows[0].source_file_id, "told-br-alpha");
    assert_eq!(rows[0].source_language_code, "pt-BR");
    assert_eq!(rows[0].detector_language, Some(Language::Pt));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("PT"));
    assert_eq!(rows[0].source_label, "toxic_annotator_votes=0");
    assert_eq!(rows[1].source_label, "toxic_annotator_votes=2");
    assert_eq!(rows[2].source_label, "toxic_annotator_votes=1");
    assert_eq!(rows[2].text, "one vote");
    assert_eq!(rows[2].source_split, SourceSplit::Unsplit);
}

#[test]
fn portuguese_accepts_mixed_integer_and_decimal_binary_values() {
    let csv = concat!(
        "text,homophobia_1,homophobia_2,homophobia_3,obscene_1,obscene_2,obscene_3,insult_1,insult_2,insult_3,racism_1,racism_2,racism_3,misogyny_1,misogyny_2,misogyny_3,xenophobia_1,xenophobia_2,xenophobia_3,obs_1,obs_2,obs_3\n",
        "mixed,1,0.0,0,0.0,1,0.0,0,0.0,0,0.0,0,0.0,0,0.0,0,0.0,0,0.0,0.0,0.0,0.0\n",
    );

    let rows = import_told_br(csv.as_bytes()).expect("import mixed binary values");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source_label, "toxic_annotator_votes=2");
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
}

#[test]
fn portuguese_rejects_non_binary_category_values() {
    let header = concat!(
        "text,homophobia_1,homophobia_2,homophobia_3,obscene_1,obscene_2,obscene_3,",
        "insult_1,insult_2,insult_3,racism_1,racism_2,racism_3,misogyny_1,",
        "misogyny_2,misogyny_3,xenophobia_1,xenophobia_2,xenophobia_3,obs_1,obs_2,obs_3",
    );

    for invalid_value in ["", "2", "2.0", "0.5", "01", "1.00"] {
        for label_column in 1..=18 {
            let mut fields = vec!["0.0"; 22];
            fields[0] = "bad";
            fields[label_column] = invalid_value;
            let csv = format!("{header}\n{}\n", fields.join(","));

            assert!(
                import_told_br(csv.as_bytes()).is_err(),
                "label column {label_column} accepted {invalid_value:?}"
            );
        }
    }
}

#[test]
fn turkish_joins_official_test_labels_and_preserves_source_splits() {
    let mut training = Cursor::new(include_bytes!(
        "fixtures/offenseval_tr/offenseval-tr-training-v1.tsv"
    ));
    let mut test = Cursor::new(include_bytes!(
        "fixtures/offenseval_tr/offenseval-tr-testset-v1.tsv"
    ));
    let mut labels = Cursor::new(include_bytes!(
        "fixtures/offenseval_tr/offenseval-tr-labela-v1.tsv"
    ));
    let mut inputs = [
        SourceInput {
            source_file_id: "offenseval-tr-training",
            source_split: SourceSplit::Train,
            reader: &mut training,
        },
        SourceInput {
            source_file_id: "offenseval-tr-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
        SourceInput {
            source_file_id: "offenseval-tr-test-labels",
            source_split: SourceSplit::Test,
            reader: &mut labels,
        },
    ];

    let rows = OffensEvalTrAdapter.import(&mut inputs).expect("import");

    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].source_id, "offenseval-tr@official-v1/train/1");
    assert_eq!(rows[0].source_file_id, "offenseval-tr-training");
    assert_eq!(rows[0].source_language_code, "tr");
    assert_eq!(rows[0].detector_language, Some(Language::Tr));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("TR"));
    assert_eq!(rows[0].source_label, "OFF");
    assert_eq!(rows[0].text, "  tam metin  ");
    assert_eq!(rows[0].source_split, SourceSplit::Train);
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert_eq!(rows[2].source_id, "offenseval-tr@official-v1/test/3");
    assert_eq!(rows[2].source_file_id, "offenseval-tr-test");
    assert_eq!(rows[2].source_label, "NOT");
    assert_eq!(rows[2].source_split, SourceSplit::Test);
    assert_eq!(
        rows[2].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
}

#[test]
fn turkish_treats_an_unmatched_tweet_quote_as_literal_data() {
    let training = b"id\ttweet\tsubtask_a\n1\ttraining text\tNOT\n";
    let test =
        include_bytes!("fixtures/offenseval_tr/offenseval-tr-testset-unmatched-literal-quote.tsv");
    let labels = b"2,OFF\n3,NOT\n";

    let rows = import_turkish(training, test, labels).expect("import literal tweet quote");

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].source_id, "offenseval-tr@official-v1/test/2");
    assert_eq!(rows[1].text, "\"ilk alıntı");
    assert_eq!(rows[2].source_id, "offenseval-tr@official-v1/test/3");
    assert_eq!(rows[2].text, "sonraki kayıt");
}

#[test]
fn turkish_rejects_missing_unused_duplicate_and_malformed_join_rows() {
    let training = b"id\ttweet\tsubtask_a\n1\ttext\tOFF\n";
    let test = b"id\ttweet\n2\ttext\n";
    let missing = b"3,OFF\n";
    assert!(import_turkish(training, test, missing).is_err());

    let unused = b"2,OFF\n3,NOT\n";
    assert!(import_turkish(training, test, unused).is_err());

    let duplicate_text = b"id\ttweet\n2\ttext\n2\tother\n";
    let label = b"2,OFF\n";
    assert!(import_turkish(training, duplicate_text, label).is_err());

    let duplicate_label = b"2,OFF\n2,NOT\n";
    assert!(import_turkish(training, test, duplicate_label).is_err());

    let malformed_label = b"2,BAD\n";
    assert!(import_turkish(training, test, malformed_label).is_err());

    let bad_header = b"id\ttext\n1\ttext\n";
    assert!(import_turkish(bad_header, test, label).is_err());
}

#[test]
fn vietnamese_preserves_text_and_official_splits() {
    let mut train = Cursor::new(include_bytes!("fixtures/vihos/train.csv"));
    let mut development = Cursor::new(include_bytes!("fixtures/vihos/dev.csv"));
    let mut test = Cursor::new(include_bytes!("fixtures/vihos/test.csv"));
    let mut inputs = [
        SourceInput {
            source_file_id: "vihos-train",
            source_split: SourceSplit::Train,
            reader: &mut train,
        },
        SourceInput {
            source_file_id: "vihos-development",
            source_split: SourceSplit::Development,
            reader: &mut development,
        },
        SourceInput {
            source_file_id: "vihos-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
    ];

    let rows = ViHosAdapter.import(&mut inputs).expect("import");

    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0].source_id,
        "vihos@fe31c4b304650d62bb0cb668e2fb2060fc6f98fd/train/0"
    );
    assert_eq!(rows[0].source_language_code, "vi");
    assert_eq!(rows[0].detector_language, Some(Language::Vi));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("VI"));
    assert_eq!(rows[0].source_label, "has-span");
    assert_eq!(rows[0].text, "  tiếng ác  ");
    assert_eq!(rows[0].source_split, SourceSplit::Train);
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert_eq!(rows[1].source_label, "no-span");
    assert_eq!(rows[1].source_split, SourceSplit::Development);
    assert_eq!(
        rows[1].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
    assert_eq!(rows[2].source_split, SourceSplit::Test);
}

#[test]
fn vietnamese_rejects_invalid_spans_and_headers() {
    assert_eq!(
        vihos_label("[]", 10).unwrap(),
        RowDisposition::Candidate(EvalLabel::Clean)
    );
    assert_eq!(
        vihos_label("[1,2]", 10).unwrap(),
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert!(vihos_label("[12]", 10).is_err());
    assert!(vihos_label("[1,]", 10).is_err());

    let mut train = Cursor::new(b"id,content,index_spans\n0,text,[]\n");
    let mut development = Cursor::new(b",content,index_spans\n0,text,[]\n");
    let mut test = Cursor::new(b",content,index_spans\n0,text,[]\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "vihos-train",
            source_split: SourceSplit::Train,
            reader: &mut train,
        },
        SourceInput {
            source_file_id: "vihos-development",
            source_split: SourceSplit::Development,
            reader: &mut development,
        },
        SourceInput {
            source_file_id: "vihos-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
    ];
    assert!(ViHosAdapter.import(&mut inputs).is_err());

    let mut train = Cursor::new(",content,index_spans\n0,\"a\u{0301}\",[2]\n".as_bytes());
    let mut development = Cursor::new(b",content,index_spans\n0,text,[]\n");
    let mut test = Cursor::new(b",content,index_spans\n0,text,[]\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "vihos-train",
            source_split: SourceSplit::Train,
            reader: &mut train,
        },
        SourceInput {
            source_file_id: "vihos-development",
            source_split: SourceSplit::Development,
            reader: &mut development,
        },
        SourceInput {
            source_file_id: "vihos-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
    ];
    assert!(ViHosAdapter.import(&mut inputs).is_err());
}

#[test]
fn korean_requires_eight_alone_for_clean() {
    assert_eq!(
        kmhas_label(&[8]).unwrap(),
        RowDisposition::Candidate(EvalLabel::Clean)
    );
    assert_eq!(
        kmhas_label(&[0, 7]).unwrap(),
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert!(kmhas_label(&[8, 2]).is_err());
    assert!(kmhas_label(&[]).is_err());
    assert!(kmhas_label(&[9]).is_err());
}

#[test]
fn korean_preserves_official_splits_and_canonical_label_sets() {
    let mut train = Cursor::new(include_bytes!("fixtures/kmhas/kmhas_train.txt"));
    let mut validation = Cursor::new(include_bytes!("fixtures/kmhas/kmhas_valid.txt"));
    let mut test = Cursor::new(include_bytes!("fixtures/kmhas/kmhas_test.txt"));
    let mut inputs = [
        SourceInput {
            source_file_id: "kmhas-train",
            source_split: SourceSplit::Train,
            reader: &mut train,
        },
        SourceInput {
            source_file_id: "kmhas-validation",
            source_split: SourceSplit::Validation,
            reader: &mut validation,
        },
        SourceInput {
            source_file_id: "kmhas-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
    ];

    let rows = KMHasAdapter.import(&mut inputs).expect("import");

    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[0].source_id,
        "k-mhas@ec7a7e775d650b825872f6f538fc717822cdfc1a/train/0"
    );
    assert_eq!(rows[0].source_file_id, "kmhas-train");
    assert_eq!(rows[0].source_language_code, "ko");
    assert_eq!(rows[0].detector_language, Some(Language::Ko));
    assert_eq!(rows[0].detector_language_code.as_deref(), Some("KO"));
    assert_eq!(rows[0].source_label, "0,7");
    assert_eq!(rows[0].text, "  나쁜 말  ");
    assert_eq!(rows[0].source_split, SourceSplit::Train);
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert_eq!(rows[1].source_label, "8");
    assert_eq!(rows[1].source_split, SourceSplit::Validation);
    assert_eq!(
        rows[1].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
    assert_eq!(rows[2].source_split, SourceSplit::Test);
}

#[test]
fn korean_rejects_a_bad_header_or_label_set() {
    let mut train = Cursor::new(b"text\tlabel\nmessage\t8\n");
    let mut validation = Cursor::new(b"document\tlabel\nmessage\t8\n");
    let mut test = Cursor::new(b"document\tlabel\nmessage\t8\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "kmhas-train",
            source_split: SourceSplit::Train,
            reader: &mut train,
        },
        SourceInput {
            source_file_id: "kmhas-validation",
            source_split: SourceSplit::Validation,
            reader: &mut validation,
        },
        SourceInput {
            source_file_id: "kmhas-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
    ];
    assert!(KMHasAdapter.import(&mut inputs).is_err());

    let mut train = Cursor::new(b"document\tlabel\nmessage\t8,0\n");
    let mut validation = Cursor::new(b"document\tlabel\nmessage\t8\n");
    let mut test = Cursor::new(b"document\tlabel\nmessage\t8\n");
    let mut inputs = [
        SourceInput {
            source_file_id: "kmhas-train",
            source_split: SourceSplit::Train,
            reader: &mut train,
        },
        SourceInput {
            source_file_id: "kmhas-validation",
            source_split: SourceSplit::Validation,
            reader: &mut validation,
        },
        SourceInput {
            source_file_id: "kmhas-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
    ];
    assert!(KMHasAdapter.import(&mut inputs).is_err());
}

fn labels(rows: &[blasphem_train::datasets::ImportedRow]) -> Vec<EvalLabel> {
    rows.iter()
        .map(|row| match row.disposition {
            RowDisposition::Candidate(label) => label,
            RowDisposition::Excluded(reason) => panic!("unexpected exclusion: {reason:?}"),
        })
        .collect()
}

fn dispositions(rows: &[blasphem_train::datasets::ImportedRow]) -> Vec<RowDisposition> {
    rows.iter().map(|row| row.disposition).collect()
}

fn import_turkish(
    training: &[u8],
    test: &[u8],
    labels: &[u8],
) -> Result<Vec<blasphem_train::datasets::ImportedRow>, blasphem_train::datasets::ImportError> {
    let mut training = Cursor::new(training);
    let mut test = Cursor::new(test);
    let mut labels = Cursor::new(labels);
    let mut inputs = [
        SourceInput {
            source_file_id: "offenseval-tr-training",
            source_split: SourceSplit::Train,
            reader: &mut training,
        },
        SourceInput {
            source_file_id: "offenseval-tr-test",
            source_split: SourceSplit::Test,
            reader: &mut test,
        },
        SourceInput {
            source_file_id: "offenseval-tr-test-labels",
            source_split: SourceSplit::Test,
            reader: &mut labels,
        },
    ];
    OffensEvalTrAdapter.import(&mut inputs)
}
