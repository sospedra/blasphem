use std::io::Read;

use blasphem::{EvalLabel, Language};

use super::{
    DatasetAdapter, DatasetId, ImportError, ImportedRow, RowDisposition, SourceInput, SourceSplit,
    source_id,
};

const IBROHIM_BUDI_REVISION: &str = "be98de98e974b65838d2b5145ee2c89e9bf53a6b";
const IBROHIM_BUDI_SOURCE_FILE_ID: &str = "ibrohim-budi-re-dataset";
const IBROHIM_BUDI_HEADER: [&[u8]; 13] = [
    b"Tweet",
    b"HS",
    b"Abusive",
    b"HS_Individual",
    b"HS_Group",
    b"HS_Religion",
    b"HS_Race",
    b"HS_Physical",
    b"HS_Gender",
    b"HS_Other",
    b"HS_Weak",
    b"HS_Moderate",
    b"HS_Strong",
];

pub struct IbrohimBudiAdapter;

impl DatasetAdapter for IbrohimBudiAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::IbrohimBudi
    }

    fn label_conversion_version(&self) -> &'static str {
        "ibrohim-budi-hs-or-abusive-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let [input] = inputs else {
            return Err(ImportError::InvalidSource("ibrohim-budi inputs".to_owned()));
        };
        import_source(&mut input.reader, input.source_file_id, input.source_split)
    }
}

pub fn import_indonesian(input: impl AsRef<[u8]>) -> Result<Vec<ImportedRow>, ImportError> {
    import_source(
        input.as_ref(),
        IBROHIM_BUDI_SOURCE_FILE_ID,
        SourceSplit::Unsplit,
    )
}

fn import_source(
    reader: impl Read,
    source_file_id: &str,
    source_split: SourceSplit,
) -> Result<Vec<ImportedRow>, ImportError> {
    let mut csv = csv::ReaderBuilder::new().from_reader(reader);
    let header = csv.byte_headers()?;
    if header.iter().ne(IBROHIM_BUDI_HEADER) {
        return Err(ImportError::InvalidSource("ibrohim-budi header".to_owned()));
    }

    let mut output = Vec::new();
    for (index, record) in csv.byte_records().enumerate() {
        let record = record?;
        let native_id = format!("{index:06}");
        let row_source_id = source_id(
            DatasetId::IbrohimBudi,
            IBROHIM_BUDI_REVISION,
            source_split,
            &native_id,
        );
        let labels = (1..=12)
            .map(|column| binary_label(record.get(column), &row_source_id))
            .collect::<Result<Vec<_>, _>>()?;
        let hs = labels[0];
        let abusive = labels[1];
        let disposition = match (hs, abusive) {
            (0, 0) => RowDisposition::Candidate(EvalLabel::Clean),
            (0 | 1, 0 | 1) => RowDisposition::Candidate(EvalLabel::Toxic),
            _ => return Err(ImportError::InvalidBinaryLabel(row_source_id)),
        };
        output.push(ImportedRow {
            dataset: DatasetId::IbrohimBudi,
            source_file_id: source_file_id.to_owned(),
            source_id: row_source_id,
            source_language_code: "id".to_owned(),
            detector_language: Some(Language::Ms),
            detector_language_code: Some(Language::Ms.storage_code().to_owned()),
            source_label: format!("HS={hs};Abusive={abusive}"),
            text: String::from_utf8_lossy(
                record
                    .get(0)
                    .ok_or_else(|| ImportError::InvalidSource(native_id.clone()))?,
            )
            .into_owned(),
            source_split,
            disposition,
        });
    }
    Ok(output)
}

fn binary_label(value: Option<&[u8]>, source_id: &str) -> Result<u8, ImportError> {
    match value {
        Some(b"0") => Ok(0),
        Some(b"1") => Ok(1),
        _ => Err(ImportError::InvalidBinaryLabel(source_id.to_owned())),
    }
}
