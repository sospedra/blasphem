use std::io::Read;

use blasphem::{EvalLabel, Language};

use super::{
    DatasetAdapter, DatasetId, ExclusionReason, ImportError, ImportedRow, RowDisposition,
    SourceInput, SourceSplit, source_id,
};

const TOLD_BR_REVISION: &str = "6b325d26a9d25b321a3e9ba98ef98832b56729f5";
const TOLD_BR_SOURCE_FILE_ID: &str = "told-br-alpha";
const TOLD_BR_HEADER: [&str; 22] = [
    "text",
    "homophobia_1",
    "homophobia_2",
    "homophobia_3",
    "obscene_1",
    "obscene_2",
    "obscene_3",
    "insult_1",
    "insult_2",
    "insult_3",
    "racism_1",
    "racism_2",
    "racism_3",
    "misogyny_1",
    "misogyny_2",
    "misogyny_3",
    "xenophobia_1",
    "xenophobia_2",
    "xenophobia_3",
    "obs_1",
    "obs_2",
    "obs_3",
];
const CATEGORIES: [&str; 6] = [
    "homophobia",
    "obscene",
    "insult",
    "racism",
    "misogyny",
    "xenophobia",
];

pub struct ToldBrAdapter;

impl DatasetAdapter for ToldBrAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::ToldBr
    }

    fn label_conversion_version(&self) -> &'static str {
        "told-br-annotator-consensus-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let [input] = inputs else {
            return Err(ImportError::InvalidSource("told-br inputs".to_owned()));
        };
        import_source(&mut input.reader, input.source_file_id, input.source_split)
    }
}

pub fn import_told_br(input: impl AsRef<[u8]>) -> Result<Vec<ImportedRow>, ImportError> {
    import_source(input.as_ref(), TOLD_BR_SOURCE_FILE_ID, SourceSplit::Unsplit)
}

fn import_source(
    reader: impl Read,
    source_file_id: &str,
    source_split: SourceSplit,
) -> Result<Vec<ImportedRow>, ImportError> {
    let mut csv = csv::ReaderBuilder::new().from_reader(reader);
    let header = csv.headers()?;
    if header.iter().ne(TOLD_BR_HEADER) {
        return Err(ImportError::InvalidSource("told-br header".to_owned()));
    }

    let mut output = Vec::new();
    for (index, record) in csv.records().enumerate() {
        let record = record?;
        let native_id = format!("{index:06}");
        let row_source_id = source_id(
            DatasetId::ToldBr,
            TOLD_BR_REVISION,
            source_split,
            &native_id,
        );
        let values = category_values(&record, &row_source_id)?;
        let toxic_votes = (1..=3)
            .filter(|annotator| {
                CATEGORIES
                    .iter()
                    .enumerate()
                    .any(|(category, _)| values[category][annotator - 1] == 1)
            })
            .count();
        let disposition = match toxic_votes {
            0 => RowDisposition::Candidate(EvalLabel::Clean),
            1 => RowDisposition::Excluded(ExclusionReason::AmbiguousLabel),
            2 | 3 => RowDisposition::Candidate(EvalLabel::Toxic),
            _ => return Err(ImportError::InvalidSource(row_source_id)),
        };
        output.push(ImportedRow {
            dataset: DatasetId::ToldBr,
            source_file_id: source_file_id.to_owned(),
            source_id: row_source_id,
            source_language_code: "pt-BR".to_owned(),
            detector_language: Some(Language::Pt),
            detector_language_code: Some(Language::Pt.code().to_owned()),
            source_label: format!("toxic_annotator_votes={toxic_votes}"),
            text: record
                .get(0)
                .ok_or_else(|| ImportError::InvalidSource(native_id.clone()))?
                .to_owned(),
            source_split,
            disposition,
        });
    }
    Ok(output)
}

fn category_values(
    record: &csv::StringRecord,
    source_id: &str,
) -> Result<[[u8; 3]; 6], ImportError> {
    let mut values = [[0; 3]; 6];
    for (category_index, category_values) in values.iter_mut().enumerate() {
        for annotator in 1..=3 {
            let column = 1 + category_index * 3 + (annotator - 1);
            let value = record
                .get(column)
                .ok_or_else(|| ImportError::InvalidSource(source_id.to_owned()))?;
            category_values[annotator - 1] = match value {
                "0" | "0.0" => 0,
                "1" | "1.0" => 1,
                _ => return Err(ImportError::InvalidBinaryLabel(source_id.to_owned())),
            };
        }
    }
    Ok(values)
}
