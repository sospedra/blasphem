use std::io::Read;

use blasphem::{EvalLabel, Language};

use crate::datasets::{
    DatasetAdapter, DatasetId, ExclusionReason, ImportError, ImportedRow, RowDisposition,
    SourceInput, SourceSplit,
};

const COMMUNITY_HEADER: [&str; 3] = ["native_id", "label", "text"];

/// Reads one stranger-contributed training-only corpus source.
///
/// The format is a three-column TSV (`native_id`, `label`, `text`) with no
/// language column: one adapter instance covers exactly one declared
/// `source_file_id` in exactly one language.
pub struct CommunityCorpusAdapter {
    language: Language,
    source_file_id: String,
}

impl CommunityCorpusAdapter {
    #[must_use]
    pub fn new(language: Language, source_file_id: impl Into<String>) -> Self {
        Self {
            language,
            source_file_id: source_file_id.into(),
        }
    }
}

impl DatasetAdapter for CommunityCorpusAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::Community
    }

    fn label_conversion_version(&self) -> &'static str {
        "community-binary-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let [input] = inputs else {
            return Err(ImportError::InvalidSource(self.source_file_id.clone()));
        };
        if input.source_file_id != self.source_file_id {
            return Err(ImportError::InvalidSource(input.source_file_id.to_owned()));
        }
        import_source(
            &mut input.reader,
            &self.source_file_id,
            self.language,
            input.source_split,
        )
    }
}

fn import_source(
    reader: impl Read,
    source_file_id: &str,
    language: Language,
    source_split: SourceSplit,
) -> Result<Vec<ImportedRow>, ImportError> {
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(reader);
    let header = csv.headers()?;
    if header.iter().ne(COMMUNITY_HEADER) {
        return Err(ImportError::InvalidSource(format!(
            "{source_file_id}: header must be native_id, label, text"
        )));
    }

    let mut output = Vec::new();
    for record in csv.records() {
        let record = record?;
        let native_id = record.get(0).unwrap_or_default();
        let raw_label = record.get(1).unwrap_or_default();
        let text = record.get(2).unwrap_or_default();
        let disposition = row_disposition(raw_label, text, source_file_id, native_id)?;
        output.push(ImportedRow {
            dataset: DatasetId::Community,
            source_file_id: source_file_id.to_owned(),
            source_id: format!("{source_file_id}/{native_id}"),
            source_language_code: language.code().to_ascii_lowercase(),
            detector_language: Some(language),
            detector_language_code: Some(language.storage_code().to_owned()),
            source_label: raw_label.to_owned(),
            text: text.to_owned(),
            source_split,
            disposition,
        });
    }
    Ok(output)
}

fn row_disposition(
    raw_label: &str,
    text: &str,
    source_file_id: &str,
    native_id: &str,
) -> Result<RowDisposition, ImportError> {
    if text.is_empty() {
        return Ok(RowDisposition::Excluded(ExclusionReason::EmptyText));
    }
    match raw_label {
        "toxic" => Ok(RowDisposition::Candidate(EvalLabel::Toxic)),
        "clean" => Ok(RowDisposition::Candidate(EvalLabel::Clean)),
        _ => Err(ImportError::InvalidSource(format!(
            "{source_file_id}/{native_id}: invalid label {raw_label}"
        ))),
    }
}
