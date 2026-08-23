use super::{LanguageRules, PhraseSet};

const RULE_IDENTITY_MAGIC: &[u8] = b"TOXRULE1";

#[must_use]
pub fn canonical_rule_identity_for(rules: &LanguageRules) -> Vec<u8> {
    let fields = [
        rules.targets,
        rules.harm_predicates,
        rules.intent_markers,
        rules.implicit_target_threats,
        rules.wish_markers,
        rules.harm_outcomes,
        rules.implicit_target_harm_wishes,
        rules.self_harm_commands,
        rules.strong_insults,
        rules.implicit_target_directed_insults,
        rules.negative_sentiment,
        rules.copulas_or_vocatives,
        rules.negators,
        rules.reports,
        rules.counterspeech_markers,
        rules.proposition_boundaries,
    ];
    let mut output = Vec::new();
    output.extend_from_slice(RULE_IDENTITY_MAGIC);
    output.extend_from_slice(rules.language.code().as_bytes());
    output.extend_from_slice(&rules.version.to_le_bytes());
    output.push(rules.matching as u8);
    for (ordinal, phrases) in fields.into_iter().enumerate() {
        encode_phrase_set(&mut output, ordinal, phrases);
    }
    output
}

fn encode_phrase_set(output: &mut Vec<u8>, ordinal: usize, phrases: PhraseSet) {
    output.push(u8::try_from(ordinal).expect("rule field ordinal fits in u8"));
    output.extend_from_slice(
        &u32::try_from(phrases.phrases().len())
            .expect("rule phrase count fits in u32")
            .to_le_bytes(),
    );
    for phrase in phrases.phrases() {
        output.extend_from_slice(
            &u32::try_from(phrase.len())
                .expect("rule phrase length fits in u32")
                .to_le_bytes(),
        );
        output.extend_from_slice(phrase.as_bytes());
    }
}
