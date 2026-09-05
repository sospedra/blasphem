//! Experimental multilingual toxicity detector.
//!
//! Blasphem hashes word and character n-grams into sparse feature vectors.
//! A linear classifier trained offline scores them with 16-bit weights.
//! Lexicons and context rules contribute to the verdict.
//! Detection runs locally without neural networks or cloud inference.

mod detector;
#[cfg(feature = "embedded")]
mod embedded;
mod engine;
mod evaluation;
mod features;
mod grawlix;
mod judge;
mod language;
mod language_detection;
mod lexicon;
mod normalization;
mod pack;
mod policy;
mod registry;
mod rule_pack;
pub mod rules;
mod runtime;
mod sparse;
mod text;

pub use detector::{
    Detection, Detector, DetectorError, LexiconMatch, lexicon_marked_text, normalize_text,
    uses_lexicon_features,
};
#[cfg(feature = "embedded")]
pub use embedded::{embedded_detector, embedded_lexicon_bytes};
pub use engine::{Engine, EngineError, EngineJudgement, EngineSource};
pub use evaluation::{ConfusionMatrix, EvalLabel, EvalRow, Metrics};
pub use features::{FeatureError, extract_feature_bins};
pub use grawlix::{apply_grawlix, masked_spans};
pub use judge::{Judge, JudgeError, JudgeOptions, Judgement};
pub use language::{
    FeatureProfile, FeatureSchema, Language, NormalizationProfile, UnsupportedLanguage,
};
pub use language_detection::{
    LanguageDetection, LanguageIdentifier, LanguageResolution, LanguageSelection, LanguageSource,
    resolve_language,
};
#[cfg(feature = "language-detection")]
pub use language_detection::{LanguageDetector, LanguageDetectorError};
pub use lexicon::{LexiconEntry, MatchLevel, ParseLexiconError, parse_lexicon};
pub use normalization::normalize;
pub use pack::{
    DecodedPack, PACK_FORMAT_VERSION, PACK_HEADER_LEN, PACK_MAGIC, PackError, PackInput,
    PackSource, decode_pack, detect_file_name, encode_pack, pack_file_name, parse_sha256,
    verify_digest,
};
pub use policy::{
    CategoryScores, NudgeResult, PolicyAction, PolicyCategory, PolicyResult, ReplyTarget,
    RuleContext, RuleEvidence, RuleId,
};
pub use registry::{LanguageSpec, language_spec};
pub use rules::{
    DIRECT_THREAT_SCORE, DIRECTED_INSULT_SCORE, HARM_WISH_SCORE, LEXICON_SCORE, LanguageRules,
    NEGATIVE_SENTIMENT_SCORE, PhraseSet, RULE_NUDGE_THRESHOLD, RuleChannel, RuleChannelError,
    RuleMatchProfile, RuleOutcome, SELF_HARM_COMMAND_SCORE, analyze_with_rules, arabic_hindi_rules,
    canonical_rule_identity, canonical_rule_identity_for, cjk_rules, word_rules,
};
pub use runtime::{NudgeDetector, RuntimeInitError};
pub use sparse::{SparseInput, SparseModel, SparseModelError, encode_sparse};
pub use text::{CandidateView, CandidateViewKind, TextDocument, TextSpan};
