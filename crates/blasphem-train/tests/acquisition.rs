use std::{
    collections::VecDeque,
    io::{Cursor, Write},
};

use blasphem_train::{
    TextDetoxAcquisitionError, TextDetoxHttpClient, TextDetoxHttpResponse, TextDetoxTransportError,
    acquire_textdetox,
    acquisition::{MAX_ARCHIVE_MEMBER_BYTES, extract_archive_member},
};

#[test]
fn archive_extraction_reads_one_exact_member() {
    let archive = archive_with(&[("folder/data.tsv", b"exact bytes")]);

    let bytes = extract_archive_member(&archive, "folder/data.tsv").expect("member");

    assert_eq!(bytes, b"exact bytes");
}

#[test]
fn archive_extraction_rejects_a_missing_member() {
    let archive = archive_with(&[("folder/data.tsv", b"exact bytes")]);

    let error = extract_archive_member(&archive, "missing.tsv").expect_err("missing member");

    assert!(error.to_string().contains("missing"));
}

#[test]
fn archive_extraction_ignores_a_central_directory_signature_in_member_data() {
    let mut fake_central_directory = vec![0_u8; 54];
    fake_central_directory[..4].copy_from_slice(b"PK\x01\x02");
    fake_central_directory[28..30].copy_from_slice(&8_u16.to_le_bytes());
    fake_central_directory[46..54].copy_from_slice(b"data.tsv");
    let archive = archive_with(&[("data.tsv", fake_central_directory.as_slice())]);

    let bytes = extract_archive_member(&archive, "data.tsv").expect("one archive member");

    assert_eq!(bytes, fake_central_directory);
}

#[test]
fn archive_extraction_rejects_an_oversized_member_before_allocation() {
    let mut bytes = Vec::new();
    let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
    writer
        .start_file("large.tsv", zip::write::SimpleFileOptions::default())
        .expect("start member");
    writer
        .write_all(&vec![b'x'; MAX_ARCHIVE_MEMBER_BYTES + 1])
        .expect("write member");
    writer.finish().expect("finish archive");

    let error = extract_archive_member(&bytes, "large.tsv").expect_err("oversized member");

    assert!(error.to_string().contains("67108864"));
}

#[test]
fn archive_extraction_rejects_a_stream_overrun_after_an_in_range_declared_size() {
    let mut bytes = Vec::new();
    let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
    writer
        .start_file("large.tsv", zip::write::SimpleFileOptions::default())
        .expect("start member");
    writer
        .write_all(&vec![b'x'; MAX_ARCHIVE_MEMBER_BYTES + 1])
        .expect("write member");
    writer.finish().expect("finish archive");
    patch_central_directory_uncompressed_size(&mut bytes, MAX_ARCHIVE_MEMBER_BYTES as u32);

    assert!(extract_archive_member(&bytes, "large.tsv").is_err());
}

#[test]
fn acquisition_rejects_a_page_without_a_revision_header() {
    let mut client = FakeClient::new([revision_response("rev-a"), page_response(None, 0, 1, 1)]);

    let error = acquire_textdetox(&mut client, &["en".to_owned()], Some(1))
        .expect_err("missing page revision");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::MissingPageRevision { language, offset }
            if language == "en" && offset == 0
    ));
}

#[test]
fn acquisition_rejects_a_mismatched_page_revision() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-b"), 0, 1, 1),
    ]);

    let error = acquire_textdetox(&mut client, &["en".to_owned()], Some(1))
        .expect_err("mismatched page revision");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::PageRevisionMismatch {
            expected,
            actual,
            language,
            offset,
        } if expected == "rev-a" && actual == "rev-b" && language == "en" && offset == 0
    ));
}

#[test]
fn acquisition_rejects_a_changed_final_revision() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 1, 1),
        revision_response("rev-b"),
    ]);

    let error = acquire_textdetox(&mut client, &["en".to_owned()], Some(1))
        .expect_err("changed final revision");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::FinalRevisionMismatch { expected, actual }
            if expected == "rev-a" && actual == "rev-b"
    ));
}

#[test]
fn acquisition_rejects_an_empty_page_before_the_expected_end() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 0, 1),
    ]);

    let error =
        acquire_textdetox(&mut client, &["en".to_owned()], Some(1)).expect_err("empty page");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::EmptyPage { language, offset }
            if language == "en" && offset == 0
    ));
}

#[test]
fn acquisition_rejects_a_short_page_before_the_expected_end() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 2, 3),
        revision_response("rev-a"),
    ]);

    let error =
        acquire_textdetox(&mut client, &["en".to_owned()], Some(3)).expect_err("short page");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::ShortPage {
            language,
            offset,
            expected: 3,
            actual: 2,
        } if language == "en" && offset == 0
    ));
}

#[test]
fn acquisition_rejects_a_repeated_row_on_a_later_page() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 100, 101),
        page_response(Some("rev-a"), 99, 1, 101),
    ]);

    let error =
        acquire_textdetox(&mut client, &["en".to_owned()], None).expect_err("repeated row index");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::NonContiguousRow {
            language,
            expected: 100,
            actual: 99,
        } if language == "en"
    ));
}

#[test]
fn acquisition_rejects_a_total_change_on_a_later_page() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 100, 101),
        page_response(Some("rev-a"), 100, 1, 102),
    ]);

    let error =
        acquire_textdetox(&mut client, &["en".to_owned()], None).expect_err("changed source total");

    assert!(matches!(
        error,
        TextDetoxAcquisitionError::TotalChanged {
            language,
            offset: 100,
            expected: 101,
            actual: 102,
        } if language == "en"
    ));
}

fn archive_with(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
    for (name, content) in files {
        writer
            .start_file(
                *name,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .expect("start member");
        writer.write_all(content).expect("write member");
    }
    writer.finish().expect("finish archive");
    bytes
}

fn patch_central_directory_uncompressed_size(bytes: &mut [u8], size: u32) {
    let signature = b"PK\x01\x02";
    let offset = bytes
        .windows(signature.len())
        .rposition(|window| window == signature)
        .expect("central directory");
    bytes[offset + 24..offset + 28].copy_from_slice(&size.to_le_bytes());
}

struct FakeClient {
    responses: VecDeque<TextDetoxHttpResponse>,
}

impl FakeClient {
    fn new(responses: impl IntoIterator<Item = TextDetoxHttpResponse>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
        }
    }
}

impl TextDetoxHttpClient for FakeClient {
    fn get(&mut self, _url: &str) -> Result<TextDetoxHttpResponse, TextDetoxTransportError> {
        self.responses
            .pop_front()
            .ok_or_else(|| TextDetoxTransportError::new("unexpected request"))
    }
}

fn revision_response(revision: &str) -> TextDetoxHttpResponse {
    TextDetoxHttpResponse {
        revision: None,
        body: format!(r#"{{"sha":"{revision}"}}"#).into_bytes(),
    }
}

fn page_response(
    revision: Option<&str>,
    row_index: usize,
    row_count: usize,
    total: usize,
) -> TextDetoxHttpResponse {
    let rows = (0..row_count)
        .map(|index| {
            format!(
                r#"{{"row_idx":{},"row":{{"text":"message","toxic":0}}}}"#,
                row_index + index
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    TextDetoxHttpResponse {
        revision: revision.map(str::to_owned),
        body: format!(r#"{{"rows":[{rows}],"num_rows_total":{total}}}"#).into_bytes(),
    }
}
