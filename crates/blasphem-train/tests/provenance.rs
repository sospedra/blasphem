use blasphem_train::{
    DatasetSplit, ProvenanceRow, ProvenanceStatus, write_textdetox_provenance_tsv,
};

#[test]
fn provenance_tsv_sorts_source_ids_and_writes_lowercase_values() {
    let rows = vec![
        provenance_row("z", ProvenanceStatus::Duplicate, Some("a")),
        provenance_row("a", ProvenanceStatus::Representative, Some("a")),
    ];
    let mut output = Vec::new();

    write_textdetox_provenance_tsv(&mut output, &rows).expect("provenance TSV");

    assert_eq!(
        String::from_utf8(output).expect("UTF-8 TSV"),
        concat!(
            "source_id\tsource_language\tdetector_language\tgroup_id\tsplit\t",
            "canonical_source_id\tstatus\n",
            "a\ten\tEN\tv1-a9e8e8eea9fd77d5\tdevelopment\ta\trepresentative\n",
            "z\ten\tEN\tv1-a9e8e8eea9fd77d5\tdevelopment\ta\tduplicate\n",
        )
    );
}

fn provenance_row(
    source_id: &str,
    status: ProvenanceStatus,
    canonical_source_id: Option<&str>,
) -> ProvenanceRow {
    ProvenanceRow {
        source_id: source_id.to_owned(),
        source_language: "en".to_owned(),
        detector_language: "EN".to_owned(),
        group_id: Some("v1-a9e8e8eea9fd77d5".to_owned()),
        split: Some(DatasetSplit::Development),
        canonical_source_id: canonical_source_id.map(str::to_owned),
        status,
    }
}
