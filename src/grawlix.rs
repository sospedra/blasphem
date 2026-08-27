//! Symbol masking for matched toxic spans.

use crate::policy::{PolicyResult, RuleEvidence};
use crate::text::TextSpan;

const SYMBOLS: [char; 6] = ['@', '#', '$', '%', '&', '!'];

/// Collects every matched raw byte range from one result, sorted and merged.
///
/// Lexical matches always carry a raw range. Rule evidence carries one only
/// when the rule matched a span of the original text.
#[must_use]
pub fn masked_spans(result: &PolicyResult) -> Vec<TextSpan> {
    let lexical = result.lexical.matches.iter().map(|found| TextSpan {
        start: found.raw_start,
        end: found.raw_end,
    });
    let rules = result.evidence.iter().filter_map(evidence_span);

    let mut spans: Vec<TextSpan> = lexical
        .chain(rules)
        .filter(|span| span.end > span.start)
        .collect();
    spans.sort_by_key(|span| (span.start, span.end));
    merge(spans)
}

fn evidence_span(evidence: &RuleEvidence) -> Option<TextSpan> {
    let start = evidence.raw_start?;
    let end = evidence.raw_end?;
    Some(TextSpan { start, end })
}

fn merge(sorted: Vec<TextSpan>) -> Vec<TextSpan> {
    let mut merged: Vec<TextSpan> = Vec::with_capacity(sorted.len());
    for span in sorted {
        match merged.last_mut() {
            Some(last) if span.start <= last.end => last.end = last.end.max(span.end),
            _ => merged.push(span),
        }
    }
    merged
}

/// Replaces each span with symbols and returns the masked text.
///
/// Whitespace inside a span stays. Spans are clamped to character
/// boundaries, so multibyte text survives.
#[must_use]
pub fn apply_grawlix(text: &str, spans: &[TextSpan]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;

    for span in spans {
        let (start, end) = clamp(text, span);
        if start < cursor {
            continue;
        }
        out.push_str(&text[cursor..start]);
        out.extend(text[start..end].chars().enumerate().map(symbol_for));
        cursor = end;
    }

    out.push_str(&text[cursor..]);
    out
}

fn symbol_for((index, source): (usize, char)) -> char {
    if source.is_whitespace() {
        return source;
    }
    SYMBOLS[index % SYMBOLS.len()]
}

fn clamp(text: &str, span: &TextSpan) -> (usize, usize) {
    let start = floor_boundary(text, span.start.min(text.len()));
    let end = ceil_boundary(text, span.end.min(text.len()));
    (start, end.max(start))
}

fn floor_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}
