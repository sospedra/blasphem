use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

use blasphem::{EvalLabel, Language};

use super::{
    DatasetAdapter, DatasetId, ImportError, ImportedRow, RowDisposition, SourceInput, SourceSplit,
    source_id,
};

const OFFENSEVAL_TR_REVISION: &str = "official-v1";
const TRAINING_HEADER: [&str; 3] = ["id", "tweet", "subtask_a"];
const TEST_HEADER: [&str; 2] = ["id", "tweet"];

pub struct OffensEvalTrAdapter;

impl DatasetAdapter for OffensEvalTrAdapter {
    fn dataset_id(&self) -> DatasetId {
        DatasetId::OffensEvalTr
    }

    fn label_conversion_version(&self) -> &'static str {
        "offenseval-tr-off-not-v1"
    }

    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError> {
        let [training, test, test_labels] = inputs else {
            return Err(ImportError::InvalidSource(
                "offenseval-tr inputs".to_owned(),
            ));
        };
        if training.source_file_id != "offenseval-tr-training"
            || training.source_split != SourceSplit::Train
            || test.source_file_id != "offenseval-tr-test"
            || test.source_split != SourceSplit::Test
            || test_labels.source_file_id != "offenseval-tr-test-labels"
            || test_labels.source_split != SourceSplit::Test
        {
            return Err(ImportError::InvalidSource(
                "offenseval-tr inputs".to_owned(),
            ));
        }

        let mut output = read_turkish_texts(&mut training.reader, true)?
            .into_iter()
            .map(|row| convert_turkish(row, SourceSplit::Train, training.source_file_id))
            .collect::<Result<Vec<_>, _>>()?;
        let labels = read_turkish_labels(&mut test_labels.reader)?;
        let mut used_labels = BTreeSet::new();
        for row in read_turkish_texts(&mut test.reader, false)? {
            let label = labels
                .get(&row.id)
                .ok_or_else(|| ImportError::MissingJoinedLabel(row.id.clone()))?;
            used_labels.insert(row.id.clone());
            output.push(convert_turkish(
                TurkishTextRow {
                    label: *label,
                    ..row
                },
                SourceSplit::Test,
                test.source_file_id,
            )?);
        }
        if let Some(id) = labels.keys().find(|id| !used_labels.contains(*id)) {
            return Err(ImportError::UnusedJoinedLabel(id.clone()));
        }
        Ok(output)
    }
}

struct TurkishTextRow {
    id: String,
    text: String,
    label: TurkishLabel,
}

#[derive(Clone, Copy)]
enum TurkishLabel {
    Off,
    Not,
}

fn read_turkish_texts(
    reader: impl Read,
    has_label: bool,
) -> Result<Vec<TurkishTextRow>, ImportError> {
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .quoting(false)
        .from_reader(reader);
    let header = csv.headers()?;
    let expected = if has_label {
        &TRAINING_HEADER[..]
    } else {
        &TEST_HEADER[..]
    };
    if header.iter().ne(expected.iter().copied()) {
        return Err(ImportError::InvalidSource(
            "offenseval-tr header".to_owned(),
        ));
    }
    let mut ids = BTreeSet::new();
    let mut output = Vec::new();
    for record in csv.records() {
        let record = record?;
        let id = required_field(&record, 0, "offenseval-tr id")?.to_owned();
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(ImportError::InvalidSource(id));
        }
        let text = required_field(&record, 1, "offenseval-tr text")?.to_owned();
        let label = if has_label {
            parse_turkish_label(required_field(&record, 2, "offenseval-tr label")?)?
        } else {
            TurkishLabel::Not
        };
        output.push(TurkishTextRow { id, text, label });
    }
    Ok(output)
}

fn read_turkish_labels(reader: impl Read) -> Result<BTreeMap<String, TurkishLabel>, ImportError> {
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(reader);
    let mut labels = BTreeMap::new();
    for record in csv.records() {
        let record = record?;
        if record.len() != 2 {
            return Err(ImportError::InvalidSource(
                "offenseval-tr test label".to_owned(),
            ));
        }
        let id = required_field(&record, 0, "offenseval-tr test label id")?.to_owned();
        if id.is_empty() || labels.contains_key(&id) {
            return Err(ImportError::InvalidSource(id));
        }
        let label = parse_turkish_label(required_field(&record, 1, "offenseval-tr test label")?)?;
        labels.insert(id, label);
    }
    Ok(labels)
}

fn convert_turkish(
    row: TurkishTextRow,
    source_split: SourceSplit,
    source_file_id: &str,
) -> Result<ImportedRow, ImportError> {
    let (source_label, disposition) = match row.label {
        TurkishLabel::Off => ("OFF", RowDisposition::Candidate(EvalLabel::Toxic)),
        TurkishLabel::Not => ("NOT", RowDisposition::Candidate(EvalLabel::Clean)),
    };
    Ok(ImportedRow {
        dataset: DatasetId::OffensEvalTr,
        source_file_id: source_file_id.to_owned(),
        source_id: source_id(
            DatasetId::OffensEvalTr,
            OFFENSEVAL_TR_REVISION,
            source_split,
            &row.id,
        ),
        source_language_code: "tr".to_owned(),
        detector_language: Some(Language::Tr),
        detector_language_code: Some(Language::Tr.code().to_owned()),
        source_label: source_label.to_owned(),
        text: row.text,
        source_split,
        disposition,
    })
}

fn parse_turkish_label(value: &str) -> Result<TurkishLabel, ImportError> {
    match value {
        "OFF" => Ok(TurkishLabel::Off),
        "NOT" => Ok(TurkishLabel::Not),
        _ => Err(ImportError::InvalidBinaryLabel(value.to_owned())),
    }
}

fn required_field<'a>(
    record: &'a csv::StringRecord,
    index: usize,
    field: &'static str,
) -> Result<&'a str, ImportError> {
    record.get(index).ok_or(ImportError::MissingColumn(field))
}
