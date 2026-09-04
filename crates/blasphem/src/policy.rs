use std::{collections::BTreeMap, fmt, ops::Range};

use crate::rule_pack::{
    RulePack, for_language, lexical_collision_excluded, lexical_collision_reactivation_phrase,
};
use crate::text::ContextToken;
use crate::{CandidateViewKind, Detection, Detector, MatchLevel, RuleOutcome, TextDocument};
use unicode_normalization::UnicodeNormalization;

const CONSERVATIVE_PROFANITY: u8 = 30;
const INCLUSIVE_PROFANITY: u8 = 20;
const EVASION_BONUS: u8 = 5;
const SUPPRESSED_PROFANITY: u8 = 10;
const TARGETED_ABUSE: u8 = 70;
const DIRECTED_HOSTILITY: u8 = 60;
const IDENTITY_ATTACK: u8 = 85;
const HOSTILE_WISH: u8 = 85;
const THREAT_LANGUAGE: u8 = 95;
const MAX_SENTIMENT_SUPPORT: u8 = 8;
const NUDGE_THRESHOLD: u8 = 50;

#[derive(Clone, Copy, Default)]
struct ClauseSentiment {
    support: u8,
    negative_lexical: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReplyTarget {
    #[default]
    Unknown,
    Person,
    ProtectedGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleContext<'a> {
    pub language: Option<&'a str>,
    pub reply_target: ReplyTarget,
}

impl<'a> RuleContext<'a> {
    #[must_use]
    pub fn for_language(language: &'a str) -> Self {
        Self {
            language: Some(language),
            reply_target: ReplyTarget::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CategoryScores {
    pub profanity: u8,
    pub targeted_abuse: u8,
    pub identity_attack: u8,
    pub threat_language: u8,
    pub sentiment_support: u8,
}

/// The small product result for a pre-send message nudge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NudgeResult {
    /// An ordinal risk score from 0 through 100.
    pub score: u8,
    /// The score that activates the nudge.
    pub threshold: u8,
    /// Whether the product should show the nudge.
    pub should_nudge: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyAction {
    Allow,
    Review,
    Block,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyCategory {
    Profanity,
    TargetedAbuse,
    IdentityAttack,
    ThreatLanguage,
    SentimentSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleId {
    LexicalMatch,
    LexicalCollisionExcluded,
    TargetedLexicalMatch,
    ReplyTargetedLexicalMatch,
    DirectThreat,
    ThreatIntentMarker,
    HostileWish,
    SelfHarmCommand,
    ImplicitTargetedLexicalMatch,
    SemanticDirectedHostility,
    SemanticGroupHostility,
    IdentityGroupTarget,
    IdentityStereotypeSupport,
    NegatedEvidence,
    QuotedEvidence,
    ReportedEvidence,
    CounterspeechEvidence,
    NegativeSentiment,
    CapsSupport,
    PunctuationSupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleEvidence {
    pub rule_id: RuleId,
    pub category: PolicyCategory,
    pub points: u8,
    pub language: Option<String>,
    pub matched_text: String,
    pub candidate_view: Option<CandidateViewKind>,
    pub normalized_start: Option<usize>,
    pub normalized_end: Option<usize>,
    pub raw_start: Option<usize>,
    pub raw_end: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyResult {
    pub original_text: String,
    pub lexical: Detection,
    pub scores: CategoryScores,
    pub sparse_score: Option<u8>,
    pub action: PolicyAction,
    pub evidence: Vec<RuleEvidence>,
}

impl PolicyResult {
    #[must_use]
    pub fn has_rule(&self, rule_id: RuleId) -> bool {
        self.evidence.iter().any(|item| item.rule_id == rule_id)
    }

    #[must_use]
    pub fn max_risk_points(&self) -> u8 {
        [
            self.scores.profanity,
            self.scores.targeted_abuse,
            self.scores.identity_attack,
            self.scores.threat_language,
            self.scores.sentiment_support,
        ]
        .into_iter()
        .max()
        .unwrap_or(0)
    }

    /// Returns the product-facing nudge result.
    #[must_use]
    pub fn nudge(&self) -> NudgeResult {
        let score = self.sparse_score.map_or_else(
            || self.max_risk_points(),
            |sparse| sparse.max(self.max_risk_points()),
        );
        NudgeResult {
            score,
            threshold: NUDGE_THRESHOLD,
            should_nudge: score >= NUDGE_THRESHOLD,
        }
    }
}

macro_rules! display_enum {
    ($type:ty, {$($variant:path => $text:literal),+ $(,)?}) => {
        impl fmt::Display for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(match self {
                    $($variant => $text),+
                })
            }
        }
    };
}

display_enum!(PolicyAction, {
    PolicyAction::Allow => "allow",
    PolicyAction::Review => "review",
    PolicyAction::Block => "block",
});
display_enum!(PolicyCategory, {
    PolicyCategory::Profanity => "profanity",
    PolicyCategory::TargetedAbuse => "targeted_abuse",
    PolicyCategory::IdentityAttack => "identity_attack",
    PolicyCategory::ThreatLanguage => "threat_language",
    PolicyCategory::SentimentSupport => "sentiment_support",
});
display_enum!(RuleId, {
    RuleId::LexicalMatch => "lexical_match",
    RuleId::LexicalCollisionExcluded => "lexical_collision_excluded",
    RuleId::TargetedLexicalMatch => "targeted_lexical_match",
    RuleId::ReplyTargetedLexicalMatch => "reply_targeted_lexical_match",
    RuleId::DirectThreat => "direct_threat",
    RuleId::ThreatIntentMarker => "threat_intent_marker",
    RuleId::HostileWish => "hostile_wish",
    RuleId::SelfHarmCommand => "self_harm_command",
    RuleId::ImplicitTargetedLexicalMatch => "implicit_targeted_lexical_match",
    RuleId::SemanticDirectedHostility => "semantic_directed_hostility",
    RuleId::SemanticGroupHostility => "semantic_group_hostility",
    RuleId::IdentityGroupTarget => "identity_group_target",
    RuleId::IdentityStereotypeSupport => "identity_stereotype_support",
    RuleId::NegatedEvidence => "negated_evidence",
    RuleId::QuotedEvidence => "quoted_evidence",
    RuleId::ReportedEvidence => "reported_evidence",
    RuleId::CounterspeechEvidence => "counterspeech_evidence",
    RuleId::NegativeSentiment => "negative_sentiment",
    RuleId::CapsSupport => "caps_support",
    RuleId::PunctuationSupport => "punctuation_support",
});

pub(crate) fn analyze_rules(
    detector: &Detector,
    text: &str,
    context: RuleContext<'_>,
) -> PolicyResult {
    let pack = context.language.and_then(for_language);
    analyze_rule_pack(
        detector,
        text,
        ResolvedRuleContext {
            context,
            pack: pack.as_ref(),
        },
    )
}

pub(crate) struct ResolvedRuleContext<'a> {
    pub context: RuleContext<'a>,
    pub pack: Option<&'a RulePack>,
}

/// Reads one category score.
const fn category_score(scores: &CategoryScores, category: PolicyCategory) -> u8 {
    match category {
        PolicyCategory::Profanity => scores.profanity,
        PolicyCategory::TargetedAbuse => scores.targeted_abuse,
        PolicyCategory::IdentityAttack => scores.identity_attack,
        PolicyCategory::ThreatLanguage => scores.threat_language,
        PolicyCategory::SentimentSupport => scores.sentiment_support,
    }
}

/// Rebuilds the category scores of a rule channel that keeps the highest point of each category.
///
/// The clause rules of the fourteen v2 languages combine with `max`. The legacy Spanish policy
/// accumulates instead, at `score_lexical` and `score_threats`, so it reports its own scores.
pub(crate) fn category_scores_from_evidence(evidence: &[RuleEvidence]) -> CategoryScores {
    let mut scores = CategoryScores::default();
    for item in evidence {
        let score = match item.category {
            PolicyCategory::Profanity => &mut scores.profanity,
            PolicyCategory::TargetedAbuse => &mut scores.targeted_abuse,
            PolicyCategory::IdentityAttack => &mut scores.identity_attack,
            PolicyCategory::ThreatLanguage => &mut scores.threat_language,
            PolicyCategory::SentimentSupport => &mut scores.sentiment_support,
        };
        *score = (*score).max(item.points);
    }
    scores
}

pub(crate) fn policy_result_from_rule_channel(
    text: &str,
    lexical: Detection,
    outcome: RuleOutcome,
    scores: CategoryScores,
    sparse_score: Option<u8>,
) -> PolicyResult {
    debug_assert_eq!(
        outcome.score,
        [
            scores.profanity,
            scores.targeted_abuse,
            scores.identity_attack,
            scores.threat_language,
            scores.sentiment_support,
        ]
        .into_iter()
        .max()
        .unwrap_or(0),
    );
    debug_assert!(
        outcome
            .evidence
            .iter()
            .all(|item| item.points <= category_score(&scores, item.category)),
        "a rule channel reports an evidence point above its own category score",
    );
    let action = select_action(scores);

    PolicyResult {
        original_text: text.to_owned(),
        lexical,
        scores,
        sparse_score,
        action,
        evidence: outcome.evidence,
    }
}

pub(crate) fn analyze_rule_pack(
    detector: &Detector,
    text: &str,
    resolved: ResolvedRuleContext<'_>,
) -> PolicyResult {
    let ResolvedRuleContext { context, pack } = resolved;
    let lexical = detector.check(text);
    let document = TextDocument::new(text);
    let mut scores = CategoryScores::default();
    let mut evidence = Vec::new();

    let auto_lexical_mode = context.language.is_none();
    let clause_support = pack.map_or_else(BTreeMap::new, |pack| {
        score_sentiment(text, &document, pack, &mut scores, &mut evidence)
    });
    score_lexical(
        text,
        &document,
        &lexical,
        pack,
        auto_lexical_mode,
        context.reply_target,
        &clause_support,
        &mut scores,
        &mut evidence,
    );

    if let Some(pack) = pack {
        score_threats(
            text,
            &document,
            pack,
            context.reply_target,
            &clause_support,
            &mut scores,
            &mut evidence,
        );
        score_semantic_events(text, &document, pack, &mut scores, &mut evidence);
        score_compact_events(text, pack, &mut scores, &mut evidence);
    }

    let action = select_action(scores);
    PolicyResult {
        original_text: text.to_owned(),
        lexical,
        scores,
        sparse_score: None,
        action,
        evidence,
    }
}

#[allow(clippy::too_many_arguments)]
fn score_lexical(
    text: &str,
    document: &TextDocument,
    lexical: &Detection,
    pack: Option<&RulePack>,
    auto_lexical_mode: bool,
    reply_target: ReplyTarget,
    clause_support: &BTreeMap<usize, ClauseSentiment>,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let mut groups = BTreeMap::new();
    let mut collision_cache = BTreeMap::new();
    let mut suppressed_profanity = 0;
    for found in &lexical.matches {
        groups
            .entry((found.raw_start, found.raw_end))
            .or_insert_with(Vec::new)
            .push(found);
    }

    for ((raw_start, raw_end), matches) in groups {
        let token_range = token_range_for_span(document.context_tokens(), raw_start, raw_end);
        let (excluded_matches, active_matches): (Vec<_>, Vec<_>) =
            matches.iter().copied().partition(|found| {
                lexical_event_excluded(
                    found,
                    pack,
                    auto_lexical_mode,
                    document.context_tokens(),
                    token_range.clone(),
                    &mut collision_cache,
                )
            });
        let mut excluded_representatives: BTreeMap<String, &crate::LexiconMatch> = BTreeMap::new();
        for found in excluded_matches {
            let language = found.entry.language.to_ascii_uppercase();
            excluded_representatives
                .entry(language)
                .and_modify(|representative| {
                    if compare_lexicon_matches(found, representative).is_gt() {
                        *representative = found;
                    }
                })
                .or_insert(found);
        }
        for (language, representative) in excluded_representatives {
            evidence.push(lexical_evidence(
                RuleId::LexicalCollisionExcluded,
                PolicyCategory::Profanity,
                0,
                Some(language),
                text,
                representative,
            ));
        }
        let Some(representative) = active_matches
            .iter()
            .copied()
            .max_by(|left, right| compare_lexicon_matches(left, right))
        else {
            continue;
        };
        let context_representative = pack.and_then(|pack| {
            active_matches
                .iter()
                .copied()
                .filter(|found| found.entry.language.eq_ignore_ascii_case(pack.language))
                .max_by(|left, right| compare_lexicon_matches(left, right))
        });
        let context_applies = context_representative.is_some();
        let context_representative = context_representative.unwrap_or(representative);
        let suppression = if context_applies {
            token_range
                .clone()
                .map(|range| suppression(document.context_tokens(), range, pack.expect("checked")))
        } else {
            None
        };
        let is_suppressed = suppression.as_ref().is_some_and(|scope| scope.any());
        let mut profanity = lexical_match_points(representative);
        if is_suppressed {
            profanity = profanity.min(SUPPRESSED_PROFANITY);
            let remaining = SUPPRESSED_PROFANITY.saturating_sub(suppressed_profanity);
            profanity = profanity.min(remaining);
            suppressed_profanity = suppressed_profanity.saturating_add(profanity);
        }
        scores.profanity = scores.profanity.max(profanity);
        evidence.push(lexical_evidence(
            RuleId::LexicalMatch,
            PolicyCategory::Profanity,
            profanity,
            Some(representative.entry.language.clone()),
            text,
            representative,
        ));

        let (Some(pack), Some(token_range)) = (pack, token_range) else {
            continue;
        };
        if !context_applies {
            continue;
        }
        if let Some(suppression) = suppression {
            append_suppression_evidence(
                suppression,
                PolicyCategory::Profanity,
                pack.language,
                text,
                raw_start,
                raw_end,
                Some(context_representative),
                evidence,
            );
            if is_suppressed {
                continue;
            }
        }

        let tokens = document.context_tokens();
        let sentiment = clause_support
            .get(&tokens[token_range.start].clause)
            .copied()
            .unwrap_or_default();
        let support = sentiment.support.min(5);
        let person_target = reply_target == ReplyTarget::Person
            || nearby_phrase(tokens, token_range.clone(), 4, &pack.targets).is_some()
            || nearby_phrase(
                tokens,
                token_range.clone(),
                4,
                &pack.semantic.implicit_targets,
            )
            .is_some()
            || nearby_mention(tokens, token_range.clone(), 4);
        if person_target {
            scores.targeted_abuse = scores
                .targeted_abuse
                .saturating_add(TARGETED_ABUSE.saturating_add(support))
                .min(100);
            let rule_id = if reply_target == ReplyTarget::Person {
                RuleId::ReplyTargetedLexicalMatch
            } else if nearby_phrase(
                tokens,
                token_range.clone(),
                4,
                &pack.semantic.implicit_targets,
            )
            .is_some()
            {
                RuleId::ImplicitTargetedLexicalMatch
            } else {
                RuleId::TargetedLexicalMatch
            };
            evidence.push(lexical_evidence(
                rule_id,
                PolicyCategory::TargetedAbuse,
                TARGETED_ABUSE,
                Some(pack.language.to_owned()),
                text,
                context_representative,
            ));
        }

        let categorized_identity_representative = active_matches
            .iter()
            .copied()
            .filter(|found| {
                found.entry.language.eq_ignore_ascii_case(pack.language)
                    && matches!(
                        found.entry.category.to_ascii_lowercase().as_str(),
                        "ps" | "rci" | "om" | "ddf" | "ddp"
                    )
            })
            .max_by(|left, right| compare_lexicon_matches(left, right));
        let stereotype_representative = active_matches
            .iter()
            .copied()
            .filter(|found| {
                found.entry.language.eq_ignore_ascii_case(pack.language) && found.entry.stereotype
            })
            .max_by(|left, right| compare_lexicon_matches(left, right));
        let protected_group_reply = reply_target == ReplyTarget::ProtectedGroup;
        let nearby_groups = nearby_phrases(tokens, token_range.clone(), 4, &pack.groups);
        let group_target = protected_group_reply || !nearby_groups.is_empty();
        let direct_relation = protected_group_reply
            || nearby_groups.iter().any(|group_range| {
                direct_identity_relation(
                    text,
                    tokens,
                    group_range.clone(),
                    token_range.clone(),
                    pack.language,
                    &pack.identity_links,
                )
            });
        let identity_representative = categorized_identity_representative
            .or_else(|| direct_relation.then_some(context_representative));
        if let Some(identity_representative) = identity_representative
            && group_target
            && (direct_relation || sentiment.negative_lexical)
        {
            scores.identity_attack = scores
                .identity_attack
                .saturating_add(IDENTITY_ATTACK.saturating_add(support))
                .min(100);
            evidence.push(lexical_evidence(
                RuleId::IdentityGroupTarget,
                PolicyCategory::IdentityAttack,
                IDENTITY_ATTACK,
                Some(pack.language.to_owned()),
                text,
                identity_representative,
            ));
            if let Some(stereotype_representative) = stereotype_representative {
                evidence.push(lexical_evidence(
                    RuleId::IdentityStereotypeSupport,
                    PolicyCategory::IdentityAttack,
                    0,
                    Some(pack.language.to_owned()),
                    text,
                    stereotype_representative,
                ));
            }
        }
    }
}

fn lexical_event_excluded<'a>(
    found: &'a crate::LexiconMatch,
    pack: Option<&RulePack>,
    auto_lexical_mode: bool,
    tokens: &[ContextToken],
    event: Option<Range<usize>>,
    collision_cache: &mut BTreeMap<(&'a str, &'a str), CollisionPolicy>,
) -> bool {
    let collision_policy = *collision_cache
        .entry((found.entry.language.as_str(), found.entry.lemma.as_str()))
        .or_insert_with(|| {
            (
                lexical_collision_excluded(&found.entry.language, &found.entry.lemma),
                lexical_collision_reactivation_phrase(&found.entry.language, &found.entry.lemma),
            )
        });
    if !collision_policy.0 {
        return false;
    }
    if auto_lexical_mode {
        return true;
    }
    let Some(pack) = pack else {
        return false;
    };
    if !found.entry.language.eq_ignore_ascii_case(pack.language) {
        return false;
    }

    !event
        .is_some_and(|event| collision_reactivated(found, pack, tokens, event, collision_policy.1))
}

type CollisionPolicy = (bool, Option<&'static [&'static str]>);

fn collision_reactivated(
    found: &crate::LexiconMatch,
    pack: &RulePack,
    tokens: &[ContextToken],
    event: Range<usize>,
    phrase: Option<&[&str]>,
) -> bool {
    let Some(phrase) = phrase else {
        return false;
    };
    if !found.entry.language.eq_ignore_ascii_case(pack.language) || phrase.len() > event.end {
        return false;
    }
    let start = event.end - phrase.len();
    let Some(candidate) = tokens.get(start..event.end) else {
        return false;
    };

    start < event.start
        && candidate
            .iter()
            .all(|token| token.clause == tokens[event.start].clause)
        && candidate
            .iter()
            .map(|token| token.text.as_str())
            .eq(phrase.iter().copied())
}

fn lexical_match_points(found: &crate::LexiconMatch) -> u8 {
    let base = match found.entry.level {
        MatchLevel::Conservative => CONSERVATIVE_PROFANITY,
        MatchLevel::Inclusive => INCLUSIVE_PROFANITY,
    };
    if found.view == CandidateViewKind::Normalized {
        base
    } else {
        base.saturating_add(EVASION_BONUS)
    }
}

fn compare_lexicon_matches(
    left: &crate::LexiconMatch,
    right: &crate::LexiconMatch,
) -> std::cmp::Ordering {
    lexical_match_points(left)
        .cmp(&lexical_match_points(right))
        .then_with(|| candidate_view_priority(left.view).cmp(&candidate_view_priority(right.view)))
        .then_with(|| left.entry.language.cmp(&right.entry.language))
        .then_with(|| left.entry.id.cmp(&right.entry.id))
}

const fn candidate_view_priority(view: CandidateViewKind) -> u8 {
    match view {
        CandidateViewKind::Normalized => 2,
        CandidateViewKind::Confusable => 1,
        CandidateViewKind::Evasion => 0,
    }
}

fn score_threats(
    text: &str,
    document: &TextDocument,
    pack: &RulePack,
    reply_target: ReplyTarget,
    clause_support: &BTreeMap<usize, ClauseSentiment>,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let tokens = document.context_tokens();
    let mut index = 0;
    while index < tokens.len() {
        let Some(threat_len) = phrase_len_at(tokens, index, &pack.threats) else {
            index += 1;
            continue;
        };
        let threat_range = index..index + threat_len;
        if phrase_len_at(tokens, index, &pack.semantic.benign_harm_phrases).is_some() {
            index = threat_range.end;
            continue;
        }
        let token = &tokens[index];
        let threat_end = tokens[threat_range.end - 1].span.end;
        let target_ranges = nearby_phrases(tokens, threat_range.clone(), 5, &pack.targets);
        let target_range = target_ranges.first().cloned();
        let has_target = reply_target == ReplyTarget::Person
            || target_range.is_some()
            || nearby_mention(tokens, threat_range.clone(), 5);
        if !has_target {
            index = threat_range.end;
            continue;
        }
        let intent_range = nearby_phrase(tokens, threat_range.clone(), 3, &pack.intent);
        let target_follows_threat = target_ranges
            .iter()
            .any(|target| target.start >= threat_range.end);
        if reply_target != ReplyTarget::Person && !target_follows_threat && intent_range.is_none() {
            index = threat_range.end;
            continue;
        }
        let mut frame_start = threat_range.start;
        let mut frame_end = threat_range.end;
        for range in [target_range.as_ref(), intent_range.as_ref()]
            .into_iter()
            .flatten()
        {
            frame_start = frame_start.min(range.start);
            frame_end = frame_end.max(range.end);
        }
        let suppression = suppression(tokens, frame_start..frame_end, pack);
        if suppression.any() {
            append_suppression_evidence(
                suppression,
                PolicyCategory::ThreatLanguage,
                pack.language,
                text,
                token.span.start,
                threat_end,
                None,
                evidence,
            );
            index = threat_range.end;
            continue;
        }
        scores.threat_language = scores
            .threat_language
            .saturating_add(
                THREAT_LANGUAGE.saturating_add(
                    clause_support
                        .get(&token.clause)
                        .copied()
                        .unwrap_or_default()
                        .support
                        .min(5),
                ),
            )
            .min(100);
        evidence.push(rule_span_evidence(
            RuleId::DirectThreat,
            PolicyCategory::ThreatLanguage,
            THREAT_LANGUAGE,
            Some(pack.language.to_owned()),
            text,
            token.span.start,
            threat_end,
        ));
        if let Some(intent_range) = intent_range {
            let intent = &tokens[intent_range.start];
            let intent_end = tokens[intent_range.end - 1].span.end;
            evidence.push(rule_span_evidence(
                RuleId::ThreatIntentMarker,
                PolicyCategory::ThreatLanguage,
                0,
                Some(pack.language.to_owned()),
                text,
                intent.span.start,
                intent_end,
            ));
        }
        index = threat_range.end;
    }
}

fn score_semantic_events(
    text: &str,
    document: &TextDocument,
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let tokens = document.context_tokens();
    score_implicit_threats(text, tokens, pack, scores, evidence);
    score_self_harm_commands(text, tokens, pack, scores, evidence);
    score_hostile_wishes(text, tokens, pack, scores, evidence);
    score_directed_hostility(text, tokens, pack, scores, evidence);
    score_group_hostility(text, tokens, pack, scores, evidence);
}

fn score_compact_events(
    text: &str,
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    for clause in compact_clause_ranges(text) {
        let clause_text = &text[clause.clone()];
        let compact = compact_text(clause_text);
        let Some((direct_threat, hostile_wish)) = compact_event_flags(pack.language, &compact)
        else {
            return;
        };

        if direct_threat {
            score_compact_event(
                text,
                clause.clone(),
                &compact,
                pack,
                RuleId::DirectThreat,
                THREAT_LANGUAGE,
                true,
                scores,
                evidence,
            );
        }
        if hostile_wish {
            score_compact_event(
                text,
                clause,
                &compact,
                pack,
                RuleId::HostileWish,
                HOSTILE_WISH,
                false,
                scores,
                evidence,
            );
        }
    }
}

fn compact_event_flags(language: &str, compact: &str) -> Option<(bool, bool)> {
    Some(match language {
        "ZH" => {
            let target = contains_any(compact, &["你", "你们", "你家人", "你全家"]);
            let intent = contains_any(compact, &["我会", "我要", "我将"])
                || (compact.contains("等我") && compact.contains("就"));
            let lethal = contains_any(compact, &["杀了", "弄死", "打死", "捅死", "砍死"]);
            let body_harm =
                compact.contains("打断") && contains_any(compact, &["腿", "手", "骨头", "牙"]);
            let wish = target
                && contains_any(compact, &["希望", "巴不得", "最好"])
                && contains_any(compact, &["去死", "死掉", "再也醒不过来", "永远醒不过来"]);
            (target && intent && (lethal || body_harm), wish)
        }
        "JA" => {
            let target = contains_any(
                compact,
                &["お前", "あなた", "あんた", "君", "てめえ", "貴様"],
            );
            let explicit =
                target && contains_any(compact, &["殺してやる", "殴ってやる", "ぶっ殺す"]);
            let conditional = contains_any(compact, &["今度会ったら", "見つけたら", "捕まえたら"])
                && contains_any(compact, &["殺す", "殴る", "刺す", "骨を折る", "歯を折る"]);
            let wish = (target || compact.starts_with("死ねばいい"))
                && contains_any(
                    compact,
                    &[
                        "死ねばいい",
                        "死んでほしい",
                        "消えてほしい",
                        "二度と目を覚まさなければいい",
                    ],
                );
            (explicit || conditional, wish)
        }
        "KO" => {
            let target = contains_any(
                compact,
                &["너", "네가", "널", "너를", "니가", "당신", "너희"],
            );
            let direct = target
                && contains_any(
                    compact,
                    &[
                        "죽이겠다",
                        "죽여버릴거야",
                        "때려죽일거야",
                        "부러뜨리겠다",
                        "부러뜨릴거야",
                    ],
                );
            let wish = target
                && contains_any(
                    compact,
                    &[
                        "죽기를",
                        "죽길",
                        "죽었으면",
                        "다시는깨어나지않기를",
                        "사라졌으면",
                    ],
                )
                && contains_any(compact, &["바란다", "좋겠다", "빌어"]);
            (direct, wish)
        }
        "EN" | "ES" | "AR" | "MS" | "ID" | "PT" | "FR" | "HI" | "RU" | "DE" | "TR" | "VI"
        | "IT" => {
            return None;
        }
        _ => return None,
    })
}

#[allow(clippy::too_many_arguments)]
fn score_compact_event(
    text: &str,
    clause: Range<usize>,
    compact: &str,
    pack: &RulePack,
    rule_id: RuleId,
    points: u8,
    apply_negation: bool,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let (report_content, report_actions, negators, counterspeech) = match pack.language {
        "ZH" => (
            &["消息", "帖子", "评论"][..],
            &["删除", "举报", "报告", "封禁"][..],
            &["不会", "不要", "别"][..],
            &["停止", "不要威胁"][..],
        ),
        "JA" => (
            &["投稿", "メッセージ", "コメント"][..],
            &["削除", "通報", "報告", "凍結"][..],
            &["つもりはない", "殴らない", "殺さない"][..],
            &["やめろ", "脅すのはやめ"][..],
        ),
        "KO" => (
            &["메시지", "게시물", "댓글"][..],
            &["삭제", "신고", "차단"][..],
            &["생각이없다", "죽이지않", "해치지않"][..],
            &["하지마", "협박하지마"][..],
        ),
        "EN" | "ES" | "AR" | "MS" | "ID" | "PT" | "FR" | "HI" | "RU" | "DE" | "TR" | "VI"
        | "IT" => return,
        _ => return,
    };
    let suppression = Suppression {
        negated: apply_negation && contains_any(compact, negators),
        quoted: has_complete_quote_pair(&text[clause.clone()]),
        reported: contains_any(compact, report_content) && contains_any(compact, report_actions),
        counterspeech: contains_any(compact, counterspeech),
    };
    if suppression.any() {
        append_suppression_evidence(
            suppression,
            PolicyCategory::ThreatLanguage,
            pack.language,
            text,
            clause.start,
            clause.end,
            None,
            evidence,
        );
        return;
    }

    scores.threat_language = scores.threat_language.max(points);
    evidence.push(rule_span_evidence(
        rule_id,
        PolicyCategory::ThreatLanguage,
        points,
        Some(pack.language.to_owned()),
        text,
        clause.start,
        clause.end,
    ));
}

fn compact_clause_ranges(text: &str) -> Vec<Range<usize>> {
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if matches!(
            character,
            '.' | '!' | '?' | ',' | ';' | '。' | '！' | '？' | '、' | '，' | '；' | '\n' | '\r'
        ) {
            if start < index {
                clauses.push(start..index);
            }
            start = index + character.len_utf8();
        }
    }
    if start < text.len() {
        clauses.push(start..text.len());
    }
    clauses
}

fn compact_text(text: &str) -> String {
    text.nfkc()
        .flat_map(char::to_lowercase)
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

fn has_complete_quote_pair(text: &str) -> bool {
    [
        ('"', '"'),
        ('“', '”'),
        ('«', '»'),
        ('「', '」'),
        ('『', '』'),
        ('„', '“'),
    ]
    .into_iter()
    .any(|(opening, closing)| {
        let Some(start) = text.find(opening) else {
            return false;
        };
        let remainder = &text[start + opening.len_utf8()..];
        remainder.contains(closing)
    })
}

fn score_implicit_threats(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    for event in phrase_ranges(tokens, &pack.semantic.implicit_threats) {
        score_semantic_threat(
            text,
            tokens,
            pack,
            event,
            RuleId::DirectThreat,
            THREAT_LANGUAGE,
            scores,
            evidence,
        );
    }
}

fn score_self_harm_commands(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    for event in phrase_ranges(tokens, &pack.semantic.self_harm_commands) {
        score_semantic_threat(
            text,
            tokens,
            pack,
            event,
            RuleId::SelfHarmCommand,
            THREAT_LANGUAGE,
            scores,
            evidence,
        );
    }
}

fn score_hostile_wishes(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let mut markers = phrase_ranges(tokens, &pack.semantic.wish_markers);
    markers.extend(
        phrase_ranges(tokens, &pack.semantic.clause_initial_wish_markers)
            .into_iter()
            .filter(|range| is_clause_start(tokens, range.start)),
    );
    markers.sort_by_key(|range| (range.start, range.end));
    markers.dedup_by_key(|range| (range.start, range.end));

    for marker in markers {
        let Some((outcome, implicit_target)) = following_harm_outcome(tokens, marker.clone(), pack)
        else {
            continue;
        };
        let explicit_target = nearby_phrase(tokens, outcome.clone(), 5, &pack.targets).is_some()
            || nearby_mention(tokens, outcome.clone(), 5);
        if !implicit_target && !explicit_target {
            continue;
        }

        score_hostile_wish(
            text,
            tokens,
            pack,
            marker.start..outcome.end,
            outcome.start,
            scores,
            evidence,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn score_hostile_wish(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    event: Range<usize>,
    outcome_start: usize,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let event_start = tokens[event.start].span.start;
    let event_end = tokens[event.end - 1].span.end;
    let mut event_suppression = suppression(tokens, event, pack);
    event_suppression.negated = preceding_phrase_in_scope(tokens, outcome_start, 3, &pack.negators);
    if event_suppression.any() {
        append_suppression_evidence(
            event_suppression,
            PolicyCategory::ThreatLanguage,
            pack.language,
            text,
            event_start,
            event_end,
            None,
            evidence,
        );
        return;
    }

    scores.threat_language = scores.threat_language.max(HOSTILE_WISH);
    evidence.push(rule_span_evidence(
        RuleId::HostileWish,
        PolicyCategory::ThreatLanguage,
        HOSTILE_WISH,
        Some(pack.language.to_owned()),
        text,
        event_start,
        event_end,
    ));
}

fn score_group_hostility(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    for predicate in phrase_ranges(tokens, &pack.semantic.group_hostility) {
        let group = nearby_phrases(tokens, predicate.clone(), 4, &pack.groups)
            .into_iter()
            .filter(|candidate| candidate.end <= predicate.start)
            .max_by_key(|candidate| candidate.end);
        let Some(group) = group else {
            continue;
        };
        let event = group.start..predicate.end;
        let event_start = tokens[event.start].span.start;
        let event_end = tokens[event.end - 1].span.end;
        let event_suppression = suppression(tokens, event, pack);
        if event_suppression.any() {
            append_suppression_evidence(
                event_suppression,
                PolicyCategory::IdentityAttack,
                pack.language,
                text,
                event_start,
                event_end,
                None,
                evidence,
            );
            continue;
        }

        scores.identity_attack = scores.identity_attack.max(IDENTITY_ATTACK);
        evidence.push(rule_span_evidence(
            RuleId::SemanticGroupHostility,
            PolicyCategory::IdentityAttack,
            IDENTITY_ATTACK,
            Some(pack.language.to_owned()),
            text,
            event_start,
            event_end,
        ));
    }
}

fn score_directed_hostility(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    for event in phrase_ranges(tokens, &pack.semantic.directed_hostility) {
        let event_start = tokens[event.start].span.start;
        let event_end = tokens[event.end - 1].span.end;
        let mut event_suppression = suppression(tokens, event.clone(), pack);
        if overlapping_phrase(tokens, event.clone(), &pack.negators) {
            event_suppression.negated = preceding_phrase(tokens, event.start, 3, &pack.negators);
        }
        if event_suppression.any() {
            append_suppression_evidence(
                event_suppression,
                PolicyCategory::TargetedAbuse,
                pack.language,
                text,
                event_start,
                event_end,
                None,
                evidence,
            );
            continue;
        }

        scores.targeted_abuse = scores.targeted_abuse.max(DIRECTED_HOSTILITY);
        evidence.push(rule_span_evidence(
            RuleId::SemanticDirectedHostility,
            PolicyCategory::TargetedAbuse,
            DIRECTED_HOSTILITY,
            Some(pack.language.to_owned()),
            text,
            event_start,
            event_end,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn score_semantic_threat(
    text: &str,
    tokens: &[ContextToken],
    pack: &RulePack,
    event: Range<usize>,
    rule_id: RuleId,
    base_points: u8,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) {
    let event_start = tokens[event.start].span.start;
    let event_end = tokens[event.end - 1].span.end;
    let event_suppression = suppression(tokens, event.clone(), pack);
    if event_suppression.any() {
        append_suppression_evidence(
            event_suppression,
            PolicyCategory::ThreatLanguage,
            pack.language,
            text,
            event_start,
            event_end,
            None,
            evidence,
        );
        return;
    }

    scores.threat_language = scores.threat_language.max(base_points);
    evidence.push(rule_span_evidence(
        rule_id,
        PolicyCategory::ThreatLanguage,
        base_points,
        Some(pack.language.to_owned()),
        text,
        event_start,
        event_end,
    ));
}

fn following_harm_outcome(
    tokens: &[ContextToken],
    marker: Range<usize>,
    pack: &RulePack,
) -> Option<(Range<usize>, bool)> {
    let clause = tokens[marker.start].clause;
    let search_end = marker.end.saturating_add(8).min(tokens.len());
    for start in marker.end..search_end {
        if tokens[start].clause != clause {
            break;
        }
        if let Some(length) = phrase_len_at(tokens, start, &pack.semantic.implicit_harm_outcomes) {
            return Some((start..start + length, true));
        }
        if let Some(length) = phrase_len_at(tokens, start, &pack.semantic.harm_outcomes) {
            return Some((start..start + length, false));
        }
    }
    None
}

fn phrase_ranges(tokens: &[ContextToken], phrases: &[Vec<String>]) -> Vec<Range<usize>> {
    (0..tokens.len())
        .filter_map(|start| {
            phrase_len_at(tokens, start, phrases).map(|length| start..start + length)
        })
        .collect()
}

fn is_clause_start(tokens: &[ContextToken], index: usize) -> bool {
    index == 0 || tokens[index - 1].clause != tokens[index].clause
}

fn score_sentiment(
    text: &str,
    document: &TextDocument,
    pack: &RulePack,
    scores: &mut CategoryScores,
    evidence: &mut Vec<RuleEvidence>,
) -> BTreeMap<usize, ClauseSentiment> {
    let tokens = document.context_tokens();
    let mut totals = BTreeMap::<usize, i16>::new();
    let mut index = 0;
    while index < tokens.len() {
        let (mut value, phrase_len) =
            if let Some(length) = phrase_len_at(tokens, index, &pack.negative) {
                (1_i16, length)
            } else if let Some(length) = phrase_len_at(tokens, index, &pack.positive) {
                (-1_i16, length)
            } else {
                index += 1;
                continue;
            };
        let token = &tokens[index];
        if immediately_preceding_phrase(tokens, index, &pack.intensifiers) {
            value += value.signum();
        }
        if immediately_preceding_phrase(tokens, index, &pack.diminishers) {
            value -= value.signum();
        }
        if preceding_phrase(tokens, index, 3, &pack.negators) {
            value = -value;
        }
        *totals.entry(token.clause).or_default() += value;
        if value > 0 {
            evidence.push(rule_span_evidence(
                RuleId::NegativeSentiment,
                PolicyCategory::SentimentSupport,
                value as u8,
                Some(pack.language.to_owned()),
                text,
                token.span.start,
                tokens[index + phrase_len - 1].span.end,
            ));
        }
        index += phrase_len;
    }

    let mut supports = totals
        .into_iter()
        .map(|(clause, total)| {
            let support = total.clamp(0, i16::from(MAX_SENTIMENT_SUPPORT)) as u8;
            (
                clause,
                ClauseSentiment {
                    support,
                    negative_lexical: support > 0,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    for (clause, clause_text) in clause_texts(tokens, text) {
        let sentiment = supports.entry(clause).or_default();
        let uppercase = clause_text
            .chars()
            .filter(|character| character.is_uppercase())
            .count();
        let has_lowercase = clause_text
            .chars()
            .any(|character| character.is_lowercase());
        if uppercase >= 3 && !has_lowercase {
            sentiment.support = sentiment
                .support
                .saturating_add(1)
                .min(MAX_SENTIMENT_SUPPORT);
            evidence.push(global_evidence(
                RuleId::CapsSupport,
                PolicyCategory::SentimentSupport,
                1,
                pack.language,
                clause_text,
            ));
        }
        if ["!!", "??", "!?", "?!"]
            .iter()
            .any(|value| clause_text.contains(value))
        {
            sentiment.support = sentiment
                .support
                .saturating_add(1)
                .min(MAX_SENTIMENT_SUPPORT);
            evidence.push(global_evidence(
                RuleId::PunctuationSupport,
                PolicyCategory::SentimentSupport,
                1,
                pack.language,
                clause_text,
            ));
        }
    }
    scores.sentiment_support = supports
        .values()
        .map(|sentiment| sentiment.support)
        .max()
        .unwrap_or(0);
    supports
}

fn clause_texts<'a>(tokens: &[ContextToken], text: &'a str) -> Vec<(usize, &'a str)> {
    let mut starts = BTreeMap::new();
    for token in tokens {
        starts.entry(token.clause).or_insert(token.span.start);
    }
    let clauses = starts.into_iter().collect::<Vec<_>>();
    clauses
        .iter()
        .enumerate()
        .map(|(index, (clause, start))| {
            let end = clauses
                .get(index + 1)
                .map_or(text.len(), |(_, next_start)| *next_start);
            (*clause, &text[*start..end])
        })
        .collect()
}

fn select_action(scores: CategoryScores) -> PolicyAction {
    if scores.identity_attack > 0 || scores.threat_language > 0 {
        PolicyAction::Block
    } else if scores.targeted_abuse > 0 || scores.profanity >= 20 {
        PolicyAction::Review
    } else {
        PolicyAction::Allow
    }
}

#[derive(Clone, Copy)]
struct Suppression {
    negated: bool,
    quoted: bool,
    reported: bool,
    counterspeech: bool,
}

impl Suppression {
    fn any(self) -> bool {
        self.negated || self.quoted || self.reported || self.counterspeech
    }
}

fn suppression(tokens: &[ContextToken], event: Range<usize>, pack: &RulePack) -> Suppression {
    Suppression {
        negated: preceding_phrase_in_scope(tokens, event.start, 3, &pack.negators)
            || overlapping_phrase(tokens, event.clone(), &pack.negators)
            || preceding_phrase_in_scope(
                tokens,
                event.start,
                8,
                &pack.semantic.long_scope_negators,
            ),
        quoted: tokens[event.clone()].iter().any(|token| token.quoted),
        reported: preceding_phrase_in_scope(tokens, event.start, 3, &pack.reports),
        counterspeech: preceding_phrase(tokens, event.start, 4, &pack.counterspeech),
    }
}

fn overlapping_phrase(
    tokens: &[ContextToken],
    event: Range<usize>,
    phrases: &[Vec<String>],
) -> bool {
    let clause = tokens[event.start].clause;
    overlapping_candidate_starts(tokens.len(), event.clone(), phrases).any(|start| {
        phrase_len_at(tokens, start, phrases).is_some_and(|length| {
            let end = start + length;
            tokens[start].clause == clause && start < event.end && event.start < end
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn append_suppression_evidence(
    suppression: Suppression,
    category: PolicyCategory,
    language: &str,
    text: &str,
    start: usize,
    end: usize,
    source: Option<&crate::LexiconMatch>,
    evidence: &mut Vec<RuleEvidence>,
) {
    for (active, rule_id) in [
        (suppression.negated, RuleId::NegatedEvidence),
        (suppression.quoted, RuleId::QuotedEvidence),
        (suppression.reported, RuleId::ReportedEvidence),
        (suppression.counterspeech, RuleId::CounterspeechEvidence),
    ] {
        if active {
            let item = if let Some(source) = source {
                lexical_evidence(
                    rule_id,
                    category,
                    0,
                    Some(language.to_owned()),
                    text,
                    source,
                )
            } else {
                rule_span_evidence(
                    rule_id,
                    category,
                    0,
                    Some(language.to_owned()),
                    text,
                    start,
                    end,
                )
            };
            evidence.push(item);
        }
    }
}

fn token_range_for_span(tokens: &[ContextToken], start: usize, end: usize) -> Option<Range<usize>> {
    let first = tokens.partition_point(|token| token.span.end <= start);
    let last = tokens.partition_point(|token| token.span.start < end);
    (first < last).then_some(first..last)
}

fn nearby_phrase(
    tokens: &[ContextToken],
    event: Range<usize>,
    distance: usize,
    phrases: &[Vec<String>],
) -> Option<Range<usize>> {
    let clause = tokens[event.start].clause;
    nearby_candidate_starts(tokens.len(), event.clone(), distance, phrases).find_map(|start| {
        let length = phrase_len_at(tokens, start, phrases)?;
        let candidate = start..start + length;
        let candidate_distance = if candidate.end <= event.start {
            event.start - (candidate.end - 1)
        } else if candidate.start >= event.end {
            candidate.start - (event.end - 1)
        } else {
            0
        };
        (tokens[start].clause == clause && candidate_distance <= distance).then_some(candidate)
    })
}

fn nearby_phrases(
    tokens: &[ContextToken],
    event: Range<usize>,
    distance: usize,
    phrases: &[Vec<String>],
) -> Vec<Range<usize>> {
    let clause = tokens[event.start].clause;
    nearby_candidate_starts(tokens.len(), event.clone(), distance, phrases)
        .filter_map(|start| {
            let length = phrase_len_at(tokens, start, phrases)?;
            let candidate = start..start + length;
            let candidate_distance = if candidate.end <= event.start {
                event.start - (candidate.end - 1)
            } else if candidate.start >= event.end {
                candidate.start - (event.end - 1)
            } else {
                0
            };
            (tokens[start].clause == clause && candidate_distance <= distance).then_some(candidate)
        })
        .collect()
}

fn direct_identity_relation(
    text: &str,
    tokens: &[ContextToken],
    group: Range<usize>,
    event: Range<usize>,
    language: &str,
    identity_links: &[Vec<String>],
) -> bool {
    if tokens[group.start].clause != tokens[event.start].clause {
        return false;
    }

    if group.end <= event.start {
        let gap = group.end..event.start;
        let supported_gap = exact_phrase(tokens, gap.clone(), identity_links)
            || (matches!(language, "RU" | "AR") && gap.is_empty());
        return supported_gap && whitespace_separated(text, tokens, group.end - 1, event.start);
    }

    matches!(language, "RU" | "AR")
        && event.end == group.start
        && whitespace_separated(text, tokens, event.end - 1, group.start)
}

fn exact_phrase(tokens: &[ContextToken], range: Range<usize>, phrases: &[Vec<String>]) -> bool {
    let Some(candidate) = tokens.get(range) else {
        return false;
    };
    phrases.iter().any(|phrase| {
        !phrase.is_empty()
            && candidate.len() == phrase.len()
            && candidate
                .iter()
                .map(|token| token.text.as_str())
                .eq(phrase.iter().map(String::as_str))
    })
}

fn whitespace_separated(text: &str, tokens: &[ContextToken], first: usize, last: usize) -> bool {
    let Some(sequence) = tokens.get(first..=last) else {
        return false;
    };
    sequence.windows(2).all(|pair| {
        text.get(pair[0].span.end..pair[1].span.start)
            .is_some_and(|separator| separator.chars().all(char::is_whitespace))
    })
}

fn nearby_mention(tokens: &[ContextToken], event: Range<usize>, distance: usize) -> bool {
    let clause = tokens[event.start].clause;
    let start = event.start.saturating_sub(distance);
    let end = event.end.saturating_add(distance).min(tokens.len());
    tokens[start..end]
        .iter()
        .enumerate()
        .any(|(candidate_index, candidate)| {
            let candidate_index = start + candidate_index;
            let candidate_distance = if candidate_index < event.start {
                event.start - candidate_index
            } else if candidate_index >= event.end {
                candidate_index - (event.end - 1)
            } else {
                0
            };
            candidate.clause == clause && candidate_distance <= distance && candidate.mention
        })
}

fn preceding_phrase(
    tokens: &[ContextToken],
    event_start: usize,
    distance: usize,
    phrases: &[Vec<String>],
) -> bool {
    let clause = tokens[event_start].clause;
    preceding_candidate_starts(event_start, distance, phrases).any(|start| {
        phrase_len_at(tokens, start, phrases).is_some_and(|length| {
            let end = start + length;
            end <= event_start
                && tokens[start].clause == clause
                && event_start - (end - 1) <= distance
        })
    })
}

fn preceding_phrase_in_scope(
    tokens: &[ContextToken],
    event_start: usize,
    distance: usize,
    phrases: &[Vec<String>],
) -> bool {
    let scope = tokens[event_start].scope;
    preceding_candidate_starts(event_start, distance, phrases).any(|start| {
        phrase_len_at(tokens, start, phrases).is_some_and(|length| {
            let end = start + length;
            end <= event_start
                && tokens[start].scope == scope
                && event_start - (end - 1) <= distance
        })
    })
}

fn immediately_preceding_phrase(
    tokens: &[ContextToken],
    event_start: usize,
    phrases: &[Vec<String>],
) -> bool {
    let clause = tokens[event_start].clause;
    let maximum_length = maximum_phrase_length(phrases);
    (event_start.saturating_sub(maximum_length)..event_start).any(|start| {
        phrase_len_at(tokens, start, phrases)
            .is_some_and(|length| start + length == event_start && tokens[start].clause == clause)
    })
}

fn nearby_candidate_starts(
    token_count: usize,
    event: Range<usize>,
    distance: usize,
    phrases: &[Vec<String>],
) -> Range<usize> {
    let maximum_length = maximum_phrase_length(phrases);
    let start = event
        .start
        .saturating_sub(distance.saturating_add(maximum_length.saturating_sub(1)));
    let end = event.end.saturating_add(distance).min(token_count);
    start..end
}

fn overlapping_candidate_starts(
    token_count: usize,
    event: Range<usize>,
    phrases: &[Vec<String>],
) -> Range<usize> {
    let maximum_length = maximum_phrase_length(phrases);
    let start = event.start.saturating_sub(maximum_length.saturating_sub(1));
    start..event.end.min(token_count)
}

fn preceding_candidate_starts(
    event_start: usize,
    distance: usize,
    phrases: &[Vec<String>],
) -> Range<usize> {
    let maximum_length = maximum_phrase_length(phrases);
    let start =
        event_start.saturating_sub(distance.saturating_add(maximum_length.saturating_sub(1)));
    start..event_start
}

fn maximum_phrase_length(phrases: &[Vec<String>]) -> usize {
    phrases.iter().map(Vec::len).max().unwrap_or(0)
}

fn phrase_len_at(tokens: &[ContextToken], start: usize, phrases: &[Vec<String>]) -> Option<usize> {
    phrases.iter().find_map(|phrase| {
        let end = start.checked_add(phrase.len())?;
        let candidate = tokens.get(start..end)?;
        (!phrase.is_empty()
            && candidate
                .iter()
                .all(|token| token.clause == tokens[start].clause)
            && candidate
                .iter()
                .map(|token| token.text.as_str())
                .eq(phrase.iter().map(String::as_str)))
        .then_some(phrase.len())
    })
}

fn lexical_evidence(
    rule_id: RuleId,
    category: PolicyCategory,
    points: u8,
    language: Option<String>,
    text: &str,
    source: &crate::LexiconMatch,
) -> RuleEvidence {
    RuleEvidence {
        rule_id,
        category,
        points,
        language,
        matched_text: text
            .get(source.raw_start..source.raw_end)
            .unwrap_or_default()
            .to_owned(),
        candidate_view: Some(source.view),
        normalized_start: Some(source.normalized_start),
        normalized_end: Some(source.normalized_end),
        raw_start: Some(source.raw_start),
        raw_end: Some(source.raw_end),
    }
}

fn rule_span_evidence(
    rule_id: RuleId,
    category: PolicyCategory,
    points: u8,
    language: Option<String>,
    text: &str,
    start: usize,
    end: usize,
) -> RuleEvidence {
    RuleEvidence {
        rule_id,
        category,
        points,
        language,
        matched_text: text.get(start..end).unwrap_or_default().to_owned(),
        candidate_view: None,
        normalized_start: None,
        normalized_end: None,
        raw_start: Some(start),
        raw_end: Some(end),
    }
}

fn global_evidence(
    rule_id: RuleId,
    category: PolicyCategory,
    points: u8,
    language: &str,
    text: &str,
) -> RuleEvidence {
    RuleEvidence {
        rule_id,
        category,
        points,
        language: Some(language.to_owned()),
        matched_text: text.to_owned(),
        candidate_view: None,
        normalized_start: None,
        normalized_end: None,
        raw_start: None,
        raw_end: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{nearby_phrase, phrase_len_at};
    use crate::{TextDocument, rule_pack::for_language};

    #[test]
    fn normalizes_a_rule_stop_word() {
        let document = TextDocument::new("te");
        let pack = for_language("FR").expect("French pack");

        assert_eq!(
            phrase_len_at(document.context_tokens(), 0, &pack.targets),
            Some(1)
        );
    }

    #[test]
    fn matches_french_threat_and_target_terms() {
        let document = TextDocument::new("Je vais te tuer");
        let pack = for_language("FR").expect("French pack");
        let tokens = document.context_tokens();

        assert_eq!(phrase_len_at(tokens, 3, &pack.threats), Some(1));
        assert_eq!(phrase_len_at(tokens, 2, &pack.targets), Some(1));
        assert_eq!(
            tokens.iter().map(|token| token.clause).collect::<Vec<_>>(),
            [0, 0, 0, 0]
        );
        assert!(nearby_phrase(tokens, 3..4, 5, &pack.targets).is_some());
    }
}
