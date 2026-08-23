use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EvalLabel {
    Clean,
    Toxic,
}

impl FromStr for EvalLabel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "clean" => Ok(Self::Clean),
            "toxic" => Ok(Self::Toxic),
            _ => Err(value.to_owned()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalRow {
    pub language: String,
    pub label: EvalLabel,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfusionMatrix {
    pub true_positive: u64,
    pub true_negative: u64,
    pub false_positive: u64,
    pub false_negative: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub accuracy: Option<f64>,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub specificity: Option<f64>,
    pub f1: Option<f64>,
}

impl ConfusionMatrix {
    #[must_use]
    pub fn metrics(self) -> Metrics {
        let total = self
            .true_positive
            .saturating_add(self.true_negative)
            .saturating_add(self.false_positive)
            .saturating_add(self.false_negative);
        Metrics {
            accuracy: ratio(self.true_positive.saturating_add(self.true_negative), total),
            precision: ratio(
                self.true_positive,
                self.true_positive.saturating_add(self.false_positive),
            ),
            recall: ratio(
                self.true_positive,
                self.true_positive.saturating_add(self.false_negative),
            ),
            specificity: ratio(
                self.true_negative,
                self.true_negative.saturating_add(self.false_positive),
            ),
            f1: ratio(
                self.true_positive.saturating_mul(2),
                self.true_positive
                    .saturating_mul(2)
                    .saturating_add(self.false_positive)
                    .saturating_add(self.false_negative),
            ),
        }
    }
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator != 0).then(|| numerator as f64 / denominator as f64)
}
