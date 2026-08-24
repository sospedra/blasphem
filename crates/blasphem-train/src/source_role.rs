use serde::{Deserialize, Serialize};

/// How the preparation pipeline may use one corpus source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// A frozen source whose partition the pipeline must preserve.
    Baseline,
    /// A community source whose rows enter only the development partition.
    TrainingOnly,
    /// A source reserved for sealed validation and test rows.
    SealedEvaluation,
}

impl SourceRole {
    /// Returns true when the role forbids new validation or test rows.
    #[must_use]
    pub const fn is_development_only(self) -> bool {
        matches!(self, Self::TrainingOnly)
    }
}
