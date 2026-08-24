//! Experimental multilingual lexical toxicity detector.

mod detector;
mod evaluation;
mod features;
mod language;
mod language_detection;
mod lexicon;
mod normalization;
mod policy;
mod registry;
mod rule_pack;
pub mod rules;
mod runtime;
mod sparse;
mod text;
mod workflow;

pub use detector::{Detection, Detector, DetectorError, LexiconMatch, normalize_text};
pub use evaluation::{ConfusionMatrix, EvalLabel, EvalRow, Metrics};
pub use features::{FeatureError, extract_feature_bins};
pub use language::{
    FeatureProfile, FeatureSchema, Language, NormalizationProfile, UnsupportedLanguage,
};
pub use language_detection::{
    LanguageDetection, LanguageIdentifier, LanguageResolution, LanguageSelection, LanguageSource,
    resolve_language,
};
#[cfg(feature = "language-detection")]
pub use language_detection::{LanguageDetector, LanguageDetectorError};
pub use lexicon::{LexiconEntry, MatchLevel, ParseLexiconError, parse_hurtlex};
pub use normalization::{NormalizationError, normalize_v2};
pub use policy::{
    AnalysisContext, CategoryScores, NudgeResult, PolicyAction, PolicyCategory, PolicyResult,
    ReplyTarget, RuleEvidence, RuleId,
};
pub use registry::{LanguageSpec, language_spec};
pub use rules::{
    DIRECT_THREAT_SCORE, DIRECTED_INSULT_SCORE, HARM_WISH_SCORE, HURTLEX_SCORE, LanguageRules,
    NEGATIVE_SENTIMENT_SCORE, PhraseSet, RULE_NUDGE_THRESHOLD, RuleChannel, RuleChannelError,
    RuleMatchProfile, RuleOutcome, SELF_HARM_COMMAND_SCORE, analyze_with_rules, arabic_hindi_rules,
    canonical_rule_identity, canonical_rule_identity_for, cjk_rules, word_rules,
};
pub use runtime::{NudgeDetector, RuntimeInitError};
pub use sparse::{
    SparseModel, SparseModelError, SparseV1Input, SparseV2Input, encode_sparse_v1, encode_sparse_v2,
};
pub use text::{CandidateView, CandidateViewKind, TextDocument, TextSpan};
pub use workflow::{
    EvaluationReport, LevelSelection, WorkflowError, evaluate, evaluate_policy, load_lexica,
};
