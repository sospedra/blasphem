use crate::Language;

use super::super::LanguageRules;
#[cfg(any(feature = "all-rules", feature = "ar", feature = "hi"))]
use super::super::{PhraseSet, RuleMatchProfile};

#[cfg(any(feature = "all-rules", feature = "ar", feature = "hi"))]
const EMPTY: PhraseSet = PhraseSet::empty();

#[cfg(any(feature = "all-rules", feature = "ar"))]
const AR_RULES: LanguageRules = LanguageRules {
    language: Language::Ar,
    version: 2,
    targets: PhraseSet::new(&["انت", "وحيدا"]),
    harm_predicates: PhraseSet::new(&["اقتلك"]),
    intent_markers: PhraseSet::new(&["لن"]),
    implicit_target_threats: PhraseSet::new(&[
        "ساقتلك",
        "14<>يا خرا يا زق يا و ال له نهايتكم عايدي اقسم بالله لو ما ترجعولنا حقنا اموتكم",
    ]),
    wish_markers: PhraseSet::new(&["اتمني"]),
    harm_outcomes: PhraseSet::new(&["تموت"]),
    implicit_target_harm_wishes: PhraseSet::new(&[
        "اتمني ان تموت",
        "ال له ياخذك يا ال يغري يا جبان يا حمار",
    ]),
    self_harm_commands: PhraseSet::new(&["اقتل نفسك", "موتی برا"]),
    strong_insults: PhraseSet::new(&["حقير", "حمار"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: EMPTY,
    copulas_or_vocatives: EMPTY,
    negators: PhraseSet::new(&["لن", "لا"]),
    reports: PhraseSet::new(&["قال", "قال لي ال طبيب", "كتب"]),
    counterspeech_markers: PhraseSet::new(&[
        "توقف عن قول",
        "من ال خطا ان تقول",
        "من ال خطا ان تقول لشخص",
    ]),
    proposition_boundaries: PhraseSet::new(&["لكن"]),
    matching: RuleMatchProfile::WordClauses,
};

#[cfg(any(feature = "all-rules", feature = "hi"))]
const HI_RULES: LanguageRules = LanguageRules {
    language: Language::Hi,
    version: 2,
    targets: PhraseSet::new(&["तू", "तुम", "तुम्हें", "तुम्हारी"]),
    harm_predicates: PhraseSet::new(&["मार दूंगा", "मारूंगा", "हड्डियाँ तोड़ दूँगा"]),
    intent_markers: PhraseSet::new(&["मैं", "ढूँढकर"]),
    implicit_target_threats: PhraseSet::new(&["इनके हाथ पैर तोड़ दो"]),
    wish_markers: PhraseSet::new(&["काश"]),
    harm_outcomes: PhraseSet::new(&["मर जाओ"]),
    implicit_target_harm_wishes: PhraseSet::new(&["ये कमीनी जिस दिन मरेगी उस दिन full पार्टी"]),
    self_harm_commands: PhraseSet::new(&["आत्महत्या कर ले", "जहर खा कर मर"]),
    strong_insults: PhraseSet::new(&["हरामी", "निहायत कमीना"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: EMPTY,
    copulas_or_vocatives: PhraseSet::new(&["है"]),
    negators: PhraseSet::new(&["नहीं", "न"]),
    reports: PhraseSet::new(&["उसने कहा", "उसने कहा कि", "उसने लिखा"]),
    counterspeech_markers: PhraseSet::new(&[
        "ऐसा कहना गलत है",
        "ऐसा कहना गलत है कि",
        "यह कहना गलत है",
    ]),
    proposition_boundaries: PhraseSet::new(&["और", "पर"]),
    matching: RuleMatchProfile::WordClauses,
};

/// Returns the static semantic rules for Arabic or Hindi.
#[must_use]
pub const fn arabic_hindi_rules(language: Language) -> Option<&'static LanguageRules> {
    match language {
        #[cfg(any(feature = "all-rules", feature = "ar"))]
        Language::Ar => Some(&AR_RULES),
        #[cfg(any(feature = "all-rules", feature = "hi"))]
        Language::Hi => Some(&HI_RULES),
        _ => None,
    }
}
