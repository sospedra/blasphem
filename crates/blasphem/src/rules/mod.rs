//! Immutable multilingual semantic rules.

mod channel;
mod compact;
mod identity;
pub mod packs;
mod word;

use std::ops::Range;

use crate::{Language, PolicyCategory, ReplyTarget, RuleEvidence, RuleId};

pub use identity::canonical_rule_identity_for;
pub use packs::{arabic_hindi_rules, cjk_rules, word_rules};

pub const RULE_NUDGE_THRESHOLD: u8 = 50;
pub const DIRECT_THREAT_SCORE: u8 = 95;
pub const HARM_WISH_SCORE: u8 = 85;
pub const SELF_HARM_COMMAND_SCORE: u8 = 95;
pub const DIRECTED_INSULT_SCORE: u8 = 70;
pub const LEXICON_SCORE: u8 = 30;
pub const NEGATIVE_SENTIMENT_SCORE: u8 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhraseSet(&'static [&'static str]);

impl PhraseSet {
    #[must_use]
    pub const fn new(phrases: &'static [&'static str]) -> Self {
        Self(phrases)
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self(&[])
    }

    pub(crate) const fn phrases(self) -> &'static [&'static str] {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RuleMatchProfile {
    WordClauses = 0,
    CompactClauses = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageRules {
    pub language: Language,
    pub version: u16,
    pub targets: PhraseSet,
    pub harm_predicates: PhraseSet,
    pub intent_markers: PhraseSet,
    pub implicit_target_threats: PhraseSet,
    pub wish_markers: PhraseSet,
    pub harm_outcomes: PhraseSet,
    pub implicit_target_harm_wishes: PhraseSet,
    pub self_harm_commands: PhraseSet,
    pub strong_insults: PhraseSet,
    pub implicit_target_directed_insults: PhraseSet,
    pub negative_sentiment: PhraseSet,
    pub copulas_or_vocatives: PhraseSet,
    pub negators: PhraseSet,
    pub reports: PhraseSet,
    pub counterspeech_markers: PhraseSet,
    pub proposition_boundaries: PhraseSet,
    pub matching: RuleMatchProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleOutcome {
    pub score: u8,
    pub should_nudge: bool,
    pub evidence: Vec<RuleEvidence>,
}

impl RuleOutcome {
    /// Returns whether contextual evidence suppresses the sparse channel decision.
    #[must_use]
    pub fn suppresses_sparse_channel(&self) -> bool {
        self.evidence.iter().any(|item| {
            item.points == 0
                && matches!(
                    item.rule_id,
                    RuleId::NegatedEvidence
                        | RuleId::QuotedEvidence
                        | RuleId::ReportedEvidence
                        | RuleId::CounterspeechEvidence
                )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuleEvent {
    pub score: u8,
    pub rule_id: RuleId,
    pub category: PolicyCategory,
    pub event_range: Range<usize>,
    pub suppression: Option<(RuleId, Range<usize>)>,
}

#[must_use]
pub fn analyze_with_rules(
    rules: &LanguageRules,
    text: &str,
    reply_target: ReplyTarget,
) -> RuleOutcome {
    let events = match rules.matching {
        RuleMatchProfile::WordClauses => word::analyze(rules, text, reply_target),
        RuleMatchProfile::CompactClauses => compact::analyze(rules, text, reply_target),
    };
    outcome(rules.language, text, events)
}

fn outcome(language: Language, text: &str, events: Vec<RuleEvent>) -> RuleOutcome {
    let quote_ranges = balanced_quote_ranges(text);
    let events = events
        .into_iter()
        .map(|mut event| {
            if quote_ranges.iter().any(|quote| {
                quote.start <= event.event_range.start && event.event_range.end <= quote.end
            }) {
                event.suppression = Some((RuleId::QuotedEvidence, event.event_range.clone()));
            }
            event
        })
        .collect::<Vec<_>>();
    let score = events
        .iter()
        .filter(|event| event.suppression.is_none())
        .map(|event| event.score)
        .max()
        .unwrap_or(0);
    let evidence = events
        .into_iter()
        .map(|event| {
            let (rule_id, points, range) = event.suppression.map_or_else(
                || (event.rule_id, event.score, event.event_range),
                |(rule_id, range)| (rule_id, 0, range),
            );
            RuleEvidence {
                rule_id,
                category: event.category,
                points,
                language: Some(language.code().to_owned()),
                matched_text: text.get(range.clone()).unwrap_or_default().to_owned(),
                candidate_view: None,
                normalized_start: None,
                normalized_end: None,
                raw_start: Some(range.start),
                raw_end: Some(range.end),
            }
        })
        .collect();

    RuleOutcome {
        score,
        should_nudge: score >= RULE_NUDGE_THRESHOLD,
        evidence,
    }
}

fn balanced_quote_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut straight = None;
    let mut curly = None;
    let mut low_double = None;
    let mut guillemet = None;
    let mut corner = None;
    let mut white_corner = None;
    for (index, character) in text.char_indices() {
        match character {
            '"' => toggle_quote(&mut ranges, &mut straight, index, character.len_utf8()),
            '„' => low_double = Some(index),
            '“' => {
                if let Some(start) = low_double.take() {
                    ranges.push(start..index + character.len_utf8());
                } else {
                    curly = Some(index);
                }
            }
            '”' => close_quote(&mut ranges, &mut curly, index, character.len_utf8()),
            '«' => pair_quote(
                &mut ranges,
                &mut guillemet,
                index,
                character.len_utf8(),
                character,
                '»',
            ),
            '»' => pair_quote(
                &mut ranges,
                &mut guillemet,
                index,
                character.len_utf8(),
                character,
                '«',
            ),
            '「' => corner = Some(index),
            '」' => close_quote(&mut ranges, &mut corner, index, character.len_utf8()),
            '『' => white_corner = Some(index),
            '』' => close_quote(&mut ranges, &mut white_corner, index, character.len_utf8()),
            _ => {}
        }
    }
    ranges
}

fn pair_quote(
    ranges: &mut Vec<Range<usize>>,
    state: &mut Option<(usize, char)>,
    index: usize,
    width: usize,
    character: char,
    expected_close: char,
) {
    match *state {
        Some((start, expected)) if expected == character => {
            ranges.push(start..index + width);
            *state = None;
        }
        Some(_) => {}
        None => *state = Some((index, expected_close)),
    }
}

fn toggle_quote(
    ranges: &mut Vec<Range<usize>>,
    start: &mut Option<usize>,
    index: usize,
    width: usize,
) {
    if let Some(open) = start.take() {
        ranges.push(open..index + width);
    } else {
        *start = Some(index);
    }
}

fn close_quote(
    ranges: &mut Vec<Range<usize>>,
    start: &mut Option<usize>,
    index: usize,
    width: usize,
) {
    if let Some(open) = start.take() {
        ranges.push(open..index + width);
    }
}

pub(super) const fn has_reply_target(reply_target: ReplyTarget) -> bool {
    matches!(
        reply_target,
        ReplyTarget::Person | ReplyTarget::ProtectedGroup
    )
}
pub use channel::{RuleChannel, RuleChannelError, canonical_rule_identity};
