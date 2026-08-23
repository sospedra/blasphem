use std::{
    collections::BTreeMap,
    fs::File,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    AnalysisContext, ConfusionMatrix, Detector, DetectorError, EvalLabel, EvalRow, LexiconEntry,
    MatchLevel, ParseLexiconError, PolicyAction, parse_hurtlex,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSelection {
    Conservative,
    All,
}

impl LevelSelection {
    fn includes(self, level: MatchLevel) -> bool {
        self == Self::All || level == MatchLevel::Conservative
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationReport {
    pub overall: ConfusionMatrix,
    pub by_language: BTreeMap<String, ConfusionMatrix>,
}

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ParseLexiconError,
    },
    #[error("threshold must be between 0 and 1")]
    InvalidThreshold,
    #[error("no lexicon entries exist for language {0}")]
    MissingLanguage(String),
    #[error("the evaluation dataset is empty")]
    EmptyDataset,
    #[error("an evaluation counter overflowed")]
    CounterOverflow,
    #[error(transparent)]
    Detector(#[from] DetectorError),
}

pub fn load_lexica(
    data_directory: &Path,
    languages: &[String],
    levels: LevelSelection,
) -> Result<Vec<LexiconEntry>, WorkflowError> {
    let mut entries = Vec::new();
    for language in languages {
        let language = language.trim().to_ascii_uppercase();
        let path = data_directory.join(format!("hurtlex_{language}.tsv"));
        let file = File::open(&path).map_err(|source| WorkflowError::Read {
            path: path.clone(),
            source,
        })?;
        let parsed = parse_hurtlex(file, &language).map_err(|source| WorkflowError::Parse {
            path: path.clone(),
            source,
        })?;
        entries.extend(
            parsed
                .into_iter()
                .filter(|entry| levels.includes(entry.level)),
        );
    }
    Ok(entries)
}

pub fn evaluate(
    rows: &[EvalRow],
    entries: Vec<LexiconEntry>,
    threshold: f64,
) -> Result<EvaluationReport, WorkflowError> {
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return Err(WorkflowError::InvalidThreshold);
    }
    if rows.is_empty() {
        return Err(WorkflowError::EmptyDataset);
    }

    let mut entries_by_language = BTreeMap::<String, Vec<LexiconEntry>>::new();
    for entry in entries {
        entries_by_language
            .entry(entry.language.clone())
            .or_default()
            .push(entry);
    }
    let mut detectors = BTreeMap::new();
    for language in rows.iter().map(|row| row.language.as_str()) {
        if detectors.contains_key(language) {
            continue;
        }
        let language_entries = entries_by_language
            .remove(language)
            .ok_or_else(|| WorkflowError::MissingLanguage(language.to_owned()))?;
        detectors.insert(language.to_owned(), Detector::new(language_entries)?);
    }

    let mut overall = ConfusionMatrix::default();
    let mut by_language = BTreeMap::<String, ConfusionMatrix>::new();
    for row in rows {
        let detector = detectors
            .get(&row.language)
            .ok_or_else(|| WorkflowError::MissingLanguage(row.language.clone()))?;
        let predicted_toxic = detector.check(&row.text).score >= threshold;
        record(&mut overall, row.label, predicted_toxic)?;
        record(
            by_language.entry(row.language.clone()).or_default(),
            row.label,
            predicted_toxic,
        )?;
    }

    Ok(EvaluationReport {
        overall,
        by_language,
    })
}

pub fn evaluate_policy(
    rows: &[EvalRow],
    entries: Vec<LexiconEntry>,
    minimum_action: PolicyAction,
) -> Result<EvaluationReport, WorkflowError> {
    if rows.is_empty() {
        return Err(WorkflowError::EmptyDataset);
    }

    let mut entries_by_language = BTreeMap::<String, Vec<LexiconEntry>>::new();
    for mut entry in entries {
        entry.language = entry.language.trim().to_ascii_uppercase();
        entries_by_language
            .entry(entry.language.clone())
            .or_default()
            .push(entry);
    }
    let row_languages = rows
        .iter()
        .map(|row| row.language.trim().to_ascii_uppercase())
        .collect::<std::collections::BTreeSet<_>>();
    let mut detectors = BTreeMap::new();
    for language in row_languages {
        let language_entries = entries_by_language
            .remove(&language)
            .ok_or_else(|| WorkflowError::MissingLanguage(language.clone()))?;
        detectors.insert(language, Detector::new(language_entries)?);
    }

    let mut overall = ConfusionMatrix::default();
    let mut by_language = BTreeMap::<String, ConfusionMatrix>::new();
    for row in rows {
        let language = row.language.trim().to_ascii_uppercase();
        let detector = detectors
            .get(&language)
            .ok_or_else(|| WorkflowError::MissingLanguage(language.clone()))?;
        let result = detector.analyze(&row.text, AnalysisContext::for_language(&language));
        let predicted_toxic = if minimum_action == PolicyAction::Review {
            result.nudge().should_nudge
        } else {
            result.action >= minimum_action
        };
        record(&mut overall, row.label, predicted_toxic)?;
        record(
            by_language.entry(language).or_default(),
            row.label,
            predicted_toxic,
        )?;
    }

    Ok(EvaluationReport {
        overall,
        by_language,
    })
}

fn record(
    matrix: &mut ConfusionMatrix,
    expected: EvalLabel,
    predicted_toxic: bool,
) -> Result<(), WorkflowError> {
    let counter = match (expected, predicted_toxic) {
        (EvalLabel::Toxic, true) => &mut matrix.true_positive,
        (EvalLabel::Clean, false) => &mut matrix.true_negative,
        (EvalLabel::Clean, true) => &mut matrix.false_positive,
        (EvalLabel::Toxic, false) => &mut matrix.false_negative,
    };
    *counter = counter
        .checked_add(1)
        .ok_or(WorkflowError::CounterOverflow)?;
    Ok(())
}
