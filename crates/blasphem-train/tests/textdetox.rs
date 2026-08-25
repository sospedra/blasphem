use std::{io::Cursor, sync::Arc};

use blasphem::EvalLabel;
use blasphem_train::{
    TextDetoxError, TextDetoxLanguage, TextDetoxParquetLimits, TextDetoxSourceRow,
    parse_textdetox_page, parse_textdetox_parquet, parse_textdetox_parquet_with_limits,
    parse_textdetox_rows, textdetox_rows_url, write_textdetox_source_tsv,
};
use parquet::{
    basic::Compression,
    data_type::{ByteArray, ByteArrayType, Int64Type},
    file::properties::WriterProperties,
    file::writer::SerializedFileWriter,
    schema::parser::parse_message_type,
};

#[test]
fn rejects_parquet_metadata_beyond_the_row_limit_before_row_allocation() {
    let bytes = parquet_fixture(&["one", "two"], &[0, 1]);
    let limits = TextDetoxParquetLimits {
        max_rows: 1,
        max_text_bytes: 1024,
    };

    let error = parse_textdetox_parquet_with_limits(&bytes, "en", "revision", limits)
        .expect_err("row metadata limit");

    assert!(matches!(error, TextDetoxError::ParquetRowLimit { .. }));
}

#[test]
fn rejects_compressed_text_expansion_beyond_the_byte_limit() {
    let expanded = "repeated text ".repeat(1024);
    let bytes = parquet_fixture(&[expanded.as_str()], &[1]);
    let limits = TextDetoxParquetLimits {
        max_rows: 10,
        max_text_bytes: 64,
    };

    let error = parse_textdetox_parquet_with_limits(&bytes, "en", "revision", limits)
        .expect_err("text byte limit");

    assert!(matches!(error, TextDetoxError::ParquetTextByteLimit { .. }));
}

#[test]
fn rejects_malformed_parquet_footer_metadata() {
    let mut bytes = parquet_fixture(&["message"], &[0]);
    let footer_length = bytes.len();
    bytes[footer_length - 8..footer_length - 4].copy_from_slice(&u32::MAX.to_le_bytes());

    let error =
        parse_textdetox_parquet(&bytes, "en", "revision").expect_err("malformed footer metadata");

    assert!(matches!(error, TextDetoxError::Parquet(_)));
}

#[test]
fn parses_a_pinned_parquet_file_into_stable_source_rows() {
    let bytes = parquet_fixture(&["exact text", "威胁"], &[0, 1]);

    let rows = parse_textdetox_parquet(&bytes, "zh", "abc123").expect("valid Parquet");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].source_id, "textdetox@abc123/zh/000000");
    assert_eq!(rows[0].label, EvalLabel::Clean);
    assert_eq!(rows[0].text, "exact text");
    assert_eq!(rows[1].source_id, "textdetox@abc123/zh/000001");
    assert_eq!(rows[1].label, EvalLabel::Toxic);
    assert_eq!(rows[1].text, "威胁");

    let mut canonical_tsv = Vec::new();
    write_textdetox_source_tsv(&mut canonical_tsv, &rows).expect("canonical TSV");
    assert_eq!(
        String::from_utf8(canonical_tsv).expect("UTF-8 TSV"),
        concat!(
            "source_id\tlanguage\ttoxic\ttext\n",
            "textdetox@abc123/zh/000000\tzh\t0\texact text\n",
            "textdetox@abc123/zh/000001\tzh\t1\t威胁\n",
        )
    );
}

#[test]
fn keeps_hindi_and_hinglish_separate() {
    let input = concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "textdetox@rev/hi/1\thi\t0\tनमस्ते\n",
        "textdetox@rev/hin/1\thin\t1\ttu idiot hai\n",
    );

    let rows = parse_textdetox_rows(Cursor::new(input)).expect("valid rows");

    assert_eq!(rows[0].language.detector_code(), "HI");
    assert_eq!(rows[1].language.detector_code(), "HINGLISH");
}

#[test]
fn maps_all_supported_source_languages_and_rejects_unknown_codes() {
    let source_codes = [
        "am", "ar", "de", "en", "es", "fr", "he", "hi", "hin", "it", "ja", "ru", "tt", "uk", "zh",
    ];
    let detector_codes = [
        "AM", "AR", "DE", "EN", "ES", "FR", "HE", "HI", "HINGLISH", "IT", "JA", "RU", "TT", "UK",
        "ZH",
    ];
    let input = format!(
        "source_id\tlanguage\ttoxic\ttext\n{}",
        source_codes
            .iter()
            .enumerate()
            .map(|(index, language)| format!("{index}\t{language}\t0\tmessage {index}\n"))
            .collect::<String>()
    );

    let rows = parse_textdetox_rows(Cursor::new(input)).expect("supported languages");
    let error = parse_textdetox_rows(Cursor::new(concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "a\tpt\t0\thello\n",
    )))
    .expect_err("unknown source language");

    assert_eq!(
        rows.iter()
            .map(|row| row.language.source_code())
            .collect::<Vec<_>>(),
        source_codes
    );
    assert_eq!(
        rows.iter()
            .map(|row| row.language.detector_code())
            .collect::<Vec<_>>(),
        detector_codes
    );
    assert!(matches!(
        error,
        TextDetoxError::InvalidLanguage(language) if language == "pt"
    ));
}

#[test]
fn rejects_a_noncanonical_uppercase_source_language() {
    let error = parse_textdetox_rows(Cursor::new(concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "a\tEN\t0\thello\n",
    )))
    .expect_err("uppercase source language");

    assert!(matches!(
        error,
        TextDetoxError::InvalidLanguage(language) if language == "EN"
    ));
}

#[test]
fn rejects_a_source_label_outside_zero_and_one() {
    let error = parse_textdetox_rows(Cursor::new(concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "a\ten\t2\thello\n",
    )))
    .expect_err("invalid label");

    assert!(matches!(
        error,
        TextDetoxError::InvalidLabel(label) if label == "2"
    ));
}

#[test]
fn rejects_a_blank_source_id() {
    let input = concat!("source_id\tlanguage\ttoxic\ttext\n", " \ten\t0\thello\n",);

    let error = parse_textdetox_rows(Cursor::new(input)).expect_err("blank source ID");

    assert!(matches!(error, TextDetoxError::BlankSourceId));
}

#[test]
fn rejects_a_duplicate_source_id() {
    let input = concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "same\ten\t0\tfirst\n",
        "same\tes\t1\tsecond\n",
    );

    let error = parse_textdetox_rows(Cursor::new(input)).expect_err("duplicate source ID");

    assert!(matches!(
        error,
        TextDetoxError::DuplicateSourceId(source_id) if source_id == "same"
    ));
}

#[test]
fn parses_a_rows_api_page_with_revision_source_ids() {
    let input = concat!(
        "{\"rows\":[{\"row_idx\":7,\"row\":{\"text\":\"hello\",\"toxic\":0}}],",
        "\"num_rows_total\":5000,\"num_rows_per_page\":100,\"partial\":false}"
    );

    let page = parse_textdetox_page(Cursor::new(input), "en", "abc123").expect("valid page");

    assert_eq!(page.rows[0].source_id, "textdetox@abc123/en/000007");
    assert_eq!(page.rows[0].text, "hello");
    assert_eq!(page.total_rows, 5000);
}

#[test]
fn rejects_invalid_rows_api_page_identity_fields() {
    let valid_row = concat!(
        "{\"rows\":[{\"row_idx\":0,\"row\":{\"text\":\"hello\",\"toxic\":0}}],",
        "\"num_rows_total\":1}"
    );
    let duplicate_index = concat!(
        "{\"rows\":[",
        "{\"row_idx\":0,\"row\":{\"text\":\"a\",\"toxic\":0}},",
        "{\"row_idx\":0,\"row\":{\"text\":\"b\",\"toxic\":1}}],",
        "\"num_rows_total\":2}"
    );
    let outside_total = concat!(
        "{\"rows\":[{\"row_idx\":1,\"row\":{\"text\":\"hello\",\"toxic\":0}}],",
        "\"num_rows_total\":1}"
    );

    assert!(matches!(
        parse_textdetox_page(Cursor::new(valid_row), "en", " "),
        Err(TextDetoxError::BlankRevision)
    ));
    assert!(matches!(
        parse_textdetox_page(Cursor::new(duplicate_index), "en", "rev"),
        Err(TextDetoxError::DuplicateRowIndex(0))
    ));
    assert!(matches!(
        parse_textdetox_page(Cursor::new(outside_total), "en", "rev"),
        Err(TextDetoxError::RowIndexOutOfBounds {
            row_index: 1,
            total_rows: 1
        })
    ));
}

#[test]
fn accepts_only_rows_api_page_lengths_from_one_through_one_hundred() {
    assert!(matches!(
        textdetox_rows_url("en", 0, 0),
        Err(TextDetoxError::InvalidPageLength(0))
    ));
    assert_eq!(
        textdetox_rows_url("en", 25, 100).expect("maximum page length"),
        concat!(
            "https://datasets-server.huggingface.co/rows?",
            "dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&",
            "split=en&offset=25&length=100"
        )
    );
    assert!(matches!(
        textdetox_rows_url("en", 0, 101),
        Err(TextDetoxError::InvalidPageLength(101))
    ));
}

#[test]
fn source_tsv_round_trips_numeric_labels_and_exact_text() {
    let rows = vec![
        source_row("clean", 0, "  exact\ttext  "),
        source_row("toxic", 1, "two\nlines"),
    ];
    let mut output = Vec::new();

    write_textdetox_source_tsv(&mut output, &rows).expect("source TSV");

    let text = String::from_utf8(output.clone()).expect("UTF-8 TSV");
    assert!(text.starts_with("source_id\tlanguage\ttoxic\ttext\n"));
    let parsed = parse_textdetox_rows(Cursor::new(output)).expect("round-trip rows");
    assert_eq!(parsed, rows);
}

fn source_row(source_id: &str, toxic: u8, text: &str) -> TextDetoxSourceRow {
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

fn parquet_fixture(texts: &[&str], labels: &[i64]) -> Vec<u8> {
    let schema = Arc::new(
        parse_message_type(concat!(
            "message schema {",
            " REQUIRED BYTE_ARRAY text (STRING);",
            " REQUIRED INT64 toxic;",
            " }"
        ))
        .expect("schema"),
    );
    let mut bytes = Vec::new();
    let properties = Arc::new(
        WriterProperties::builder()
            .set_compression(Compression::SNAPPY)
            .build(),
    );
    let mut writer =
        SerializedFileWriter::new(&mut bytes, schema, properties).expect("Parquet writer");
    let mut row_group = writer.next_row_group().expect("row group");
    let mut text_writer = row_group
        .next_column()
        .expect("text column")
        .expect("text column exists");
    let text_values = texts
        .iter()
        .map(|text| ByteArray::from(*text))
        .collect::<Vec<_>>();
    text_writer
        .typed::<ByteArrayType>()
        .write_batch(&text_values, None, None)
        .expect("write text");
    text_writer.close().expect("close text");
    let mut label_writer = row_group
        .next_column()
        .expect("label column")
        .expect("label column exists");
    label_writer
        .typed::<Int64Type>()
        .write_batch(labels, None, None)
        .expect("write labels");
    label_writer.close().expect("close labels");
    row_group.close().expect("close row group");
    writer.close().expect("close Parquet");
    bytes
}
