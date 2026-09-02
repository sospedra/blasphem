use std::cmp::Reverse;

use serde::{Deserialize, Serialize};
use toxcheck::{ConfusionMatrix, EvalLabel, Language};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationRow {
    pub label: EvalLabel,
    pub sparse_raw_score: i32,
    pub rule_should_nudge: bool,
    pub suppress_sparse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateResult {
    pub false_warning_passed: bool,
    pub precision_passed: bool,
    pub has_true_positive: bool,
}

impl GateResult {
    #[must_use]
    pub const fn passed(self) -> bool {
        self.false_warning_passed && self.precision_passed && self.has_true_positive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationResult {
    pub language: Language,
    pub boundary: i32,
    pub matrix: ConfusionMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryEvaluation {
    pub boundary: i32,
    pub matrix: ConfusionMatrix,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum CalibrationError {
    #[error("the frozen rule channel fails a validation gate for {}", .0.code())]
    RuleChannelGateFailure(Language),
    #[error("no admissible validation boundary exists for {}", .0.code())]
    NoAdmissibleBoundary(Language),
}

pub fn calibrate(
    language: Language,
    rows: &[CalibrationRow],
) -> Result<CalibrationResult, CalibrationError> {
    calibrate_at_or_above(language, rows, i32::MIN)
}

pub fn calibrate_at_or_above(
    language: Language,
    rows: &[CalibrationRow],
    minimum_boundary: i32,
) -> Result<CalibrationResult, CalibrationError> {
    let rule_matrix = confusion_matrix(rows, |row| row.rule_should_nudge);
    if !gates(rule_matrix).false_warning_passed {
        return Err(CalibrationError::RuleChannelGateFailure(language));
    }

    let mut boundaries = candidate_boundaries(rows);
    boundaries.push(minimum_boundary);
    boundaries.sort_unstable();
    boundaries.dedup();
    let candidates = boundaries
        .into_iter()
        .filter(|boundary| *boundary >= minimum_boundary)
        .map(|boundary| BoundaryEvaluation {
            boundary,
            matrix: confusion_matrix(rows, |row| {
                row.rule_should_nudge || (!row.suppress_sparse && row.sparse_raw_score >= boundary)
            }),
        })
        .collect::<Vec<_>>();
    let best = select_best(language, &candidates)?;

    Ok(CalibrationResult {
        language,
        boundary: best.boundary,
        matrix: best.matrix,
    })
}

pub fn select_best(
    language: Language,
    candidates: &[BoundaryEvaluation],
) -> Result<BoundaryEvaluation, CalibrationError> {
    candidates
        .iter()
        .copied()
        .filter(|candidate| gates(candidate.matrix).passed())
        .max_by_key(|candidate| {
            (
                candidate.matrix.true_positive,
                Reverse(candidate.matrix.false_positive),
                candidate.boundary,
            )
        })
        .ok_or(CalibrationError::NoAdmissibleBoundary(language))
}

#[must_use]
pub fn candidate_boundaries(rows: &[CalibrationRow]) -> Vec<i32> {
    let mut candidates = rows
        .iter()
        .flat_map(|row| [row.sparse_raw_score, row.sparse_raw_score.saturating_add(1)])
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

#[must_use]
pub fn gates(matrix: ConfusionMatrix) -> GateResult {
    let true_positive = u128::from(matrix.true_positive);
    let false_positive = u128::from(matrix.false_positive);
    let true_negative = u128::from(matrix.true_negative);
    let predicted_toxic = true_positive + false_positive;

    GateResult {
        false_warning_passed: 10_000 * false_positive <= 300 * (false_positive + true_negative),
        precision_passed: predicted_toxic > 0 && 100 * true_positive >= 90 * predicted_toxic,
        has_true_positive: true_positive > 0,
    }
}

fn confusion_matrix(
    rows: &[CalibrationRow],
    predicted: impl Fn(&CalibrationRow) -> bool,
) -> ConfusionMatrix {
    let mut matrix = ConfusionMatrix::default();
    for row in rows {
        match (row.label, predicted(row)) {
            (EvalLabel::Toxic, true) => matrix.true_positive += 1,
            (EvalLabel::Clean, false) => matrix.true_negative += 1,
            (EvalLabel::Clean, true) => matrix.false_positive += 1,
            (EvalLabel::Toxic, false) => matrix.false_negative += 1,
        }
    }
    matrix
}
