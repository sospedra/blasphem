//! HurtLex lexica compiled into the binary.
//!
//! The files are CC-BY-SA-4.0. See NOTICE for the attribution this
//! obligation requires.

use crate::language::Language;
use crate::runtime::{NudgeDetector, RuntimeInitError};

/// The pinned HurtLex 1.2 rows for one language.
#[must_use]
pub const fn embedded_hurtlex_bytes(language: Language) -> &'static [u8] {
    match language {
        Language::En => include_bytes!("../data/raw-v1/hurtlex/EN/1.2/hurtlex_EN.tsv"),
        Language::Zh => include_bytes!("../data/raw-v1/hurtlex/ZH/1.2/hurtlex_ZH.tsv"),
        Language::Es => include_bytes!("../data/raw-v1/hurtlex/ES/1.2/hurtlex_ES.tsv"),
        Language::Ar => include_bytes!("../data/raw-v1/hurtlex/AR/1.2/hurtlex_AR.tsv"),
        Language::Ms => include_bytes!("../data/raw-v1/hurtlex/ID/1.2/hurtlex_ID.tsv"),
        Language::Pt => include_bytes!("../data/raw-v1/hurtlex/PT/1.2/hurtlex_PT.tsv"),
        Language::Fr => include_bytes!("../data/raw-v1/hurtlex/FR/1.2/hurtlex_FR.tsv"),
        Language::Hi => include_bytes!("../data/raw-v1/hurtlex/HI/1.2/hurtlex_HI.tsv"),
        Language::Ru => include_bytes!("../data/raw-v1/hurtlex/RU/1.2/hurtlex_RU.tsv"),
        Language::Ja => include_bytes!("../data/raw-v1/hurtlex/JA/1.2/hurtlex_JA.tsv"),
        Language::De => include_bytes!("../data/raw-v1/hurtlex/DE/1.2/hurtlex_DE.tsv"),
        Language::Tr => include_bytes!("../data/raw-v1/hurtlex/TR/1.2/hurtlex_TR.tsv"),
        Language::Vi => include_bytes!("../data/raw-v1/hurtlex/VI/1.2/hurtlex_VI.tsv"),
        Language::Ko => include_bytes!("../data/raw-v1/hurtlex/KO/1.2/hurtlex_KO.tsv"),
        Language::It => include_bytes!("../data/raw-v1/hurtlex/IT/1.2/hurtlex_IT.tsv"),
    }
}

/// Builds a detector from the compiled-in lexicon for one language.
///
/// # Errors
///
/// Returns an error when the embedded resources are missing or invalid.
pub fn embedded_detector(language: Language) -> Result<NudgeDetector, RuntimeInitError> {
    NudgeDetector::from_hurtlex_bytes(language, Some(embedded_hurtlex_bytes(language)))
}
