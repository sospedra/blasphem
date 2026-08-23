use std::fmt;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::policy::policy_result_from_rule_channel;
use crate::registry::registry_entry;
use crate::{
    Language, NudgeResult, PolicyResult, RULE_NUDGE_THRESHOLD, ReplyTarget, RuleChannel,
    RuleChannelError, SparseModel,
};

/// A fixed-language detector for the product pre-send nudge.
pub struct NudgeDetector {
    language: Language,
    model: &'static SparseModel,
    rule_channel: RuleChannel,
}

impl fmt::Debug for NudgeDetector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NudgeDetector")
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Error)]
pub enum RuntimeInitError {
    #[error("{language} has an invalid embedded model: {reason}")]
    InvalidEmbeddedModel { language: Language, reason: String },
    #[error("{language} has an invalid rule pack: {reason}")]
    InvalidRulePack { language: Language, reason: String },
    #[error("{language} is missing required HurtLex data")]
    MissingHurtlex { language: Language },
    #[error("{language} received unexpected HurtLex data")]
    UnexpectedHurtlex { language: Language },
    #[error("{language} HurtLex digest mismatch")]
    HurtlexDigestMismatch { language: Language },
    #[error("cannot build the {language} rule channel: {source}")]
    RuleChannel {
        language: Language,
        #[source]
        source: RuleChannelError,
    },
}

impl NudgeDetector {
    /// Builds a detector from the pinned language resources.
    ///
    /// # Errors
    ///
    /// Returns an error when any resource is missing, changed, or invalid.
    pub fn from_hurtlex_bytes(
        language: Language,
        hurtlex: Option<&[u8]>,
    ) -> Result<Self, RuntimeInitError> {
        let entry = registry_entry(language);
        let model = entry.model()?;
        validate_hurtlex(language, entry.hurtlex_sha256, hurtlex)?;
        let rule_channel = entry.rule_channel(hurtlex)?;

        Ok(Self {
            language,
            model,
            rule_channel,
        })
    }

    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    #[must_use]
    pub fn analyze(&self, text: &str, reply_target: ReplyTarget) -> PolicyResult {
        let analysis = self.rule_channel.analyze_full(text, reply_target);
        let mut sparse_score = self.model.score(text);
        if self.language != Language::Es && analysis.outcome.suppresses_sparse_channel() {
            sparse_score = sparse_score.min(RULE_NUDGE_THRESHOLD - 1);
        }
        policy_result_from_rule_channel(
            text,
            analysis.lexical,
            analysis.outcome,
            Some(sparse_score),
        )
    }

    #[must_use]
    pub fn check(&self, text: &str, reply_target: ReplyTarget) -> NudgeResult {
        self.analyze(text, reply_target).nudge()
    }
}

fn validate_hurtlex(
    language: Language,
    expected: Option<[u8; 32]>,
    bytes: Option<&[u8]>,
) -> Result<(), RuntimeInitError> {
    match (expected, bytes) {
        (Some(_), None) => Err(RuntimeInitError::MissingHurtlex { language }),
        (None, Some(_)) => Err(RuntimeInitError::UnexpectedHurtlex { language }),
        (Some(expected), Some(bytes)) => {
            let actual: [u8; 32] = Sha256::digest(bytes).into();
            if actual == expected {
                Ok(())
            } else {
                Err(RuntimeInitError::HurtlexDigestMismatch { language })
            }
        }
        (None, None) => Ok(()),
    }
}
