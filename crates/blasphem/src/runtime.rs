use std::borrow::Cow;
use std::fmt;

#[cfg(feature = "embedded")]
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::pack::{PackError, decode_pack, pack_file_name};
use crate::policy::policy_result_from_rule_channel;
use crate::registry::registry_entry;
use crate::{
    Language, NudgeResult, PolicyResult, RULE_NUDGE_THRESHOLD, ReplyTarget, RuleChannel,
    RuleChannelError, SparseModel,
    detector::{lexicon_marked_text, uses_lexicon_features},
};

/// A fixed-language detector for the product pre-send nudge.
pub struct NudgeDetector {
    language: Language,
    model: Cow<'static, SparseModel>,
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
    #[error("{language} is missing required Lexicon data")]
    MissingLexicon { language: Language },
    #[error("{language} received unexpected Lexicon data")]
    UnexpectedLexicon { language: Language },
    #[error("{language} Lexicon digest mismatch")]
    LexiconDigestMismatch { language: Language },
    #[error("cannot build the {language} rule channel: {source}")]
    RuleChannel {
        language: Language,
        #[source]
        source: RuleChannelError,
    },
    #[error(transparent)]
    Pack(#[from] PackError),
}

impl NudgeDetector {
    /// Builds a detector from the pinned language resources.
    ///
    /// # Errors
    ///
    /// Returns an error when any resource is missing, changed, or invalid.
    #[cfg(feature = "embedded")]
    pub fn from_lexicon_bytes(
        language: Language,
        lexicon: Option<&[u8]>,
    ) -> Result<Self, RuntimeInitError> {
        let entry = registry_entry(language);
        let model = crate::embedded::embedded_model_ref(language)?;
        validate_lexicon(
            language,
            crate::embedded::embedded_lexicon_sha256(language),
            lexicon,
        )?;
        let rule_channel = entry.rule_channel(lexicon)?;

        Ok(Self {
            language,
            model,
            rule_channel,
        })
    }

    /// Builds a detector from one language's pack bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the pack is malformed, declares another language,
    /// was built for other rules, or carries an invalid artifact or lexicon.
    pub fn from_pack(language: Language, pack: &[u8]) -> Result<Self, RuntimeInitError> {
        let file = pack_file_name(language);
        let decoded = decode_pack(language, pack)?;
        let entry = registry_entry(language);
        let expected_version = entry.expected_rule_pack_version()?;
        if decoded.rule_pack_version != expected_version {
            return Err(PackError::Invalid {
                file,
                reason: format!(
                    "was built for rule pack version {}, this build has {expected_version}",
                    decoded.rule_pack_version
                ),
            }
            .into());
        }
        let model =
            SparseModel::from_bytes(decoded.artifact).map_err(|error| PackError::Invalid {
                file: file.clone(),
                reason: error.to_string(),
            })?;
        entry
            .check_model(&model)
            .map_err(|reason| PackError::Invalid {
                file: file.clone(),
                reason,
            })?;
        let rule_channel = entry.rule_channel(Some(decoded.lexicon))?;
        Ok(Self::from_parts(language, Cow::Owned(model), rule_channel))
    }

    pub(crate) const fn from_parts(
        language: Language,
        model: Cow<'static, SparseModel>,
        rule_channel: RuleChannel,
    ) -> Self {
        Self {
            language,
            model,
            rule_channel,
        }
    }

    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    #[must_use]
    pub fn analyze(&self, text: &str, reply_target: ReplyTarget) -> PolicyResult {
        let analysis = self.rule_channel.analyze_full(text, reply_target);
        let model_text = if uses_lexicon_features(self.language) {
            Cow::Owned(lexicon_marked_text(text, &analysis.lexical.matches))
        } else {
            Cow::Borrowed(text)
        };
        let mut sparse_score = self.model.score(&model_text);
        if analysis.outcome.suppresses_sparse_channel() {
            sparse_score = sparse_score.min(RULE_NUDGE_THRESHOLD - 1);
        }
        policy_result_from_rule_channel(
            text,
            analysis.lexical,
            analysis.outcome,
            analysis.scores,
            Some(sparse_score),
        )
    }

    #[must_use]
    pub fn check(&self, text: &str, reply_target: ReplyTarget) -> NudgeResult {
        self.analyze(text, reply_target).nudge()
    }
}

#[cfg(feature = "embedded")]
fn validate_lexicon(
    language: Language,
    expected: Option<[u8; 32]>,
    bytes: Option<&[u8]>,
) -> Result<(), RuntimeInitError> {
    match (expected, bytes) {
        (Some(_), None) => Err(RuntimeInitError::MissingLexicon { language }),
        (None, Some(_)) => Err(RuntimeInitError::UnexpectedLexicon { language }),
        (Some(expected), Some(bytes)) => {
            let actual: [u8; 32] = Sha256::digest(bytes).into();
            if actual == expected {
                Ok(())
            } else {
                Err(RuntimeInitError::LexiconDigestMismatch { language })
            }
        }
        (None, None) => Ok(()),
    }
}
