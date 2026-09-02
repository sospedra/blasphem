use std::io::Cursor;

use toxcheck::{EvalLabel, EvalRow};
use toxtrain::{parse_eval_rows, write_textdetox_eval_tsv};

#[test]
fn parses_labeled_multilingual_messages() {
    let input = concat!(
        "language\tlabel\ttext\n",
        "es\ttoxic\teres un idiota\n",
        "fr\tclean\tbonjour mon ami\n",
    );

    let rows = parse_eval_rows(Cursor::new(input)).expect("valid evaluation data");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].language, "ES");
    assert_eq!(rows[0].label, EvalLabel::Toxic);
    assert_eq!(rows[1].label, EvalLabel::Clean);
}

#[test]
fn evaluation_tsv_round_trips_named_labels_and_exact_text() {
    let rows = vec![
        EvalRow {
            language: "EN".to_owned(),
            label: EvalLabel::Clean,
            text: "  exact\ttext  ".to_owned(),
        },
        EvalRow {
            language: "ES".to_owned(),
            label: EvalLabel::Toxic,
            text: "two\nlines".to_owned(),
        },
    ];
    let mut output = Vec::new();

    write_textdetox_eval_tsv(&mut output, &rows).expect("evaluation TSV");

    let text = String::from_utf8(output.clone()).expect("UTF-8 TSV");
    assert!(text.starts_with("language\tlabel\ttext\n"));
    let parsed = parse_eval_rows(Cursor::new(output)).expect("round-trip rows");
    assert_eq!(parsed, rows);
}
