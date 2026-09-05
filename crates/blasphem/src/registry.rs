//! The compiled rule identity and profiles for every language.
//!
//! Data does not live here. The embedded artifacts sit in `embedded.rs`
//! behind the `embedded` feature, and packs arrive at run time through
//! `pack.rs`. Both paths validate a model against this registry.

use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::rule_pack::{RulePack, for_language};
use crate::runtime::RuntimeInitError;
use crate::{
    FeatureProfile, FeatureSchema, Language, LanguageRules, NormalizationProfile, RuleChannel,
    SparseModel, arabic_hindi_rules, canonical_rule_identity, cjk_rules, word_rules,
};

#[derive(Debug, Clone, Copy)]
pub struct LanguageSpec {
    pub language: Language,
    pub feature_profile: FeatureProfile,
    pub normalization_profile: NormalizationProfile,
    pub feature_schema: FeatureSchema,
}

impl LanguageSpec {
    const fn new(language: Language) -> Self {
        let profiles = language.profiles();
        Self {
            language,
            feature_profile: profiles.0,
            normalization_profile: profiles.1,
            feature_schema: profiles.2,
        }
    }
}

pub(crate) struct RegistryEntry {
    pub(crate) language: Language,
    pub(crate) feature_profile: FeatureProfile,
    pub(crate) normalization_profile: NormalizationProfile,
    pub(crate) feature_schema: FeatureSchema,
    pub(crate) rule_pack_version: u16,
    pub(crate) rule_pack_sha256: [u8; 32],
    rules: RuleCache,
}

enum RuleCache {
    Spanish(OnceLock<Box<RulePack>>),
    Static(OnceLock<&'static LanguageRules>),
}

impl RegistryEntry {
    const fn new(language: Language, rule_pack_version: u16, rule_pack_sha256: [u8; 32]) -> Self {
        let profiles = language.profiles();
        Self {
            language,
            feature_profile: profiles.0,
            normalization_profile: profiles.1,
            feature_schema: profiles.2,
            rule_pack_version,
            rule_pack_sha256,
            rules: if matches!(language, Language::Es) {
                RuleCache::Spanish(OnceLock::new())
            } else {
                RuleCache::Static(OnceLock::new())
            },
        }
    }

    /// Checks that a parsed model declares this entry's language and profiles.
    pub(crate) fn check_model(&self, model: &SparseModel) -> Result<(), String> {
        if model.language() != self.language
            || model.feature_profile() != self.feature_profile
            || model.normalization_profile() != self.normalization_profile
            || model.feature_schema() != self.feature_schema
        {
            return Err("artifact metadata mismatch".to_owned());
        }
        Ok(())
    }

    pub(crate) fn rule_channel(
        &'static self,
        lexicon: Option<&[u8]>,
    ) -> Result<RuleChannel, RuntimeInitError> {
        self.validate_rule_identity()?;
        match &self.rules {
            RuleCache::Spanish(cache) => {
                if cache.get().is_none() {
                    let rules = for_language(self.language.code())
                        .ok_or_else(|| self.invalid_rule_pack("no Spanish rule pack exists"))?;
                    let _ = cache.set(Box::new(rules));
                }
                RuleChannel::from_cached_spanish(
                    self.language,
                    lexicon,
                    cache
                        .get()
                        .expect("initialized Spanish rule cache")
                        .as_ref(),
                )
                .map_err(|source| RuntimeInitError::RuleChannel {
                    language: self.language,
                    source,
                })
            }
            RuleCache::Static(cache) => {
                if cache.get().is_none() {
                    let rules = resolve_static_rules(self.language)
                        .ok_or_else(|| self.invalid_rule_pack("no static rule pack exists"))?;
                    let _ = cache.set(rules);
                }
                RuleChannel::from_cached_static(
                    self.language,
                    lexicon,
                    cache.get().expect("initialized static rule cache"),
                )
                .map_err(|source| RuntimeInitError::RuleChannel {
                    language: self.language,
                    source,
                })
            }
        }
    }

    /// The rule-pack version the compiled rules carry for this language.
    pub(crate) fn expected_rule_pack_version(&self) -> Result<u16, RuntimeInitError> {
        if self.language == Language::Es {
            return Ok(1);
        }
        resolve_static_rules(self.language)
            .map(|rules| rules.version)
            .ok_or_else(|| self.invalid_rule_pack("no static rule pack exists"))
    }

    fn validate_rule_identity(&self) -> Result<(), RuntimeInitError> {
        let expected_version = self.expected_rule_pack_version()?;
        if self.rule_pack_version != expected_version {
            return Err(self.invalid_rule_pack(&format!(
                "rule-pack version mismatch: expected {expected_version}, found {}",
                self.rule_pack_version
            )));
        }
        let actual: [u8; 32] = Sha256::digest(canonical_rule_identity(self.language)).into();
        if actual != self.rule_pack_sha256 {
            return Err(self.invalid_rule_pack("rule-pack digest mismatch"));
        }
        Ok(())
    }

    fn invalid_rule_pack(&self, reason: &str) -> RuntimeInitError {
        RuntimeInitError::InvalidRulePack {
            language: self.language,
            reason: reason.to_owned(),
        }
    }
}

fn resolve_static_rules(language: Language) -> Option<&'static LanguageRules> {
    word_rules(language)
        .or_else(|| arabic_hindi_rules(language))
        .or_else(|| cjk_rules(language))
}

static LANGUAGE_SPECS: [LanguageSpec; 15] = [
    LanguageSpec::new(Language::En),
    LanguageSpec::new(Language::Zh),
    LanguageSpec::new(Language::Es),
    LanguageSpec::new(Language::Ar),
    LanguageSpec::new(Language::Ms),
    LanguageSpec::new(Language::Pt),
    LanguageSpec::new(Language::Fr),
    LanguageSpec::new(Language::Hi),
    LanguageSpec::new(Language::Ru),
    LanguageSpec::new(Language::Ja),
    LanguageSpec::new(Language::De),
    LanguageSpec::new(Language::Tr),
    LanguageSpec::new(Language::Vi),
    LanguageSpec::new(Language::Ko),
    LanguageSpec::new(Language::It),
];

static REGISTRY_ENTRIES: [RegistryEntry; 15] = [
    RegistryEntry::new(
        Language::En,
        1,
        digest("83f12c208705486045927869c1adc40d5987064de60cc5665bb24e5ee20f1bd3"),
    ),
    RegistryEntry::new(
        Language::Zh,
        1,
        digest("6faedfcb637f60f23a58ff24e3473023a2484707774fe90277cb217c9b3d7941"),
    ),
    RegistryEntry::new(
        Language::Es,
        1,
        digest("8bb5ad315f8abe69611cb192bfdf3712d8005cd331565547ec87573720a48246"),
    ),
    RegistryEntry::new(
        Language::Ar,
        2,
        digest("a882fa77392de6d327db51fc15a97729d63378df59b9ae564360e9b86aaff7ef"),
    ),
    RegistryEntry::new(
        Language::Ms,
        1,
        digest("32b05ecb070b353590ed6b2f29e4d6a13023ec4b33cfa7f48679f16475182861"),
    ),
    RegistryEntry::new(
        Language::Pt,
        1,
        digest("76b8c7927042582bee11d3c4444e1cf61c199782a223d1f95e60ea536c2a69aa"),
    ),
    RegistryEntry::new(
        Language::Fr,
        1,
        digest("d944d890212aefb86324d2d6dd4518a724f0e6de6daefee2706fb7be0e3fabc6"),
    ),
    RegistryEntry::new(
        Language::Hi,
        2,
        digest("eed30552d88d22ecc5cb33da64aaa4786299b78f046c3ff5fd0a1f744f6fb275"),
    ),
    RegistryEntry::new(
        Language::Ru,
        1,
        digest("2d3f7288619d2e801eeb2a52ea7243207752c08ee49c54098c70fb294c059b01"),
    ),
    RegistryEntry::new(
        Language::Ja,
        1,
        digest("8474720d2e8e0e85ba97e3a08f949dfa9b429b1538bff6a7b593bd32d6c9b2a1"),
    ),
    RegistryEntry::new(
        Language::De,
        1,
        digest("b48d46b1d9a84a9cf4781cd32d16f08acbeef426811919bdb6f870d52b837f96"),
    ),
    RegistryEntry::new(
        Language::Tr,
        1,
        digest("1ead20c976e68da4fdb683cdebbe680e40d7561bed08b807f595925fe663a932"),
    ),
    RegistryEntry::new(
        Language::Vi,
        1,
        digest("13fcb5ae0b82f081bd942acd347469290b369e39a159287a646ebcb4a835ff64"),
    ),
    RegistryEntry::new(
        Language::Ko,
        1,
        digest("51f02655fed2ffe57b000f4f557cf88ff03ee15fd5cb1e9b356b0fffae879a35"),
    ),
    RegistryEntry::new(
        Language::It,
        2,
        digest("c1e8cf9f3612964383114fe3108d5ae08c2d98c3662dceb24f56219c9dc8129e"),
    ),
];

pub fn language_spec(language: Language) -> &'static LanguageSpec {
    &LANGUAGE_SPECS[language.index()]
}

pub(crate) fn registry_entry(language: Language) -> &'static RegistryEntry {
    &REGISTRY_ENTRIES[language.index()]
}

pub(crate) const fn digest(value: &str) -> [u8; 32] {
    let bytes = value.as_bytes();
    assert!(bytes.len() == 64, "SHA-256 digest length");
    let mut output = [0_u8; 32];
    let mut index = 0;
    while index < output.len() {
        output[index] = (hex_nibble(bytes[index * 2]) << 4) | hex_nibble(bytes[index * 2 + 1]);
        index += 1;
    }
    output
}

const fn hex_nibble(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => panic!("invalid SHA-256 digit"),
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{REGISTRY_ENTRIES, RegistryEntry, registry_entry};
    use crate::{Language, canonical_rule_identity};

    #[test]
    fn registry_holds_one_valid_rule_identity_per_language() {
        assert_eq!(REGISTRY_ENTRIES.len(), Language::ALL.len());

        for (expected, entry) in Language::ALL.iter().copied().zip(&REGISTRY_ENTRIES) {
            assert_eq!(entry.language, expected);
            assert_eq!(registry_entry(expected).language, expected);
            entry.validate_rule_identity().expect("valid rule identity");
            assert_eq!(
                entry.expected_rule_pack_version().expect("rule version"),
                entry.rule_pack_version
            );
        }
    }

    #[test]
    fn registry_rejects_a_rule_pack_version_mismatch() {
        let entry = RegistryEntry::new(Language::En, 99, rule_digest(Language::En));

        let error = entry
            .validate_rule_identity()
            .expect_err("wrong rule version");
        assert!(error.to_string().contains("version mismatch"));
    }

    #[test]
    fn registry_rejects_a_rule_pack_digest_mismatch() {
        let entry = RegistryEntry::new(Language::En, 1, [0; 32]);

        let error = entry
            .validate_rule_identity()
            .expect_err("wrong rule digest");
        assert!(error.to_string().contains("digest mismatch"));
    }

    fn rule_digest(language: Language) -> [u8; 32] {
        Sha256::digest(canonical_rule_identity(language)).into()
    }
}
