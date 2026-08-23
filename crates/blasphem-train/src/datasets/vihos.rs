use std::{collections::BTreeSet, io::Read};

use blasphem::{EvalLabel, Language};

use super::{
    DatasetAdapter, DatasetId, ImportError, ImportedRow, RowDisposition, SourceInput, SourceSplit,
    source_id,
};

const VIHOS_REVISION: &str = "fe31c4b304650d62bb0cb668e2fb2060fc6f98fd";
const VIHOS_HEADER: [&str; 3] = ["", "content", "index_spans"];

pub struct ViHosAdapter;

impl DatasetAdapter for ViHosAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::ViHos
    }

    fn label_conversion_version(&self) -> &'static str {
        "vihos-span-presence-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let [train, development, test] = inputs else {
            return Err(ImportError::InvalidSource("vihos inputs".to_owned()));
        };
        if train.source_file_id != "vihos-train"
            || train.source_split != SourceSplit::Train
            || development.source_file_id != "vihos-development"
            || development.source_split != SourceSplit::Development
            || test.source_file_id != "vihos-test"
            || test.source_split != SourceSplit::Test
        {
            return Err(ImportError::InvalidSource("vihos inputs".to_owned()));
        }
        let mut output = import_split(&mut train.reader, train.source_file_id, SourceSplit::Train)?;
        output.extend(import_split(
            &mut development.reader,
            development.source_file_id,
            SourceSplit::Development,
        )?);
        output.extend(import_split(
            &mut test.reader,
            test.source_file_id,
            SourceSplit::Test,
        )?);
        Ok(output)
    }
}

fn import_split(
    reader: impl Read,
    source_file_id: &str,
    source_split: SourceSplit,
) -> Result<Vec<ImportedRow>, ImportError> {
    let mut csv = csv::ReaderBuilder::new().from_reader(reader);
    let header = csv.headers()?;
    if header.iter().ne(VIHOS_HEADER) {
        return Err(ImportError::InvalidSource("vihos header".to_owned()));
    }
    let mut ids = BTreeSet::new();
    let mut output = Vec::new();
    for record in csv.records() {
        let record = record?;
        let native_id = required_field(&record, 0, "vihos id")?.to_owned();
        if native_id.is_empty() || !ids.insert(native_id.clone()) {
            return Err(ImportError::InvalidSource(native_id));
        }
        let text = required_field(&record, 1, "vihos content")?.to_owned();
        let disposition = vihos_label(
            required_field(&record, 2, "vihos index_spans")?,
            text.chars().count(),
        )?;
        let source_label = match disposition {
            RowDisposition::Candidate(EvalLabel::Clean) => "no-span",
            RowDisposition::Candidate(EvalLabel::Toxic) => "has-span",
            RowDisposition::Excluded(_) => return Err(ImportError::InvalidSource(native_id)),
        };
        output.push(ImportedRow {
            dataset: DatasetId::ViHos,
            source_file_id: source_file_id.to_owned(),
            source_id: source_id(DatasetId::ViHos, VIHOS_REVISION, source_split, &native_id),
            source_language_code: "vi".to_owned(),
            detector_language: Some(Language::Vi),
            detector_language_code: Some(Language::Vi.code().to_owned()),
            source_label: source_label.to_owned(),
            text,
            source_split,
            disposition,
        });
    }
    Ok(output)
}

pub fn vihos_label(value: &str, scalar_count: usize) -> Result<RowDisposition, ImportError> {
    let spans: Vec<usize> =
        serde_json::from_str(value).map_err(|_| ImportError::InvalidSpan(value.to_owned()))?;
    if spans.iter().any(|&index| index >= scalar_count) {
        return Err(ImportError::InvalidSpan(value.to_owned()));
    }
    Ok(if spans.is_empty() {
        RowDisposition::Candidate(EvalLabel::Clean)
    } else {
        RowDisposition::Candidate(EvalLabel::Toxic)
    })
}

fn required_field<'a>(
    record: &'a csv::StringRecord,
    index: usize,
    field: &'static str,
) -> Result<&'a str, ImportError> {
    record.get(index).ok_or(ImportError::MissingColumn(field))
}
