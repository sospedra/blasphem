use std::io::Read;

use serde::Deserialize;
use thiserror::Error;
use blasphem::EvalRow;

#[derive(Debug, Error)]
pub enum ParseEvaluationError {
    #[error("cannot parse evaluation TSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid evaluation label: {0}")]
    InvalidLabel(String),
    #[error("the evaluation dataset is empty")]
    EmptyDataset,
}

#[derive(Debug, Deserialize)]
struct RawEvalRow {
    language: String,
    label: String,
    text: String,
}

pub fn parse_eval_rows(reader: impl Read) -> Result<Vec<EvalRow>, ParseEvaluationError> {
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .trim(csv::Trim::Headers)
        .from_reader(reader);
    let mut rows = Vec::new();
    for row in csv.deserialize::<RawEvalRow>() {
        let row = row?;
        let label = row
            .label
            .trim()
            .parse()
            .map_err(ParseEvaluationError::InvalidLabel)?;
        rows.push(EvalRow {
            language: row.language.trim().to_ascii_uppercase(),
            label,
            text: row.text,
        });
    }
    if rows.is_empty() {
        return Err(ParseEvaluationError::EmptyDataset);
    }
    Ok(rows)
}
