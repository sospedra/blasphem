use unicode_normalization::UnicodeNormalization;

use crate::NormalizationProfile;

pub fn normalize(profile: NormalizationProfile, text: &str) -> String {
    match profile {
        NormalizationProfile::SpanishCharabia => crate::normalize_text(text),
        NormalizationProfile::Generic => nfkc_lower(text),
        NormalizationProfile::Turkish => turkish(text),
        NormalizationProfile::Vietnamese => nfkc_lower(text),
        NormalizationProfile::Arabic => arabic(text),
        NormalizationProfile::Hindi => text.nfkc().collect(),
        NormalizationProfile::Chinese
        | NormalizationProfile::Japanese
        | NormalizationProfile::Korean => nfkc_lower(text),
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
