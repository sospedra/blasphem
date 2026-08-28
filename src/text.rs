use std::ops::Range;

use charabia::Tokenize;
use unicode_normalization::UnicodeNormalization;
use unicode_script::{Script, UnicodeScript};
use unicode_security::skeleton;

use crate::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateViewKind {
    Normalized,
    Confusable,
    Evasion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct CandidateView {
    text: String,
    byte_spans: Vec<TextSpan>,
}

impl CandidateView {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn original_span(&self, start: usize, end: usize) -> Option<Range<usize>> {
        if start >= end
            || end > self.text.len()
            || !self.text.is_char_boundary(start)
            || !self.text.is_char_boundary(end)
        {
            return None;
        }

        let spans = self.byte_spans.get(start..end)?;
        let start = spans.iter().map(|span| span.start).min()?;
        let end = spans.iter().map(|span| span.end).max()?;
        Some(start..end)
    }

    fn new() -> Self {
        Self {
            text: String::new(),
            byte_spans: Vec::new(),
        }
    }

    fn push(&mut self, value: &str, span: TextSpan) {
        self.text.push_str(value);
        for _ in value.bytes() {
            self.byte_spans.push(span);
        }
    }

    fn push_space(&mut self, span: TextSpan) {
        if !self.text.is_empty() {
            self.push(" ", span);
        }
    }
}

#[derive(Debug)]
pub struct TextDocument {
    original: String,
    context_tokens: Vec<ContextToken>,
    normalized: CandidateView,
    confusable: CandidateView,
    evasion: CandidateView,
}

impl TextDocument {
    #[must_use]
    pub fn new(original: &str) -> Self {
        Self::build(original, None)
    }

    pub(crate) fn for_rule_language(original: &str, language: Language) -> Self {
        Self::build(original, Some(language))
    }

    fn build(original: &str, rule_language: Option<Language>) -> Self {
        let tokens = original
            .tokenize()
            .filter(|token| token.is_word())
            .map(|token| WordToken {
                text: token.lemma().to_owned(),
                span: TextSpan {
                    start: token.byte_start,
                    end: token.byte_end,
                },
            })
            .collect::<Vec<_>>();
        let context_words = original
            .tokenize()
            .filter(|token| token.is_word() || token.is_stopword())
            .map(|token| WordToken {
                text: rule_token_text(
                    original,
                    token.lemma(),
                    token.byte_start,
                    token.byte_end,
                    rule_language,
                ),
                span: TextSpan {
                    start: token.byte_start,
                    end: token.byte_end,
                },
            })
            .collect::<Vec<_>>();
        let context_tokens = context_tokens(original, &context_words);
        let confusable_tokens = confusable_tokens(&tokens);

        Self {
            original: original.to_owned(),
            context_tokens,
            normalized: normalized_view(&tokens),
            confusable: normalized_view(&confusable_tokens),
            evasion: evasion_view(&tokens),
        }
    }

    #[must_use]
    pub fn original(&self) -> &str {
        &self.original
    }

    #[must_use]
    pub fn view(&self, kind: CandidateViewKind) -> &CandidateView {
        match kind {
            CandidateViewKind::Normalized => &self.normalized,
            CandidateViewKind::Confusable => &self.confusable,
            CandidateViewKind::Evasion => &self.evasion,
        }
    }

    pub(crate) fn context_tokens(&self) -> &[ContextToken] {
        &self.context_tokens
    }
}

fn rule_token_text(
    original: &str,
    lemma: &str,
    start: usize,
    end: usize,
    language: Option<Language>,
) -> String {
    let raw = &original[start..end];
    match language {
        Some(Language::Tr) => raw
            .nfkc()
            .flat_map(|character| match character {
                'I' => 'ı'.to_lowercase(),
                'İ' => 'i'.to_lowercase(),
                other => other.to_lowercase(),
            })
            .collect(),
        Some(Language::Vi) => raw.nfkc().flat_map(char::to_lowercase).collect(),
        _ => lemma.to_owned(),
    }
}

#[derive(Debug)]
struct WordToken {
    text: String,
    span: TextSpan,
}

#[derive(Debug, Clone)]
pub(crate) struct ContextToken {
    pub(crate) text: String,
    pub(crate) span: TextSpan,
    pub(crate) clause: usize,
    pub(crate) scope: usize,
    pub(crate) quoted: bool,
    pub(crate) mention: bool,
}

fn context_tokens(original: &str, tokens: &[WordToken]) -> Vec<ContextToken> {
    let quote_ranges = quote_ranges(original);
    let mut clause = 0;
    let mut scope = 0;
    let mut previous_end = 0;

    tokens
        .iter()
        .map(|token| {
            let separator = &original[previous_end..token.span.start];
            if separator.chars().any(is_clause_boundary) {
                clause += 1;
                scope += 1;
            } else if separator.chars().any(is_scope_boundary) {
                scope += 1;
            }
            previous_end = token.span.end;
            ContextToken {
                text: token.text.clone(),
                span: token.span,
                clause,
                scope,
                quoted: quote_ranges
                    .iter()
                    .any(|range| range.start <= token.span.start && token.span.end <= range.end),
                mention: token.span.start > 0
                    && original[..token.span.start]
                        .chars()
                        .next_back()
                        .is_some_and(|character| character == '@'),
            }
        })
        .collect()
}

fn is_clause_boundary(character: char) -> bool {
    matches!(
        character,
        '.' | '!' | '?' | ';' | '؟' | '۔' | '।' | '॥' | '。' | '！' | '？' | '；' | '\n' | '\r'
    )
}

fn is_scope_boundary(character: char) -> bool {
    matches!(character, ',' | '，')
}

fn quote_ranges(original: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut straight_start = None;
    let mut curly_start = None;
    let mut low_double_start = None;
    let mut guillemet_start = None;

    for (index, character) in original.char_indices() {
        match character {
            '"' => {
                if let Some(start) = straight_start.take() {
                    ranges.push(start..index + character.len_utf8());
                } else {
                    straight_start = Some(index);
                }
            }
            '„' => low_double_start = Some(index),
            '“' => {
                if let Some(start) = low_double_start.take() {
                    ranges.push(start..index + character.len_utf8());
                } else {
                    curly_start = Some(index);
                }
            }
            '”' => {
                if let Some(start) = curly_start.take() {
                    ranges.push(start..index + character.len_utf8());
                }
            }
            '«' => guillemet_start = Some(index),
            '»' => {
                if let Some(start) = guillemet_start.take() {
                    ranges.push(start..index + character.len_utf8());
                }
            }
            _ => {}
        }
    }
    ranges
}

fn normalized_view(tokens: &[WordToken]) -> CandidateView {
    let mut view = CandidateView::new();
    let mut previous = None;
    for token in tokens {
        if let Some(previous) = previous {
            view.push_space(TextSpan {
                start: previous,
                end: token.span.start,
            });
        }
        view.push(&token.text, token.span);
        previous = Some(token.span.end);
    }
    view
}

/// Rebuilds each visual word from the tokens that touch it, then folds it as
/// one unit. charabia splits a word wherever the script changes, so "idiоt"
/// arrives as three tokens; folding them apart would miss the homoglyph, and
/// folding the skeleton before tokenizing invented boundaries inside Arabic,
/// Cyrillic, and Hangul words.
fn confusable_tokens(tokens: &[WordToken]) -> Vec<WordToken> {
    let mut words: Vec<WordToken> = Vec::new();
    for token in tokens {
        match words.last_mut() {
            Some(last) if last.span.end == token.span.start => {
                last.text.push_str(&token.text);
                last.span.end = token.span.end;
            }
            _ => words.push(WordToken {
                text: token.text.clone(),
                span: token.span,
            }),
        }
    }
    for word in &mut words {
        word.text = confusable_fold(&word.text);
    }
    words
}

fn confusable_fold(text: &str) -> String {
    let foldable = text.chars().any(|character| {
        matches!(
            character.script(),
            Script::Latin | Script::Cyrillic | Script::Greek
        )
    });
    if !foldable {
        return text.to_owned();
    }
    skeleton(text).collect()
}

fn evasion_view(tokens: &[WordToken]) -> CandidateView {
    let mut view = CandidateView::new();
    let mut index = 0;
    let mut previous = None;

    while index < tokens.len() {
        let run_end = single_letter_run_end(tokens, index);
        if run_end.saturating_sub(index) >= 3 {
            let first = &tokens[index];
            let last = &tokens[run_end - 1];
            if let Some(previous) = previous {
                view.push_space(TextSpan {
                    start: previous,
                    end: first.span.start,
                });
            }
            for token in &tokens[index..run_end] {
                view.push(&token.text, token.span);
            }
            previous = Some(last.span.end);
            index = run_end;
            continue;
        }

        let token = &tokens[index];
        if let Some(previous) = previous {
            view.push_space(TextSpan {
                start: previous,
                end: token.span.start,
            });
        }
        view.push(&map_mixed_token_digits(&token.text), token.span);
        previous = Some(token.span.end);
        index += 1;
    }

    view
}

fn single_letter_run_end(tokens: &[WordToken], start: usize) -> usize {
    if tokens[start].text.chars().count() != 1 {
        return start;
    }

    let mut end = start + 1;
    while end < tokens.len()
        && tokens[end - 1].span.end < tokens[end].span.start
        && tokens[end].text.chars().count() == 1
    {
        end += 1;
    }
    end
}

fn map_mixed_token_digits(token: &str) -> String {
    let has_letter = token.chars().any(char::is_alphabetic);
    let has_digit = token.chars().any(|character| character.is_ascii_digit());
    if !has_letter || !has_digit {
        return token.to_owned();
    }

    token
        .chars()
        .map(|character| match character {
            '0' => 'o',
            '1' => 'i',
            '3' => 'e',
            '4' => 'a',
            '5' => 's',
            '7' => 't',
            _ => character,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::Language;

    use super::{TextDocument, TextSpan, WordToken, single_letter_run_end};

    #[test]
    fn does_not_join_adjacent_single_letter_tokens() {
        let tokens = vec![
            WordToken {
                text: "i".to_owned(),
                span: TextSpan { start: 0, end: 1 },
            },
            WordToken {
                text: "d".to_owned(),
                span: TextSpan { start: 1, end: 2 },
            },
            WordToken {
                text: "i".to_owned(),
                span: TextSpan { start: 2, end: 3 },
            },
        ];

        assert_eq!(single_letter_run_end(&tokens, 0), 1);
    }

    #[test]
    fn context_tokens_include_normal_words_and_stop_words() {
        let document = TextDocument::new("Je vais te tuer");
        let tokens = document
            .context_tokens()
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(tokens, ["je", "vais", "te", "tuer"]);
    }

    #[test]
    fn german_low_high_double_quotes_mark_the_enclosed_tokens() {
        let document = TextDocument::new("Sie meldete „ich werde dich töten“");
        let quoted = document
            .context_tokens()
            .iter()
            .filter(|token| token.quoted)
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(quoted, ["ich", "werde", "dich", "toten"]);
    }

    #[test]
    fn vietnamese_rule_tokens_preserve_tone_marks() {
        let document = TextDocument::for_rule_language("Mày ngủ. Từ từ đi", Language::Vi);
        let tokens = document
            .context_tokens()
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(tokens, ["mày", "ngủ", "từ", "từ", "đi"]);
    }

    #[test]
    fn turkish_rule_tokens_apply_turkish_case_mapping() {
        let original =
            TextDocument::for_rule_language("Seni bulunca bütün dişlerini kıracağım", Language::Tr);
        let uppercase =
            TextDocument::for_rule_language("SENİ BULUNCA BÜTÜN DİŞLERİNİ KIRACAĞIM", Language::Tr);
        let token_text = |document: &TextDocument| {
            document
                .context_tokens()
                .iter()
                .map(|token| token.text.clone())
                .collect::<Vec<_>>()
        };

        assert_eq!(
            token_text(&original),
            ["seni", "bulunca", "bütün", "dişlerini", "kıracağım"]
        );
        assert_eq!(token_text(&uppercase), token_text(&original));
    }
}
