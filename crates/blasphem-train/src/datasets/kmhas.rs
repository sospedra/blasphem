use std::io::Read;

use blasphem::{EvalLabel, Language};

use super::{
    DatasetAdapter, DatasetId, ImportError, ImportedRow, RowDisposition, SourceInput, SourceSplit,
    source_id,
};

const KMHAS_REVISION: &str = "ec7a7e775d650b825872f6f538fc717822cdfc1a";
const KMHAS_HEADER: [&str; 2] = ["document", "label"];

pub struct KMHasAdapter;

impl DatasetAdapter for KMHasAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::KMHas
    }

    fn label_conversion_version(&self) -> &'static str {
        "k-mhas-clean-8-toxic-0-7-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let [train, validation, test] = inputs else {
            return Err(ImportError::InvalidSource("k-mhas inputs".to_owned()));
        };
        if train.source_file_id != "kmhas-train"
            || train.source_split != SourceSplit::Train
            || validation.source_file_id != "kmhas-validation"
            || validation.source_split != SourceSplit::Validation
            || test.source_file_id != "kmhas-test"
            || test.source_split != SourceSplit::Test
        {
            return Err(ImportError::InvalidSource("k-mhas inputs".to_owned()));
        }
        let mut output = import_split(&mut train.reader, train.source_file_id, SourceSplit::Train)?;
        output.extend(import_split(
            &mut validation.reader,
            validation.source_file_id,
            SourceSplit::Validation,
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
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);
    let header = csv.headers()?;
    if header.iter().ne(KMHAS_HEADER) {
        return Err(ImportError::InvalidSource("k-mhas header".to_owned()));
    }
    let mut output = Vec::new();
    for (index, record) in csv.records().enumerate() {
        let record = record?;
        let native_id = index.to_string();
        let text = required_field(&record, 0, "k-mhas document")?.to_owned();
        let labels = parse_labels(required_field(&record, 1, "k-mhas label")?)?;
        let disposition = kmhas_label(&labels)?;
        output.push(ImportedRow {
            dataset: DatasetId::KMHas,
            source_file_id: source_file_id.to_owned(),
            source_id: source_id(DatasetId::KMHas, KMHAS_REVISION, source_split, &native_id),
            source_language_code: "ko".to_owned(),
            detector_language: Some(Language::Ko),
            detector_language_code: Some(Language::Ko.code().to_owned()),
            source_label: labels
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(","),
            text,
            source_split,
            disposition,
        });
    }
    Ok(output)
}

pub fn kmhas_label(labels: &[u8]) -> Result<RowDisposition, ImportError> {
    if labels.is_empty() || labels.iter().any(|&label| label > 8) {
        return Err(ImportError::InvalidKoreanLabel(format_labels(labels)));
    }
    if labels.contains(&8) {
        return if labels.iter().all(|&label| label == 8) {
            Ok(RowDisposition::Candidate(EvalLabel::Clean))
        } else {
            Err(ImportError::InvalidKoreanLabel(format_labels(labels)))
        };
    }
    if labels.iter().any(|&label| label <= 7) {
        Ok(RowDisposition::Candidate(EvalLabel::Toxic))
    } else {
        Err(ImportError::InvalidKoreanLabel(format_labels(labels)))
    }
}

fn parse_labels(value: &str) -> Result<Vec<u8>, ImportError> {
    if value.is_empty() {
        return Err(ImportError::InvalidKoreanLabel(value.to_owned()));
    }
    let mut labels = value
        .split(',')
        .map(|part| {
            part.parse::<u8>()
                .map_err(|_| ImportError::InvalidKoreanLabel(value.to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    labels.sort_unstable();
    labels.dedup();
    Ok(labels)
}

fn format_labels(labels: &[u8]) -> String {
    labels
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn required_field<'a>(
    record: &'a csv::StringRecord,
    index: usize,
    field: &'static str,
) -> Result<&'a str, ImportError> {
    record.get(index).ok_or(ImportError::MissingColumn(field))
}
