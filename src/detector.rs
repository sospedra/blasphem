use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::Arc;
use unicode_script::{Script, UnicodeScript};

use crate::{
    AnalysisContext, CandidateViewKind, LexiconEntry, MatchLevel, PolicyResult, TextDocument,
};
use aho_corasick::{AhoCorasick, AhoCorasickBuilder, BuildError, MatchKind};
use charabia::Tokenize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DetectorError {
    #[error("the lexicon has no usable entries")]
    EmptyLexicon,
    #[error("cannot build the matcher: {0}")]
    Matcher(#[from] BuildError),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexiconMatch {
    pub entry: Arc<LexiconEntry>,
    pub matched_text: String,
    pub matched_confusable_view: bool,
    pub view: CandidateViewKind,
    pub normalized_start: usize,
    pub normalized_end: usize,
    pub raw_start: usize,
    pub raw_end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub normalized_text: String,
    pub score: f64,
    pub matches: Vec<LexiconMatch>,
}

impl Detection {
    #[must_use]
    pub fn is_match(&self) -> bool {
        !self.matches.is_empty()
    }
}

/// Languages whose sparse model also reads lexicon hits, as marker words appended to the text.
///
/// ZH and JA stay out: their lexica cover 5% and 22% of toxic validation rows, and a marker that
/// strong moves the boundary above the toxic rows that carry no lexicon word.
#[must_use]
pub const fn uses_lexicon_features(language: crate::Language) -> bool {
    !matches!(language, crate::Language::Zh | crate::Language::Ja)
}

/// Appends one marker word per matched lexicon category, so the sparse model can weight lexicon hits.
#[must_use]
pub fn lexicon_marked_text(text: &str, matches: &[LexiconMatch]) -> String {
    let mut categories: Vec<&str> = matches
        .iter()
        .map(|found| found.entry.category.as_str())
        .collect();
    categories.sort_unstable();
    categories.dedup();
    if categories.is_empty() {
        return text.to_owned();
    }
    let mut marked = String::with_capacity(text.len() + 12 * (categories.len() + 1));
    marked.push_str(text);
    marked.push_str(" lexhit");
    for category in categories {
        marked.push_str(" lexcat");
        marked.push_str(category);
    }
    marked
}

#[derive(Debug)]
struct PatternIndex {
    matcher: AhoCorasick,
    patterns: Vec<String>,
    entry_indices: Vec<Vec<usize>>,
}

impl PatternIndex {
    fn build(
        entries: &[Arc<LexiconEntry>],
        transform: impl Fn(&str) -> String,
    ) -> Result<Self, DetectorError> {
        let mut by_pattern = BTreeMap::<String, Vec<usize>>::new();
        for (index, entry) in entries.iter().enumerate() {
            for pattern in surface_patterns(entry, transform(&entry.lemma)) {
                if !pattern.is_empty() {
                    by_pattern.entry(pattern).or_default().push(index);
                }
            }
        }
        if by_pattern.is_empty() {
            return Err(DetectorError::EmptyLexicon);
        }

        let (patterns, entry_indices): (Vec<_>, Vec<_>) = by_pattern.into_iter().unzip();
        let matcher = AhoCorasickBuilder::new()
            .match_kind(MatchKind::Standard)
            .build(&patterns)?;

        Ok(Self {
            matcher,
            patterns,
            entry_indices,
        })
    }

    fn matching_entries(&self, text: &str) -> Vec<(usize, &str, usize, usize)> {
        let mut matches = Vec::new();
        for found in self.matcher.find_overlapping_iter(text) {
            if !has_word_boundaries(text, found.start(), found.end()) {
                continue;
            }
            let pattern_index = found.pattern().as_usize();
            let pattern = self.patterns[pattern_index].as_str();
            matches.extend(
                self.entry_indices[pattern_index]
                    .iter()
                    .copied()
                    .map(|entry_index| (entry_index, pattern, found.start(), found.end())),
            );
        }
        matches
    }
}

fn surface_patterns(entry: &LexiconEntry, base: String) -> BTreeSet<String> {
    let mut patterns = BTreeSet::from([base.clone()]);
    let is_spanish_nominal = entry.language.trim().eq_ignore_ascii_case("ES")
        && matches!(entry.part_of_speech.trim(), "n" | "N" | "a" | "A")
        && !base.contains(char::is_whitespace);
    if !is_spanish_nominal {
        return patterns;
    }

    if let Some(stem) = base.strip_suffix('z') {
        patterns.insert(format!("{stem}ces"));
    } else if base.ends_with(['a', 'e', 'i', 'o', 'u']) {
        patterns.insert(format!("{base}s"));
    } else if base.ends_with('l') {
        patterns.insert(format!("{base}es"));
    }

    patterns
}

#[derive(Debug)]
pub struct Detector {
    entries: Vec<Arc<LexiconEntry>>,
    normalized: Option<PatternIndex>,
    confusable: Option<PatternIndex>,
    evasion: Option<PatternIndex>,
}

impl Detector {
    pub fn new(entries: Vec<LexiconEntry>) -> Result<Self, DetectorError> {
        if entries.is_empty() {
            return Err(DetectorError::EmptyLexicon);
        }
        let entries = entries.into_iter().map(Arc::new).collect::<Vec<_>>();
        let normalized = PatternIndex::build(&entries, |text| {
            TextDocument::new(text)
                .view(CandidateViewKind::Normalized)
                .text()
                .to_owned()
        })?;
        let confusable = PatternIndex::build(&entries, |text| {
            TextDocument::new(text)
                .view(CandidateViewKind::Confusable)
                .text()
                .to_owned()
        })?;
        let evasion = PatternIndex::build(&entries, |text| {
            TextDocument::new(text)
                .view(CandidateViewKind::Evasion)
                .text()
                .to_owned()
        })?;
        Ok(Self {
            entries,
            normalized: Some(normalized),
            confusable: Some(confusable),
            evasion: Some(evasion),
        })
    }

    /// Creates a detector that runs rule and sparse channels without HurtLex.
    #[must_use]
    pub const fn rules_only() -> Self {
        Self {
            entries: Vec::new(),
            normalized: None,
            confusable: None,
            evasion: None,
        }
    }

    #[must_use]
    pub fn check(&self, text: &str) -> Detection {
        let document = TextDocument::new(text);
        let normalized_text = document
            .view(CandidateViewKind::Normalized)
            .text()
            .to_owned();
        let mut seen = HashSet::new();
        let mut matches = Vec::new();

        for (index, view) in [
            (self.normalized.as_ref(), CandidateViewKind::Normalized),
            (self.confusable.as_ref(), CandidateViewKind::Confusable),
            (self.evasion.as_ref(), CandidateViewKind::Evasion),
        ] {
            if let Some(index) = index {
                self.collect_matches(index, document.view(view), view, &mut seen, &mut matches);
            }
        }

        let score = matches.iter().map(match_score).fold(0.0_f64, f64::max);

        Detection {
            normalized_text,
            score,
            matches,
        }
    }

    #[must_use]
    pub fn analyze(&self, text: &str, context: AnalysisContext<'_>) -> PolicyResult {
        crate::policy::analyze(self, text, context)
    }

    fn collect_matches(
        &self,
        index: &PatternIndex,
        view: &crate::CandidateView,
        kind: CandidateViewKind,
        seen: &mut HashSet<(usize, usize, usize)>,
        output: &mut Vec<LexiconMatch>,
    ) {
        for (entry_index, matched_text, start, end) in index.matching_entries(view.text()) {
            let Some(raw_span) = view.original_span(start, end) else {
                continue;
            };
            if !seen.insert((entry_index, raw_span.start, raw_span.end)) {
                continue;
            }
            output.push(LexiconMatch {
                entry: Arc::clone(&self.entries[entry_index]),
                matched_text: matched_text.to_owned(),
                matched_confusable_view: kind == CandidateViewKind::Confusable,
                view: kind,
                normalized_start: start,
                normalized_end: end,
                raw_start: raw_span.start,
                raw_end: raw_span.end,
            });
        }
    }
}

#[must_use]
pub fn normalize_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for token in text.tokenize().filter(|token| token.is_word()) {
        if !output.is_empty() {
            output.push(' ');
        }
        output.push_str(token.lemma());
    }
    output
}

fn has_word_boundaries(text: &str, start: usize, end: usize) -> bool {
    let matched = &text[start..end];
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    is_boundary(before, matched.chars().next()) && is_boundary(after, matched.chars().next_back())
}

/// Two adjacent characters form a word boundary when the outside one is not alphanumeric, or when
/// either belongs to a script written without spaces between words. Hangul stays out: Korean
/// spaces its words, and its clean control ko-c12 pins the boundary once inner matches fire.
fn is_boundary(outside: Option<char>, inside: Option<char>) -> bool {
    match (outside, inside) {
        (None, _) | (_, None) => true,
        (Some(outside), Some(inside)) => {
            !outside.is_alphanumeric() || unspaced_script(outside) || unspaced_script(inside)
        }
    }
}

fn unspaced_script(character: char) -> bool {
    matches!(
        character.script(),
        Script::Han | Script::Hiragana | Script::Katakana
    )
}

fn match_score(found: &LexiconMatch) -> f64 {
    match (found.entry.level, found.view) {
        (MatchLevel::Conservative, CandidateViewKind::Normalized) => 1.0,
        (MatchLevel::Conservative, CandidateViewKind::Confusable | CandidateViewKind::Evasion) => {
            0.9
        }
        (MatchLevel::Inclusive, CandidateViewKind::Normalized) => 0.6,
        (MatchLevel::Inclusive, CandidateViewKind::Confusable | CandidateViewKind::Evasion) => 0.5,
    }
}
