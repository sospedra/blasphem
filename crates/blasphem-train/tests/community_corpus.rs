use std::{fs::File, path::PathBuf};

use blasphem::{EvalLabel, Language};
use blasphem_train::community_corpus::CommunityCorpusAdapter;
use blasphem_train::datasets::{DatasetAdapter, RowDisposition, SourceInput, SourceSplit};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/community")
        .join(name)
}

#[test]
fn the_canonical_schema_imports_three_rows() {
    let mut reader = File::open(fixture("valid.tsv")).expect("readable fixture");
    let adapter = CommunityCorpusAdapter::new(Language::Es, "community-es-demo");
    let mut inputs = vec![SourceInput {
        source_file_id: "community-es-demo",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];
    let rows = adapter.import(&mut inputs).expect("imports the fixture");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].source_id, "community-es-demo/row-000001");
    assert_eq!(
        rows[0].disposition,
        RowDisposition::Candidate(EvalLabel::Toxic)
    );
    assert_eq!(
        rows[1].disposition,
        RowDisposition::Candidate(EvalLabel::Clean)
    );
}

#[test]
fn an_invalid_label_names_the_source_and_the_row() {
    let mut reader = std::io::Cursor::new("native_id\tlabel\ttext\nrow-1\tmaybe\thola\n");
    let adapter = CommunityCorpusAdapter::new(Language::Es, "community-es-demo");
    let mut inputs = vec![SourceInput {
        source_file_id: "community-es-demo",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];
    let error = adapter.import(&mut inputs).expect_err("rejects the label");
    let message = error.to_string();
    assert!(message.contains("community-es-demo"), "{message}");
    assert!(message.contains("row-1"), "{message}");
}
