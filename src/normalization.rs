use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::NormalizationProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NormalizationError {
    #[error("the version-one Spanish profile cannot use the version-two normalizer")]
    LegacyProfile,
}

pub fn normalize_v2(
    profile: NormalizationProfile,
    text: &str,
) -> Result<String, NormalizationError> {
    match profile {
        NormalizationProfile::EsLegacyCharabiaV1 => Err(NormalizationError::LegacyProfile),
        NormalizationProfile::GenericV2 => Ok(nfkc_lower(text)),
        NormalizationProfile::TurkishV2 => Ok(turkish(text)),
        NormalizationProfile::VietnameseV2 => Ok(nfkc_lower(text)),
        NormalizationProfile::ArabicV2 => Ok(arabic(text)),
        NormalizationProfile::HindiV2 => Ok(text.nfkc().collect()),
        NormalizationProfile::ChineseV2
        | NormalizationProfile::JapaneseV2
        | NormalizationProfile::KoreanV2 => Ok(nfkc_lower(text)),
    }
}

fn nfkc_lower(text: &str) -> String {
    text.nfkc().flat_map(char::to_lowercase).collect()
}

fn turkish(text: &str) -> String {
    text.nfkc()
        .flat_map(|ch| match ch {
            'I' => 'ı'.to_lowercase(),
            'İ' => 'i'.to_lowercase(),
            other => other.to_lowercase(),
        })
        .collect()
}

fn arabic(text: &str) -> String {
    text.nfkc()
        .filter_map(|ch| match ch {
            '\u{0640}'
            | '\u{0610}'..='\u{061a}'
            | '\u{064b}'..='\u{065f}'
            | '\u{0670}'
            | '\u{06d6}'..='\u{06ed}' => None,
            '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => Some('\u{0627}'),
            other => Some(other),
        })
        .collect()
}
