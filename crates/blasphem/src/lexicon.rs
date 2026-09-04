use std::{io::Read, str::FromStr};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MatchLevel {
    Conservative,
    Inclusive,
}

impl FromStr for MatchLevel {
    type Err = ParseLexiconError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "conservative" => Ok(Self::Conservative),
            "inclusive" => Ok(Self::Inclusive),
            _ => Err(ParseLexiconError::InvalidLevel(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexiconEntry {
    pub id: String,
    pub language: String,
    pub part_of_speech: String,
    pub category: String,
    pub stereotype: bool,
    pub lemma: String,
    pub level: MatchLevel,
}

#[derive(Debug, Error)]
pub enum ParseLexiconError {
    #[error("cannot parse Lexicon TSV: {0}")]
    Csv(#[from] csv::Error),
    #[error("invalid Lexicon level: {0}")]
    InvalidLevel(String),
    #[error("invalid Lexicon stereotype value: {0}")]
    InvalidStereotype(String),
    #[error("language code is empty")]
    EmptyLanguage,
    #[error("Lexicon row {id} has language {actual}; expected {expected}")]
    LanguageMismatch {
        id: String,
        expected: String,
        actual: String,
    },
}

#[derive(Debug, Deserialize)]
struct RawEntry {
    id: String,
    #[serde(rename = "pos")]
    part_of_speech: String,
    category: String,
    stereotype: String,
    lemma: String,
    level: String,
}

pub fn parse_lexicon(
    reader: impl Read,
    language: &str,
) -> Result<Vec<LexiconEntry>, ParseLexiconError> {
    let language = language.trim().to_ascii_uppercase();
    if language.is_empty() {
        return Err(ParseLexiconError::EmptyLanguage);
    }

    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .trim(csv::Trim::All)
        .from_reader(reader);
    let mut entries = Vec::new();

    for row in csv.deserialize::<RawEntry>() {
        let row = row?;
        let row_language = row
            .id
            .chars()
            .take_while(char::is_ascii_alphabetic)
            .collect::<String>()
            .to_ascii_uppercase();
        if row_language != language {
            return Err(ParseLexiconError::LanguageMismatch {
                id: row.id,
                expected: language,
                actual: row_language,
            });
        }
        let stereotype = match row.stereotype.to_ascii_lowercase().as_str() {
            "yes" => true,
            "no" => false,
            _ => return Err(ParseLexiconError::InvalidStereotype(row.stereotype)),
        };
        entries.push(LexiconEntry {
            id: row.id,
            language: language.clone(),
            part_of_speech: row.part_of_speech,
            category: row.category,
            stereotype,
            lemma: row.lemma,
            level: row.level.parse()?,
        });
    }

    Ok(entries)
}
