use std::ops::Range;

use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::{PolicyCategory, ReplyTarget, RuleId};

use super::{
    DIRECT_THREAT_SCORE, DIRECTED_INSULT_SCORE, HARM_WISH_SCORE, LanguageRules,
    NEGATIVE_SENTIMENT_SCORE, PhraseSet, RuleEvent, SELF_HARM_COMMAND_SCORE, has_reply_target,
};

const MAX_TARGET_GAP_CODEPOINTS: usize = 8;
const MAX_EVENT_COMPONENT_GAP_CODEPOINTS: usize = 8;

#[derive(Debug, Clone, Copy)]
struct CompactSpan {
    raw_start: usize,
    raw_end: usize,
}

#[derive(Debug)]
struct CompactClause {
    codepoints: Vec<char>,
    spans: Vec<CompactSpan>,
    scopes: Vec<usize>,
    ends_with_question: bool,
}

#[derive(Debug, Clone, Copy)]
struct PhraseMatch {
    start: usize,
    end: usize,
}

impl CompactClause {
    fn raw_range(&self, start: usize, end: usize) -> Range<usize> {
        self.spans[start].raw_start..self.spans[end - 1].raw_end
    }

    fn slice(&self, range: Range<usize>) -> Self {
        let ends_with_question = self.ends_with_question && range.end == self.codepoints.len();
        Self {
            codepoints: self.codepoints[range.clone()].to_vec(),
            spans: self.spans[range.clone()].to_vec(),
            scopes: self.scopes[range].to_vec(),
            ends_with_question,
        }
    }
}

pub(super) fn analyze(
    rules: &LanguageRules,
    text: &str,
    reply_target: ReplyTarget,
) -> Vec<RuleEvent> {
    let mut events = Vec::new();
    for clause in compact_clauses(text) {
        let boundaries = find_phrases(&clause, rules.proposition_boundaries);
        for range in proposition_ranges(clause.codepoints.len(), &boundaries) {
            events.extend(analyze_clause(rules, &clause.slice(range), reply_target));
        }
    }
    events
}

fn analyze_clause(
    rules: &LanguageRules,
    clause: &CompactClause,
    reply_target: ReplyTarget,
) -> Vec<RuleEvent> {
    let targets = find_phrases(clause, rules.targets);
    let harms = find_phrases(clause, rules.harm_predicates);
    let intents = find_phrases(clause, rules.intent_markers);
    let wishes = find_phrases(clause, rules.wish_markers);
    let outcomes = find_phrases(clause, rules.harm_outcomes);
    let self_harm = find_phrases(clause, rules.self_harm_commands);
    let insults = find_phrases(clause, rules.strong_insults);
    let sentiments = find_phrases(clause, rules.negative_sentiment);
    let copulas = find_phrases(clause, rules.copulas_or_vocatives);
    let negators = find_phrases(clause, rules.negators);
    let reports = find_phrases(clause, rules.reports);
    let counterspeech = find_phrases(clause, rules.counterspeech_markers);
    let mut events = Vec::new();

    if let Some(implicit) = exact_phrase(clause, rules.implicit_target_threats) {
        events.push(exact_event(
            clause,
            implicit,
            implicit,
            DIRECT_THREAT_SCORE,
            RuleId::DirectThreat,
            PolicyCategory::ThreatLanguage,
            &reports,
            &counterspeech,
        ));
    }
    if let Some(implicit) = exact_phrase(clause, rules.implicit_target_harm_wishes) {
        events.push(exact_event(
            clause,
            implicit,
            implicit,
            HARM_WISH_SCORE,
            RuleId::HostileWish,
            PolicyCategory::ThreatLanguage,
            &reports,
            &counterspeech,
        ));
    }
    if let Some(implicit) = exact_phrase(clause, rules.implicit_target_directed_insults) {
        events.push(exact_event(
            clause,
            implicit,
            implicit,
            DIRECTED_INSULT_SCORE,
            RuleId::SemanticDirectedHostility,
            PolicyCategory::TargetedAbuse,
            &reports,
            &counterspeech,
        ));
    }

    for harm in harms.iter().copied() {
        let intent = intents
            .iter()
            .rev()
            .find(|intent| {
                intent.end <= harm.start
                    && harm.start - intent.end <= MAX_EVENT_COMPONENT_GAP_CODEPOINTS
            })
            .copied();
        let target = target_for_anchor(clause, &targets, harm, intent.map_or(0, |m| m.start), true);
        let has_target = target.is_some() || has_reply_target(reply_target);
        let imperative = harm.start == 0
            && target.is_some_and(|target| target.start >= harm.end)
            && !clause.ends_with_question;
        if has_target && (intent.is_some() || imperative) {
            let frame = codepoint_frame(&[Some(harm), intent, target]);
            events.push(event(
                clause,
                frame,
                harm,
                DIRECT_THREAT_SCORE,
                RuleId::DirectThreat,
                PolicyCategory::ThreatLanguage,
                &negators,
                &reports,
                &counterspeech,
            ));
        }
    }

    for wish in wishes.iter().copied() {
        let Some(outcome) = outcomes
            .iter()
            .find(|outcome| {
                outcome.start >= wish.end
                    && outcome.start - wish.end <= MAX_EVENT_COMPONENT_GAP_CODEPOINTS
                    && outcome.end == clause.codepoints.len()
            })
            .copied()
        else {
            continue;
        };
        let target = target_for_anchor(clause, &targets, outcome, wish.start, false);
        if target.is_some() || has_reply_target(reply_target) {
            let frame = codepoint_frame(&[Some(wish), Some(outcome), target]);
            events.push(event(
                clause,
                frame,
                outcome,
                HARM_WISH_SCORE,
                RuleId::HostileWish,
                PolicyCategory::ThreatLanguage,
                &negators,
                &reports,
                &counterspeech,
            ));
        }
    }

    for insult in insults.iter().copied() {
        let target = target_for_anchor(clause, &targets, insult, 0, false);
        let explicit_frame = target.is_some_and(|target| {
            copulas
                .iter()
                .any(|copula| copula.start >= target.end && copula.end <= insult.start)
                || target.end == insult.start
        });
        if explicit_frame || has_reply_target(reply_target) {
            let frame = codepoint_frame(&[Some(insult), target]);
            events.push(event(
                clause,
                frame,
                insult,
                DIRECTED_INSULT_SCORE,
                RuleId::SemanticDirectedHostility,
                PolicyCategory::TargetedAbuse,
                &negators,
                &reports,
                &counterspeech,
            ));
        }
    }

    for command in self_harm.iter().copied() {
        if !match_ends_scope(clause, command) {
            continue;
        }
        events.push(event(
            clause,
            command,
            command,
            SELF_HARM_COMMAND_SCORE,
            RuleId::SelfHarmCommand,
            PolicyCategory::ThreatLanguage,
            &negators,
            &reports,
            &counterspeech,
        ));
    }

    for sentiment in sentiments.iter().copied() {
        let target = target_for_anchor(clause, &targets, sentiment, 0, false);
        if target.is_none() && !has_reply_target(reply_target) {
            continue;
        }
        let frame = codepoint_frame(&[Some(sentiment), target]);
        let sentiment_event = event(
            clause,
            frame,
            sentiment,
            NEGATIVE_SENTIMENT_SCORE,
            RuleId::NegativeSentiment,
            PolicyCategory::SentimentSupport,
            &negators,
            &reports,
            &counterspeech,
        );
        if !events
            .iter()
            .any(|complete| ranges_overlap(&complete.event_range, &sentiment_event.event_range))
        {
            events.push(sentiment_event);
        }
    }
    events
}

#[allow(clippy::too_many_arguments)]
fn event(
    clause: &CompactClause,
    frame: PhraseMatch,
    primary: PhraseMatch,
    score: u8,
    rule_id: RuleId,
    category: PolicyCategory,
    negators: &[PhraseMatch],
    reports: &[PhraseMatch],
    counterspeech: &[PhraseMatch],
) -> RuleEvent {
    RuleEvent {
        score,
        rule_id,
        category,
        event_range: clause.raw_range(frame.start, frame.end),
        suppression: suppression(clause, frame, primary, negators, reports, counterspeech),
    }
}

#[allow(clippy::too_many_arguments)]
fn exact_event(
    clause: &CompactClause,
    frame: PhraseMatch,
    primary: PhraseMatch,
    score: u8,
    rule_id: RuleId,
    category: PolicyCategory,
    reports: &[PhraseMatch],
    counterspeech: &[PhraseMatch],
) -> RuleEvent {
    event(
        clause,
        frame,
        primary,
        score,
        rule_id,
        category,
        &[],
        reports,
        counterspeech,
    )
}

fn suppression(
    clause: &CompactClause,
    frame: PhraseMatch,
    primary: PhraseMatch,
    negators: &[PhraseMatch],
    reports: &[PhraseMatch],
    counterspeech: &[PhraseMatch],
) -> Option<(RuleId, Range<usize>)> {
    if let Some(negator) = negators.iter().find(|negator| {
        clause.scopes[negator.start] == clause.scopes[primary.start]
            && ((frame.start <= negator.start && negator.end <= frame.end)
                || (negator.end <= frame.start
                    && frame.start - negator.end <= MAX_EVENT_COMPONENT_GAP_CODEPOINTS))
    }) {
        return Some((
            RuleId::NegatedEvidence,
            clause.raw_range(negator.start, negator.end),
        ));
    }
    if let Some(marker) = counterspeech
        .iter()
        .find(|marker| marker.end == frame.start)
    {
        return Some((
            RuleId::CounterspeechEvidence,
            clause.raw_range(marker.start, marker.end),
        ));
    }
    reports
        .iter()
        .find(|marker| marker.end == frame.start || marker.start == frame.end)
        .map(|marker| {
            (
                RuleId::ReportedEvidence,
                clause.raw_range(marker.start, marker.end),
            )
        })
}

fn find_phrases(clause: &CompactClause, phrases: PhraseSet) -> Vec<PhraseMatch> {
    let mut matches = Vec::new();
    for phrase in phrases.phrases() {
        let phrase = compact_phrase(phrase);
        if phrase.is_empty() || phrase.len() > clause.codepoints.len() {
            continue;
        }
        for start in 0..=clause.codepoints.len() - phrase.len() {
            if clause.codepoints[start..start + phrase.len()] == phrase {
                matches.push(PhraseMatch {
                    start,
                    end: start + phrase.len(),
                });
            }
        }
    }
    matches.sort_by_key(|item| (item.start, item.end));
    matches
}

fn exact_phrase(clause: &CompactClause, phrases: PhraseSet) -> Option<PhraseMatch> {
    phrases.phrases().iter().find_map(|phrase| {
        let phrase = compact_phrase(phrase);
        (phrase == clause.codepoints).then_some(PhraseMatch {
            start: 0,
            end: clause.codepoints.len(),
        })
    })
}

fn compact_phrase(phrase: &str) -> Vec<char> {
    phrase
        .nfkc()
        .flat_map(char::to_lowercase)
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn codepoint_frame(phrases: &[Option<PhraseMatch>]) -> PhraseMatch {
    PhraseMatch {
        start: phrases
            .iter()
            .flatten()
            .map(|phrase| phrase.start)
            .min()
            .expect("an event frame has one phrase"),
        end: phrases
            .iter()
            .flatten()
            .map(|phrase| phrase.end)
            .max()
            .expect("an event frame has one phrase"),
    }
}

fn target_for_anchor(
    clause: &CompactClause,
    targets: &[PhraseMatch],
    anchor: PhraseMatch,
    lower_bound: usize,
    forward_target_must_end_scope: bool,
) -> Option<PhraseMatch> {
    targets
        .iter()
        .find(|target| {
            target.start >= anchor.end
                && target.start - anchor.end <= MAX_TARGET_GAP_CODEPOINTS
                && if forward_target_must_end_scope {
                    match_ends_scope(clause, **target)
                } else {
                    target.end - target.start > 1 || target.end == clause.codepoints.len()
                }
        })
        .copied()
        .or_else(|| {
            targets
                .iter()
                .rev()
                .find(|target| {
                    target.start >= lower_bound
                        && target.end <= anchor.start
                        && anchor.start - target.end <= MAX_TARGET_GAP_CODEPOINTS
                })
                .copied()
        })
}

fn match_ends_scope(clause: &CompactClause, item: PhraseMatch) -> bool {
    item.end == clause.codepoints.len() || clause.scopes[item.end - 1] != clause.scopes[item.end]
}

fn proposition_ranges(length: usize, configured: &[PhraseMatch]) -> Vec<Range<usize>> {
    let mut boundaries = configured.to_vec();
    boundaries.sort_by_key(|boundary| (boundary.start, boundary.end));
    let mut ranges = Vec::new();
    let mut start = 0;
    for boundary in boundaries {
        if boundary.start > start {
            ranges.push(start..boundary.start);
        }
        start = start.max(boundary.end);
    }
    if start < length {
        ranges.push(start..length);
    }
    ranges
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn compact_clauses(text: &str) -> Vec<CompactClause> {
    compact_clauses_with_observer(text, || {})
}

fn compact_clauses_with_observer(
    text: &str,
    mut observe_character: impl FnMut(),
) -> Vec<CompactClause> {
    let mut clauses = Vec::new();
    let mut clause_start = 0;
    let mut characters = text.char_indices().peekable();
    let mut pending_boundary = None;
    while let Some((start, character)) = characters.next() {
        observe_character();
        let end = characters.peek().map_or(text.len(), |(end, _)| *end);
        if is_clause_boundary(character) {
            let boundary = pending_boundary.get_or_insert((start, end, false));
            boundary.1 = end;
            boundary.2 |= is_question_boundary(character);
            continue;
        }
        if character.is_whitespace() && pending_boundary.is_some() {
            continue;
        }
        if let Some((boundary_start, _, ends_with_question)) = pending_boundary.take() {
            if let Some(mut clause) = normalized_compact_clause(text, clause_start..boundary_start)
            {
                clause.ends_with_question = ends_with_question;
                clauses.push(clause);
            }
            clause_start = start;
        }
    }
    if let Some((boundary_start, boundary_end, ends_with_question)) = pending_boundary {
        if let Some(mut clause) = normalized_compact_clause(text, clause_start..boundary_start) {
            clause.ends_with_question = ends_with_question;
            clauses.push(clause);
        }
        clause_start = boundary_end;
    }
    if let Some(clause) = normalized_compact_clause(text, clause_start..text.len()) {
        clauses.push(clause);
    }
    clauses
}

fn normalized_compact_clause(text: &str, raw_range: Range<usize>) -> Option<CompactClause> {
    normalized_compact_clause_with_observer(text, raw_range, || {})
}

fn normalized_compact_clause_with_observer(
    text: &str,
    raw_range: Range<usize>,
    mut observe_grapheme: impl FnMut(),
) -> Option<CompactClause> {
    let source = &text[raw_range.clone()];
    let mut codepoints = Vec::new();
    let mut spans = Vec::new();
    let mut scopes = Vec::new();
    let mut scope = 0;
    for (local_start, grapheme) in source.grapheme_indices(true) {
        observe_grapheme();
        let local_end = local_start + grapheme.len();
        for codepoint in grapheme.nfkc().flat_map(char::to_lowercase) {
            if is_compact_separator(codepoint) {
                scope += 1;
                continue;
            }
            if codepoint.is_whitespace() || is_quote(codepoint) {
                continue;
            }
            codepoints.push(codepoint);
            scopes.push(scope);
            spans.push(CompactSpan {
                raw_start: raw_range.start + local_start,
                raw_end: raw_range.start + local_end,
            });
        }
    }
    (!codepoints.is_empty()).then_some(CompactClause {
        codepoints,
        spans,
        scopes,
        ends_with_question: false,
    })
}

fn is_clause_boundary(character: char) -> bool {
    matches!(
        character,
        '.' | '!' | '?' | ';' | '؟' | '۔' | '।' | '॥' | '。' | '！' | '？' | '；' | '\n' | '\r'
    )
}

fn is_question_boundary(character: char) -> bool {
    matches!(character, '?' | '؟' | '？')
}

fn is_quote(character: char) -> bool {
    matches!(
        character,
        '"' | '„' | '“' | '”' | '«' | '»' | '「' | '」' | '『' | '』'
    )
}

fn is_compact_separator(character: char) -> bool {
    matches!(character, ',' | '，')
}

#[cfg(test)]
mod tests {
    use super::{compact_clauses_with_observer, normalized_compact_clause_with_observer};

    #[test]
    fn compact_normalization_visits_each_long_decomposed_grapheme_once() {
        let source = "か\u{3099}".repeat(10_000);
        let mut visits = 0;

        let clause = normalized_compact_clause_with_observer(&source, 0..source.len(), || {
            visits += 1;
        })
        .expect("compact clause");

        assert_eq!(visits, 10_000);
        assert_eq!(clause.codepoints.len(), 10_000);
        assert_eq!(clause.spans[9_999].raw_end, source.len());
    }

    #[test]
    fn compact_clause_parsing_visits_each_spaced_terminal_once() {
        let source = format!("Kill you{}?", "! ".repeat(10_000));
        let mut visits = 0;

        let clauses = compact_clauses_with_observer(&source, || visits += 1);

        assert_eq!(visits, source.chars().count());
        assert_eq!(clauses.len(), 1);
        assert!(clauses[0].ends_with_question);
    }
}
