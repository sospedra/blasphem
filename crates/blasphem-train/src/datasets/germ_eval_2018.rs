use std::io::Read;

use toxcheck::{EvalLabel, Language};

use super::{
    DatasetAdapter, DatasetId, ImportError, ImportedRow, RowDisposition, SourceInput, SourceSplit,
    source_id,
};

const GERMEVAL_2018_REVISION: &str = "9877472d39523effd54cd079b4c61157ed141508";
const TRAINING_SOURCE_FILE_ID: &str = "germeval-2018-training";
const TEST_SOURCE_FILE_ID: &str = "germeval-2018-test";

pub struct GermEval2018Adapter;

impl DatasetAdapter for GermEval2018Adapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::GermEval2018
    }

    fn label_conversion_version(&self) -> &'static str {
        "germeval-2018-coarse-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        if inputs.len() != 2 {
            return Err(ImportError::InvalidSource(
                "germeval-2018 inputs".to_owned(),
            ));
        }

        let mut training_rows = None;
        let mut test_rows = None;
        for input in inputs {
            if input.source_split != SourceSplit::Unsplit {
                return Err(ImportError::InvalidSource(input.source_file_id.to_owned()));
            }
            let (native_prefix, slot) = match input.source_file_id {
                TRAINING_SOURCE_FILE_ID => ("training", &mut training_rows),
                TEST_SOURCE_FILE_ID => ("test", &mut test_rows),
                _ => return Err(ImportError::InvalidSource(input.source_file_id.to_owned())),
            };
            if slot.is_some() {
                return Err(ImportError::InvalidSource(input.source_file_id.to_owned()));
            }
            let rows = import_source(&mut input.reader, input.source_file_id, native_prefix)?;
            if rows.is_empty() {
                return Err(ImportError::InvalidSource(format!(
                    "{} has zero rows",
                    input.source_file_id
                )));
            }
            *slot = Some(rows);
        }
        let mut output = training_rows
            .ok_or_else(|| ImportError::InvalidSource(TRAINING_SOURCE_FILE_ID.to_owned()))?;
        output.extend(
            test_rows.ok_or_else(|| ImportError::InvalidSource(TEST_SOURCE_FILE_ID.to_owned()))?,
        );
        Ok(output)
    }
}

fn import_source(
    reader: impl Read,
    source_file_id: &str,
    native_prefix: &str,
) -> Result<Vec<ImportedRow>, ImportError> {
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .has_headers(false)
        .quoting(false)
        .from_reader(reader);
    let mut output = Vec::new();
    for (index, record) in csv.records().enumerate() {
        let record = record?;
        if record.len() != 3 {
            return Err(ImportError::InvalidSource(format!(
                "{source_file_id} row {}",
                index + 1
            )));
        }
        let text = required_field(&record, 0, "germeval-2018 text")?.to_owned();
        let coarse = required_field(&record, 1, "germeval-2018 coarse label")?;
        let fine = required_field(&record, 2, "germeval-2018 fine label")?;
        let disposition = germ_eval_label(coarse, fine, source_file_id, index + 1)?;
        let native_id = format!("{native_prefix}-{index:06}");
        output.push(ImportedRow {
            dataset: DatasetId::GermEval2018,
            source_file_id: source_file_id.to_owned(),
            source_id: source_id(
                DatasetId::GermEval2018,
                GERMEVAL_2018_REVISION,
                SourceSplit::Unsplit,
                &native_id,
            ),
            source_language_code: "de".to_owned(),
            detector_language: Some(Language::De),
            detector_language_code: Some(Language::De.code().to_owned()),
            source_label: format!("{coarse}/{fine}"),
            text,
            source_split: SourceSplit::Unsplit,
            disposition,
        });
    }
    Ok(output)
}

fn germ_eval_label(
    coarse: &str,
    fine: &str,
    source_file_id: &str,
    row_number: usize,
) -> Result<RowDisposition, ImportError> {
    match (coarse, fine) {
        ("OTHER", "OTHER") => Ok(RowDisposition::Candidate(EvalLabel::Clean)),
        ("OFFENSE", "INSULT" | "ABUSE" | "PROFANITY") => {
            Ok(RowDisposition::Candidate(EvalLabel::Toxic))
        }
        _ => Err(ImportError::InvalidSource(format!(
            "{source_file_id} row {row_number} has invalid label {coarse}/{fine}"
        ))),
    }
}

fn required_field<'a>(
    record: &'a csv::StringRecord,
    index: usize,
    field: &'static str,
) -> Result<&'a str, ImportError> {
    record
        .get(index)
        .filter(|value| !value.trim().is_empty())
        .ok_or(ImportError::MissingColumn(field))
}
