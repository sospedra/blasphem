use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::rule_pack::{
    RulePack, for_language, lexical_collision_excluded, lexical_collision_exclusions,
    lexical_collision_reactivation_phrase,
};
use crate::{
    Detection, Detector, DetectorError, Language, MatchLevel, ParseLexiconError, PolicyCategory,
    ReplyTarget, RuleEvidence, RuleId, TextDocument, normalize_text, parse_hurtlex,
};

use super::{
    HURTLEX_SCORE, LanguageRules, RULE_NUDGE_THRESHOLD, RuleOutcome, analyze_with_rules,
    arabic_hindi_rules, canonical_rule_identity_for, cjk_rules, word_rules,
};

const CHANNEL_IDENTITY_MAGIC: &[u8] = b"TOXCHANNEL1";
const LEGACY_IDENTITY_MAGIC: &[u8] = b"TOXLEGACY1";

enum ResolvedRules {
    Spanish(SpanishRules),
    V2(&'static LanguageRules),
}

enum SpanishRules {
    Owned(Box<RulePack>),
    Cached(&'static RulePack),
}

impl SpanishRules {
    fn as_ref(&self) -> &RulePack {
        match self {
            Self::Owned(rules) => rules,
            Self::Cached(rules) => rules,
        }
    }
}

/// One immutable semantic and conservative HurtLex rule channel.
pub struct RuleChannel {
    language: Language,
    lexical: Option<Detector>,
    rules: ResolvedRules,
}

pub(crate) struct RuleChannelAnalysis {
    pub outcome: RuleOutcome,
    pub lexical: Detection,
}

#[derive(Debug, Error)]
pub enum RuleChannelError {
    #[error("cannot parse {language} HurtLex data: {source}")]
    ParseHurtlex {
        language: &'static str,
        #[source]
        source: ParseLexiconError,
    },
    #[error("cannot build the {language} HurtLex detector: {source}")]
    Detector {
        language: &'static str,
        #[source]
        source: DetectorError,
    },
    #[error("no static rules exist for {0}")]
    MissingRules(&'static str),
}

impl RuleChannel {
    /// Builds one channel with conservative HurtLex entries only.
    ///
    /// # Errors
    ///
    /// Returns an error for a wrong-language HurtLex file or an invalid matcher.
    pub fn from_hurtlex_bytes(
        language: Language,
        hurtlex: Option<&[u8]>,
    ) -> Result<Self, RuleChannelError> {
        let rules = resolve_rules(language)?;
        Self::from_resolved_rules(language, hurtlex, rules)
    }

    pub(crate) fn from_cached_spanish(
        language: Language,
        hurtlex: Option<&[u8]>,
        rules: &'static RulePack,
    ) -> Result<Self, RuleChannelError> {
        Self::from_resolved_rules(
            language,
            hurtlex,
            ResolvedRules::Spanish(SpanishRules::Cached(rules)),
        )
    }

    pub(crate) fn from_cached_v2(
        language: Language,
        hurtlex: Option<&[u8]>,
        rules: &'static LanguageRules,
    ) -> Result<Self, RuleChannelError> {
        Self::from_resolved_rules(language, hurtlex, ResolvedRules::V2(rules))
    }

    fn from_resolved_rules(
        language: Language,
        hurtlex: Option<&[u8]>,
        rules: ResolvedRules,
    ) -> Result<Self, RuleChannelError> {
        let lexical = hurtlex
            .map(|bytes| {
                parse_hurtlex(bytes, language.storage_code()).map_err(|source| {
                    RuleChannelError::ParseHurtlex {
                        language: language.code(),
                        source,
                    }
                })
            })
            .transpose()?
            .map(|entries| {
                entries
                    .into_iter()
                    .filter(|entry| entry.level == MatchLevel::Conservative)
                    .collect::<Vec<_>>()
            })
            .filter(|entries| !entries.is_empty())
            .map(|entries| {
                Detector::new(entries).map_err(|source| RuleChannelError::Detector {
                    language: language.code(),
                    source,
                })
            })
            .transpose()?;

        Ok(Self {
            language,
            lexical,
            rules,
        })
    }

    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    #[must_use]
    pub fn analyze(&self, text: &str, reply_target: ReplyTarget) -> RuleOutcome {
        let RuleChannelAnalysis { outcome, lexical } = self.analyze_full(text, reply_target);
        drop(lexical);
        outcome
    }

    pub(crate) fn analyze_full(
        &self,
        text: &str,
        reply_target: ReplyTarget,
    ) -> RuleChannelAnalysis {
        match &self.rules {
            ResolvedRules::Spanish(pack) => self.analyze_spanish(pack.as_ref(), text, reply_target),
            ResolvedRules::V2(rules) => self.analyze_v2(rules, text, reply_target),
        }
    }

    fn analyze_spanish(
        &self,
        pack: &RulePack,
        text: &str,
        reply_target: ReplyTarget,
    ) -> RuleChannelAnalysis {
        let result = if let Some(detector) = &self.lexical {
            crate::policy::analyze_without_sparse_with_pack(detector, text, reply_target, pack)
        } else {
            crate::policy::analyze_without_sparse_with_pack(
                &Detector::rules_only(),
                text,
                reply_target,
                pack,
            )
        };
        let score = result.max_risk_points();

        RuleChannelAnalysis {
            outcome: RuleOutcome {
                score,
                should_nudge: score >= RULE_NUDGE_THRESHOLD,
                evidence: result.evidence,
            },
            lexical: result.lexical,
        }
    }

    fn analyze_v2(
        &self,
        rules: &LanguageRules,
        text: &str,
        reply_target: ReplyTarget,
    ) -> RuleChannelAnalysis {
        let mut outcome = analyze_with_rules(rules, text, reply_target);
        let lexical = self.lexical.as_ref().map_or_else(
            || Detection {
                normalized_text: normalize_text(text),
                score: 0.0,
                matches: Vec::new(),
            },
            |detector| detector.check(text),
        );
        let lexical_score = append_lexical_evidence(self.language, text, &lexical, &mut outcome);
        outcome.score = outcome.score.max(lexical_score);
        outcome.should_nudge = outcome.score >= RULE_NUDGE_THRESHOLD;

        RuleChannelAnalysis { outcome, lexical }
    }
}

fn resolve_rules(language: Language) -> Result<ResolvedRules, RuleChannelError> {
    if language == Language::Es {
        return for_language(language.code())
            .map(Box::new)
            .map(SpanishRules::Owned)
            .map(ResolvedRules::Spanish)
            .ok_or(RuleChannelError::MissingRules(language.code()));
    }

    word_rules(language)
        .or_else(|| arabic_hindi_rules(language))
        .or_else(|| cjk_rules(language))
        .map(ResolvedRules::V2)
        .ok_or(RuleChannelError::MissingRules(language.code()))
}

fn append_lexical_evidence(
    language: Language,
    text: &str,
    lexical: &Detection,
    outcome: &mut RuleOutcome,
) -> u8 {
    let mut seen = BTreeSet::new();
    let mut collision_cache = BTreeMap::new();
    let mut score = 0;
    let reactivation_document = lexical
        .matches
        .iter()
        .any(|found| {
            collision_policy(
                language.storage_code(),
                found.entry.lemma.as_str(),
                &mut collision_cache,
            )
            .1
            .is_some()
        })
        .then(|| TextDocument::for_rule_language(text, language));

    for found in &lexical.matches {
        let (collision_excluded, reactivation_phrase) = collision_policy(
            language.storage_code(),
            found.entry.lemma.as_str(),
            &mut collision_cache,
        );
        let excluded = collision_excluded
            && !collision_reactivated(reactivation_document.as_ref(), found, reactivation_phrase);
        let key = (
            found.raw_start,
            found.raw_end,
            found.entry.lemma.clone(),
            excluded,
        );
        if !seen.insert(key) {
            continue;
        }
        let points = if excluded { 0 } else { HURTLEX_SCORE };
        if !excluded {
            score = HURTLEX_SCORE;
        }
        outcome.evidence.push(RuleEvidence {
            rule_id: if excluded {
                RuleId::LexicalCollisionExcluded
            } else {
                RuleId::LexicalMatch
            },
            category: PolicyCategory::Profanity,
            points,
            language: Some(language.code().to_owned()),
            matched_text: text
                .get(found.raw_start..found.raw_end)
                .unwrap_or_default()
                .to_owned(),
            candidate_view: Some(found.view),
            normalized_start: Some(found.normalized_start),
            normalized_end: Some(found.normalized_end),
            raw_start: Some(found.raw_start),
            raw_end: Some(found.raw_end),
        });
    }

    score
}

type CollisionPolicy = (bool, Option<&'static [&'static str]>);

fn collision_policy<'a>(
    language: &'a str,
    lemma: &'a str,
    cache: &mut BTreeMap<(&'a str, &'a str), CollisionPolicy>,
) -> CollisionPolicy {
    *cache.entry((language, lemma)).or_insert_with(|| {
        (
            lexical_collision_excluded(language, lemma),
            lexical_collision_reactivation_phrase(language, lemma),
        )
    })
}

fn collision_reactivated(
    document: Option<&TextDocument>,
    found: &crate::LexiconMatch,
    phrase: Option<&[&str]>,
) -> bool {
    let Some(phrase) = phrase else {
        return false;
    };
    let Some(document) = document else {
        return false;
    };
    let tokens = document.context_tokens();
    let event_index = tokens.partition_point(|token| token.span.end <= found.raw_start);
    let Some(event) = tokens.get(event_index) else {
        return false;
    };
    if event.span.start > found.raw_start || found.raw_end > event.span.end {
        return false;
    }
    let end = event_index + 1;
    if phrase.len() > end {
        return false;
    }
    let start = end - phrase.len();
    let candidate = &tokens[start..end];

    candidate
        .iter()
        .all(|token| token.clause == tokens[event_index].clause)
        && candidate
            .iter()
            .map(|token| token.text.as_str())
            .eq(phrase.iter().copied())
}

/// Returns the canonical bytes for one complete rule-channel behavior identity.
#[must_use]
pub fn canonical_rule_identity(language: Language) -> Vec<u8> {
    let body = if language == Language::Es {
        let pack = for_language(language.code()).expect("Spanish rules exist");
        canonical_legacy_identity(&pack)
    } else {
        let rules = word_rules(language)
            .or_else(|| arabic_hindi_rules(language))
            .or_else(|| cjk_rules(language))
            .expect("every non-Spanish language has V2 rules");
        canonical_rule_identity_for(rules)
    };
    let exclusions = lexical_collision_exclusions(language.storage_code());
    let mut output = Vec::new();
    output.extend_from_slice(CHANNEL_IDENTITY_MAGIC);
    encode_bytes(&mut output, language.code().as_bytes());
    encode_bytes(&mut output, &body);
    output.extend_from_slice(
        &u32::try_from(exclusions.len())
            .expect("collision exclusion count fits in u32")
            .to_le_bytes(),
    );
    for exclusion in exclusions {
        encode_bytes(&mut output, exclusion.as_bytes());
    }
    let reactivations = exclusions
        .iter()
        .filter_map(|exclusion| {
            lexical_collision_reactivation_phrase(language.storage_code(), exclusion)
                .map(|phrase| (*exclusion, phrase))
        })
        .collect::<Vec<_>>();
    if !reactivations.is_empty() {
        output.extend_from_slice(b"REACT");
        output.extend_from_slice(
            &u32::try_from(reactivations.len())
                .expect("collision reactivation count fits in u32")
                .to_le_bytes(),
        );
        for (lemma, phrase) in reactivations {
            encode_bytes(&mut output, lemma.as_bytes());
            output.extend_from_slice(
                &u32::try_from(phrase.len())
                    .expect("collision reactivation phrase length fits in u32")
                    .to_le_bytes(),
            );
            for token in phrase {
                encode_bytes(&mut output, token.as_bytes());
            }
        }
    }
    output
}

fn canonical_legacy_identity(pack: &RulePack) -> Vec<u8> {
    let fields: [&[Vec<String>]; 23] = [
        &pack.targets,
        &pack.groups,
        &pack.identity_links,
        &pack.negators,
        &pack.threats,
        &pack.intent,
        &pack.reports,
        &pack.counterspeech,
        &pack.positive,
        &pack.negative,
        &pack.intensifiers,
        &pack.diminishers,
        &pack.semantic.implicit_targets,
        &pack.semantic.implicit_threats,
        &pack.semantic.self_harm_commands,
        &pack.semantic.wish_markers,
        &pack.semantic.clause_initial_wish_markers,
        &pack.semantic.harm_outcomes,
        &pack.semantic.implicit_harm_outcomes,
        &pack.semantic.benign_harm_phrases,
        &pack.semantic.long_scope_negators,
        &pack.semantic.directed_hostility,
        &pack.semantic.group_hostility,
    ];
    let mut output = Vec::new();
    output.extend_from_slice(LEGACY_IDENTITY_MAGIC);
    encode_bytes(&mut output, pack.language.as_bytes());
    output.extend_from_slice(&1_u16.to_le_bytes());
    for (ordinal, phrases) in fields.into_iter().enumerate() {
        output.push(u8::try_from(ordinal).expect("legacy rule field ordinal fits in u8"));
        output.extend_from_slice(
            &u32::try_from(phrases.len())
                .expect("legacy phrase count fits in u32")
                .to_le_bytes(),
        );
        for phrase in phrases {
            output.extend_from_slice(
                &u32::try_from(phrase.len())
                    .expect("legacy token count fits in u32")
                    .to_le_bytes(),
            );
            for token in phrase {
                encode_bytes(&mut output, token.as_bytes());
            }
        }
    }
    output
}

fn encode_bytes(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(
        &u32::try_from(bytes.len())
            .expect("identity field length fits in u32")
            .to_le_bytes(),
    );
    output.extend_from_slice(bytes);
}
