use std::ops::Range;

use crate::text::ContextToken;
use crate::{PolicyCategory, ReplyTarget, RuleId, TextDocument};

use super::{
    DIRECT_THREAT_SCORE, DIRECTED_INSULT_SCORE, HARM_WISH_SCORE, LanguageRules,
    NEGATIVE_SENTIMENT_SCORE, PhraseSet, RuleEvent, SELF_HARM_COMMAND_SCORE, has_reply_target,
};

const MAX_TARGET_GAP_TOKENS: usize = 2;
const MAX_EVENT_COMPONENT_GAP_CODEPOINTS: usize = 8;

#[derive(Debug, Clone, Copy)]
struct PhraseMatch {
    start: usize,
    end: usize,
}

pub(super) fn analyze(
    rules: &LanguageRules,
    text: &str,
    reply_target: ReplyTarget,
) -> Vec<RuleEvent> {
    let document = TextDocument::for_rule_language(text, rules.language);
    let tokens = document.context_tokens();
    if tokens.is_empty() {
        return Vec::new();
    }
    let codepoint_prefix = codepoint_prefix(tokens);

    let targets = find_phrases(tokens, rules.targets);
    let harms = find_phrases(tokens, rules.harm_predicates);
    let mut intents = find_phrases(tokens, rules.intent_markers);
    intents.sort_by_key(|item| (item.end, item.start));
    let wishes = find_phrases(tokens, rules.wish_markers);
    let outcomes = find_phrases(tokens, rules.harm_outcomes);
    let self_harm = find_phrases(tokens, rules.self_harm_commands);
    let insults = find_phrases(tokens, rules.strong_insults);
    let sentiments = find_phrases(tokens, rules.negative_sentiment);
    let copulas = find_phrases(tokens, rules.copulas_or_vocatives);
    let negators = find_phrases(tokens, rules.negators);
    let reports = find_phrases(tokens, rules.reports);
    let counterspeech = find_phrases(tokens, rules.counterspeech_markers);
    let boundaries = find_phrases(tokens, rules.proposition_boundaries);

    let mut events = Vec::new();
    for clause in proposition_ranges(tokens, &boundaries) {
        let proposition_negators = matches_in(&negators, &clause).collect::<Vec<_>>();
        if let Some(implicit) = exact_phrase(tokens, &clause, rules.implicit_target_threats) {
            events.push(exact_event(
                tokens,
                &codepoint_prefix,
                token_range(tokens, implicit),
                implicit,
                DIRECT_THREAT_SCORE,
                RuleId::DirectThreat,
                PolicyCategory::ThreatLanguage,
                &reports,
                &counterspeech,
            ));
        }
        if let Some(implicit) = exact_phrase(tokens, &clause, rules.implicit_target_harm_wishes) {
            events.push(exact_event(
                tokens,
                &codepoint_prefix,
                token_range(tokens, implicit),
                implicit,
                HARM_WISH_SCORE,
                RuleId::HostileWish,
                PolicyCategory::ThreatLanguage,
                &reports,
                &counterspeech,
            ));
        }
        if let Some(implicit) =
            exact_phrase(tokens, &clause, rules.implicit_target_directed_insults)
        {
            events.push(exact_event(
                tokens,
                &codepoint_prefix,
                token_range(tokens, implicit),
                implicit,
                DIRECTED_INSULT_SCORE,
                RuleId::SemanticDirectedHostility,
                PolicyCategory::TargetedAbuse,
                &reports,
                &counterspeech,
            ));
        }

        for harm in matches_in(&harms, &clause) {
            let intent = nearest_prior_intent(&intents, &clause, harm.start, &codepoint_prefix);
            let target = target_for_anchor(
                &targets,
                &clause,
                harm,
                intent.map_or(clause.start, |m| m.start),
            );
            let has_target = target.is_some() || has_reply_target(reply_target);
            let imperative = harm.start == clause.start
                && target.is_some_and(|target| target.start >= harm.end)
                && !proposition_ends_with_question(text, tokens, &clause);
            if has_target && (intent.is_some() || imperative) {
                let event_range = frame_range(tokens, &[Some(harm), intent, target]);
                events.push(event(
                    tokens,
                    &codepoint_prefix,
                    event_range,
                    harm,
                    DIRECT_THREAT_SCORE,
                    RuleId::DirectThreat,
                    PolicyCategory::ThreatLanguage,
                    &proposition_negators,
                    &reports,
                    &counterspeech,
                ));
            }
        }

        for wish in matches_in(&wishes, &clause) {
            let Some(outcome) = outcomes
                .iter()
                .find(|outcome| {
                    in_range(outcome, &clause)
                        && outcome.start >= wish.end
                        && codepoint_gap(&codepoint_prefix, wish.end, outcome.start)
                            <= MAX_EVENT_COMPONENT_GAP_CODEPOINTS
                })
                .copied()
            else {
                continue;
            };
            let target = target_for_anchor(&targets, &clause, outcome, wish.start);
            if target.is_some() || has_reply_target(reply_target) {
                let event_range = frame_range(tokens, &[Some(wish), Some(outcome), target]);
                events.push(event(
                    tokens,
                    &codepoint_prefix,
                    event_range,
                    outcome,
                    HARM_WISH_SCORE,
                    RuleId::HostileWish,
                    PolicyCategory::ThreatLanguage,
                    &proposition_negators,
                    &reports,
                    &counterspeech,
                ));
            }
        }

        for insult in matches_in(&insults, &clause) {
            let target = target_for_anchor(&targets, &clause, insult, clause.start);
            let explicit_frame = target.is_some_and(|target| {
                copulas.iter().any(|copula| {
                    in_range(copula, &clause)
                        && copula.start >= target.end
                        && copula.end <= insult.start
                }) || target.end == insult.start
            });
            if explicit_frame || has_reply_target(reply_target) {
                let event_range = frame_range(tokens, &[Some(insult), target]);
                events.push(event(
                    tokens,
                    &codepoint_prefix,
                    event_range,
                    insult,
                    DIRECTED_INSULT_SCORE,
                    RuleId::SemanticDirectedHostility,
                    PolicyCategory::TargetedAbuse,
                    &proposition_negators,
                    &reports,
                    &counterspeech,
                ));
            }
        }

        for command in matches_in(&self_harm, &clause) {
            events.push(event(
                tokens,
                &codepoint_prefix,
                token_range(tokens, command),
                command,
                SELF_HARM_COMMAND_SCORE,
                RuleId::SelfHarmCommand,
                PolicyCategory::ThreatLanguage,
                &proposition_negators,
                &reports,
                &counterspeech,
            ));
        }

        for sentiment in matches_in(&sentiments, &clause) {
            let target = target_for_anchor(&targets, &clause, sentiment, clause.start);
            if target.is_none() && !has_reply_target(reply_target) {
                continue;
            }
            let event_range = frame_range(tokens, &[Some(sentiment), target]);
            let sentiment_event = event(
                tokens,
                &codepoint_prefix,
                event_range,
                sentiment,
                NEGATIVE_SENTIMENT_SCORE,
                RuleId::NegativeSentiment,
                PolicyCategory::SentimentSupport,
                &proposition_negators,
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
    }
    events
}

#[allow(clippy::too_many_arguments)]
fn event(
    tokens: &[ContextToken],
    codepoint_prefix: &[usize],
    event_range: Range<usize>,
    primary: PhraseMatch,
    score: u8,
    rule_id: RuleId,
    category: PolicyCategory,
    negators: &[PhraseMatch],
    reports: &[PhraseMatch],
    counterspeech: &[PhraseMatch],
) -> RuleEvent {
    let suppression = suppression(
        tokens,
        codepoint_prefix,
        &event_range,
        primary,
        negators,
        reports,
        counterspeech,
    );
    RuleEvent {
        score,
        rule_id,
        category,
        event_range,
        suppression,
    }
}

#[allow(clippy::too_many_arguments)]
fn exact_event(
    tokens: &[ContextToken],
    codepoint_prefix: &[usize],
    event_range: Range<usize>,
    primary: PhraseMatch,
    score: u8,
    rule_id: RuleId,
    category: PolicyCategory,
    reports: &[PhraseMatch],
    counterspeech: &[PhraseMatch],
) -> RuleEvent {
    event(
        tokens,
        codepoint_prefix,
        event_range,
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
    tokens: &[ContextToken],
    codepoint_prefix: &[usize],
    event_range: &Range<usize>,
    primary: PhraseMatch,
    negators: &[PhraseMatch],
    reports: &[PhraseMatch],
    counterspeech: &[PhraseMatch],
) -> Option<(RuleId, Range<usize>)> {
    let event_start = tokens
        .iter()
        .position(|token| token.span.start == event_range.start)
        .unwrap_or(primary.start);
    if let Some(negator) = negators.iter().find(|negator| {
        let range = token_range(tokens, **negator);
        tokens[negator.start].clause == tokens[primary.start].clause
            && tokens[negator.start].scope == tokens[primary.start].scope
            && ((event_range.start <= range.start && range.end <= event_range.end)
                || (negator.end <= event_start
                    && codepoint_gap(codepoint_prefix, negator.end, event_start)
                        <= MAX_EVENT_COMPONENT_GAP_CODEPOINTS))
    }) {
        return Some((RuleId::NegatedEvidence, token_range(tokens, *negator)));
    }

    if let Some(marker) = counterspeech.iter().find(|marker| {
        marker.end == event_start && tokens[marker.start].clause == tokens[primary.start].clause
    }) {
        return Some((RuleId::CounterspeechEvidence, token_range(tokens, *marker)));
    }
    reports
        .iter()
        .find(|marker| {
            marker.end == event_start && tokens[marker.start].clause == tokens[primary.start].clause
        })
        .map(|marker| (RuleId::ReportedEvidence, token_range(tokens, *marker)))
}

fn proposition_ends_with_question(
    text: &str,
    tokens: &[ContextToken],
    proposition: &Range<usize>,
) -> bool {
    let trailing_start = tokens[proposition.end - 1].span.end;
    let trailing_end = tokens
        .get(proposition.end)
        .map_or(text.len(), |token| token.span.start);
    text[trailing_start..trailing_end]
        .chars()
        .any(is_question_boundary)
}

fn is_question_boundary(character: char) -> bool {
    matches!(character, '?' | '؟' | '？')
}

fn codepoint_prefix(tokens: &[ContextToken]) -> Vec<usize> {
    codepoint_prefix_with_observer(tokens, || {})
}

fn codepoint_prefix_with_observer(
    tokens: &[ContextToken],
    mut observe_token: impl FnMut(),
) -> Vec<usize> {
    let mut prefix = Vec::with_capacity(tokens.len() + 1);
    prefix.push(0);
    for token in tokens {
        observe_token();
        prefix.push(prefix.last().copied().unwrap_or(0) + token.text.chars().count());
    }
    prefix
}

fn codepoint_gap(prefix: &[usize], start: usize, end: usize) -> usize {
    prefix[end] - prefix[start]
}

fn nearest_prior_intent(
    intents: &[PhraseMatch],
    proposition: &Range<usize>,
    anchor_start: usize,
    codepoint_prefix: &[usize],
) -> Option<PhraseMatch> {
    nearest_prior_intent_with_observer(intents, proposition, anchor_start, codepoint_prefix, || {})
}

fn nearest_prior_intent_with_observer(
    intents: &[PhraseMatch],
    proposition: &Range<usize>,
    anchor_start: usize,
    codepoint_prefix: &[usize],
    mut observe_comparison: impl FnMut(),
) -> Option<PhraseMatch> {
    let mut left = 0;
    let mut right = intents.len();
    while left < right {
        observe_comparison();
        let middle = left + (right - left) / 2;
        if intents[middle].end <= anchor_start {
            left = middle + 1;
        } else {
            right = middle;
        }
    }
    let nearest = *intents.get(left.checked_sub(1)?)?;
    if !in_range(&nearest, proposition)
        || codepoint_gap(codepoint_prefix, nearest.end, anchor_start)
            > MAX_EVENT_COMPONENT_GAP_CODEPOINTS
    {
        return None;
    }
    Some(nearest)
}

fn find_phrases(tokens: &[ContextToken], phrases: PhraseSet) -> Vec<PhraseMatch> {
    let mut matches = Vec::new();
    for phrase in phrases.phrases() {
        let words = phrase.split_whitespace().collect::<Vec<_>>();
        if words.is_empty() || words.len() > tokens.len() {
            continue;
        }
        for start in 0..=tokens.len() - words.len() {
            let candidate = &tokens[start..start + words.len()];
            if candidate
                .iter()
                .map(|token| token.text.as_str())
                .eq(words.iter().copied())
                && candidate
                    .iter()
                    .all(|token| token.clause == candidate[0].clause)
            {
                matches.push(PhraseMatch {
                    start,
                    end: start + words.len(),
                });
            }
        }
    }
    matches.sort_by_key(|item| (item.start, item.end));
    matches
}

fn exact_phrase(
    tokens: &[ContextToken],
    proposition: &Range<usize>,
    phrases: PhraseSet,
) -> Option<PhraseMatch> {
    phrases.phrases().iter().find_map(|phrase| {
        let words = phrase.split_whitespace().collect::<Vec<_>>();
        let candidate = &tokens[proposition.clone()];
        (candidate.len() == words.len()
            && candidate
                .iter()
                .map(|token| token.text.as_str())
                .eq(words.iter().copied()))
        .then_some(PhraseMatch {
            start: proposition.start,
            end: proposition.end,
        })
    })
}

fn proposition_ranges(
    tokens: &[ContextToken],
    configured_boundaries: &[PhraseMatch],
) -> Vec<Range<usize>> {
    let mut scope_boundaries = vec![0, tokens.len()];
    for index in 1..tokens.len() {
        if tokens[index].clause != tokens[index - 1].clause {
            scope_boundaries.push(index);
        }
    }
    scope_boundaries.sort_unstable();
    scope_boundaries.dedup();
    scope_boundaries
        .windows(2)
        .flat_map(|bounds| {
            let scope = bounds[0]..bounds[1];
            let mut ranges = Vec::new();
            let mut start = scope.start;
            for boundary in configured_boundaries
                .iter()
                .filter(|boundary| in_range(boundary, &scope))
            {
                if boundary.start > start {
                    ranges.push(start..boundary.start);
                }
                start = start.max(boundary.end);
            }
            if start < scope.end {
                ranges.push(start..scope.end);
            }
            ranges
        })
        .collect()
}

fn matches_in<'a>(
    matches: &'a [PhraseMatch],
    range: &'a Range<usize>,
) -> impl Iterator<Item = PhraseMatch> + 'a {
    matches
        .iter()
        .filter(move |item| in_range(item, range))
        .copied()
}

fn target_for_anchor(
    targets: &[PhraseMatch],
    clause: &Range<usize>,
    anchor: PhraseMatch,
    lower_bound: usize,
) -> Option<PhraseMatch> {
    targets
        .iter()
        .find(|target| {
            in_range(target, clause)
                && target.start >= anchor.end
                && target.start - anchor.end <= MAX_TARGET_GAP_TOKENS
        })
        .copied()
        .or_else(|| {
            targets
                .iter()
                .rev()
                .find(|target| {
                    in_range(target, clause)
                        && target.start >= lower_bound
                        && target.end <= anchor.start
                        && anchor.start - target.end <= MAX_TARGET_GAP_TOKENS
                })
                .copied()
        })
}

fn in_range(item: &PhraseMatch, range: &Range<usize>) -> bool {
    item.start >= range.start && item.end <= range.end
}

fn ranges_overlap(left: &Range<usize>, right: &Range<usize>) -> bool {
    left.start < right.end && right.start < left.end
}

fn token_range(tokens: &[ContextToken], phrase: PhraseMatch) -> Range<usize> {
    tokens[phrase.start].span.start..tokens[phrase.end - 1].span.end
}

fn frame_range(tokens: &[ContextToken], phrases: &[Option<PhraseMatch>]) -> Range<usize> {
    let start = phrases
        .iter()
        .flatten()
        .map(|phrase| tokens[phrase.start].span.start)
        .min()
        .expect("an event frame has one phrase");
    let end = phrases
        .iter()
        .flatten()
        .map(|phrase| tokens[phrase.end - 1].span.end)
        .max()
        .expect("an event frame has one phrase");
    start..end
}

#[cfg(test)]
mod tests {
    use crate::text::{ContextToken, TextSpan};

    use super::{PhraseMatch, codepoint_prefix_with_observer, nearest_prior_intent_with_observer};

    #[test]
    fn intent_selection_builds_one_prefix_and_checks_only_the_nearest_prior_match() {
        let tokens = (0..16_384)
            .map(|index| ContextToken {
                text: "x".to_owned(),
                span: TextSpan {
                    start: index,
                    end: index + 1,
                },
                clause: 0,
                scope: 0,
                quoted: false,
                mention: false,
            })
            .collect::<Vec<_>>();
        let intents = (0..16_370)
            .map(|index| PhraseMatch {
                start: index,
                end: index + 1,
            })
            .collect::<Vec<_>>();
        let mut prefix_visits = 0;
        let prefix = codepoint_prefix_with_observer(&tokens, || prefix_visits += 1);
        let mut search_visits = 0;

        let selected = nearest_prior_intent_with_observer(
            &intents,
            &(0..tokens.len()),
            tokens.len(),
            &prefix,
            || search_visits += 1,
        );

        assert_eq!(prefix_visits, tokens.len());
        assert!(search_visits <= 16);
        assert!(selected.is_none());
    }
}
