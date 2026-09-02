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
    pub(crate) artifact: &'static [u8],
    pub(crate) artifact_sha256: [u8; 32],
    pub(crate) feature_profile: FeatureProfile,
    pub(crate) normalization_profile: NormalizationProfile,
    pub(crate) feature_schema: FeatureSchema,
    pub(crate) rule_pack_version: u16,
    pub(crate) rule_pack_sha256: [u8; 32],
    pub(crate) hurtlex_sha256: Option<[u8; 32]>,
    model: OnceLock<Result<SparseModel, String>>,
    rules: RuleCache,
}

enum RuleCache {
    Spanish(OnceLock<Box<RulePack>>),
    V2(OnceLock<&'static LanguageRules>),
}

impl RegistryEntry {
    const fn new(
        language: Language,
        artifact: &'static [u8],
        artifact_sha256: [u8; 32],
        rule_pack_version: u16,
        rule_pack_sha256: [u8; 32],
        hurtlex_sha256: Option<[u8; 32]>,
    ) -> Self {
        let profiles = language.profiles();
        Self {
            language,
            artifact,
            artifact_sha256,
            feature_profile: profiles.0,
            normalization_profile: profiles.1,
            feature_schema: profiles.2,
            rule_pack_version,
            rule_pack_sha256,
            hurtlex_sha256,
            model: OnceLock::new(),
            rules: if matches!(language, Language::Es) {
                RuleCache::Spanish(OnceLock::new())
            } else {
                RuleCache::V2(OnceLock::new())
            },
        }
    }

    pub(crate) fn model(&'static self) -> Result<&'static SparseModel, RuntimeInitError> {
        self.model
            .get_or_init(|| self.parse_model())
            .as_ref()
            .map_err(|reason| RuntimeInitError::InvalidEmbeddedModel {
                language: self.language,
                reason: reason.clone(),
            })
    }

    pub(crate) fn rule_channel(
        &'static self,
        hurtlex: Option<&[u8]>,
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
                    hurtlex,
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
            RuleCache::V2(cache) => {
                if cache.get().is_none() {
                    let rules = resolve_v2_rules(self.language)
                        .ok_or_else(|| self.invalid_rule_pack("no V2 rule pack exists"))?;
                    let _ = cache.set(rules);
                }
                RuleChannel::from_cached_v2(
                    self.language,
                    hurtlex,
                    cache.get().expect("initialized V2 rule cache"),
                )
                .map_err(|source| RuntimeInitError::RuleChannel {
                    language: self.language,
                    source,
                })
            }
        }
    }

    fn validate_rule_identity(&self) -> Result<(), RuntimeInitError> {
        let expected_version = if self.language == Language::Es {
            1
        } else {
            resolve_v2_rules(self.language)
                .ok_or_else(|| self.invalid_rule_pack("no V2 rule pack exists"))?
                .version
        };
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

    fn parse_model(&self) -> Result<SparseModel, String> {
        let actual: [u8; 32] = Sha256::digest(self.artifact).into();
        if actual != self.artifact_sha256 {
            return Err("artifact digest mismatch".to_owned());
        }
        let model = SparseModel::from_bytes(self.artifact).map_err(|error| error.to_string())?;
        if model.language() != self.language
            || model.feature_profile() != self.feature_profile
            || model.normalization_profile() != self.normalization_profile
            || model.feature_schema() != self.feature_schema
        {
            return Err("artifact metadata mismatch".to_owned());
        }
        Ok(model)
    }
}

fn resolve_v2_rules(language: Language) -> Option<&'static LanguageRules> {
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
        include_bytes!("../resources/models/multilingual-v2/en-sparse-v2.bin"),
        digest("cb37986703724b067c82207c07e9208b8ebd6d13deb1537504baed0d1f2c0a98"),
        1,
        digest("83f12c208705486045927869c1adc40d5987064de60cc5665bb24e5ee20f1bd3"),
        Some(digest(
            "a734820a63c87994781d182692e6dc7ec262c402016971a7fa31946ced0d470c",
        )),
    ),
    RegistryEntry::new(
        Language::Zh,
        include_bytes!("../resources/models/multilingual-v2/zh-sparse-v2.bin"),
        digest("ca0098bc453def36abc9069995819a0cdda575d13b4456fafa4dc8bbc9ea9c05"),
        1,
        digest("6faedfcb637f60f23a58ff24e3473023a2484707774fe90277cb217c9b3d7941"),
        Some(digest(
            "e37f5ae1c799fc9f135d27e6965459df13d594385ff6358304b2fe9c51782dd3",
        )),
    ),
    RegistryEntry::new(
        Language::Es,
        include_bytes!("../resources/models/multilingual-v2/es-chargram-v1.bin"),
        digest("3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36"),
        1,
        digest("8bb5ad315f8abe69611cb192bfdf3712d8005cd331565547ec87573720a48246"),
        Some(digest(
            "5adadf7886ea332e6e07de1f5abb98a71a3dacbf3bea993b21100c9b4bffd4ba",
        )),
    ),
    RegistryEntry::new(
        Language::Ar,
        include_bytes!("../resources/models/multilingual-v2/ar-sparse-v2.bin"),
        digest("04a2de1e4c85b27f4c86dd5284ca19407421f5b9062dd4fefa7aeec3290d69fc"),
        2,
        digest("a882fa77392de6d327db51fc15a97729d63378df59b9ae564360e9b86aaff7ef"),
        Some(digest(
            "02bef4384f6e365a2f52d1ea993de218252a75caad2a08f030b9401e8e6facf4",
        )),
    ),
    RegistryEntry::new(
        Language::Ms,
        include_bytes!("../resources/models/multilingual-v2/id-sparse-v2.bin"),
        digest("9b207f1b85a9a8da31554258dd5ccb49509119dc8db715149c467b2df6116cf8"),
        1,
        digest("32b05ecb070b353590ed6b2f29e4d6a13023ec4b33cfa7f48679f16475182861"),
        Some(digest(
            "947d3fa9f7ffb1fc65aabf73278c9484fa1769c32349ba6ff23727825b69e9b8",
        )),
    ),
    RegistryEntry::new(
        Language::Pt,
        include_bytes!("../resources/models/multilingual-v2/pt-sparse-v2.bin"),
        digest("7543698f1daa28fb72c7a84c90d161f756d778d7f1165fb437ec5d296df6c33c"),
        1,
        digest("76b8c7927042582bee11d3c4444e1cf61c199782a223d1f95e60ea536c2a69aa"),
        Some(digest(
            "157ed297e7f0f9dcf5ab69e8ad7e231dad64473e516d168bda1c9582c372c16d",
        )),
    ),
    RegistryEntry::new(
        Language::Fr,
        include_bytes!("../resources/models/multilingual-v2/fr-sparse-v2.bin"),
        digest("1aa9bde78833bb330d7f4852373ab9566144563aa8f2d1b56389e3b701d62694"),
        1,
        digest("d944d890212aefb86324d2d6dd4518a724f0e6de6daefee2706fb7be0e3fabc6"),
        Some(digest(
            "8405124a1374b65d777cba39020f817e047598c6c8cf455fb9c106ee4cad4625",
        )),
    ),
    RegistryEntry::new(
        Language::Hi,
        include_bytes!("../resources/models/multilingual-v2/hi-sparse-v2.bin"),
        digest("84a9b478e45236509882c8b524cf1e08070c7f255804dceb7118ea55d0789547"),
        2,
        digest("eed30552d88d22ecc5cb33da64aaa4786299b78f046c3ff5fd0a1f744f6fb275"),
        Some(digest(
            "12678d085fbac5d15a52e574e82a9937f05cb119804af56657698655c8036f02",
        )),
    ),
    RegistryEntry::new(
        Language::Ru,
        include_bytes!("../resources/models/multilingual-v2/ru-sparse-v2.bin"),
        digest("3706ce98e530e9d6830c80195baad420c0fa0877990bcf8d21bedd618eab18a4"),
        1,
        digest("2d3f7288619d2e801eeb2a52ea7243207752c08ee49c54098c70fb294c059b01"),
        Some(digest(
            "c0f022f03c10a1097f96ef5d346dedf9899083a4d6bc2e6c96112c6444943384",
        )),
    ),
    RegistryEntry::new(
        Language::Ja,
        include_bytes!("../resources/models/multilingual-v2/ja-sparse-v2.bin"),
        digest("ea688b3ec5848a14cfca0ac634abe631f8bd759312d1439773e2fc178c32738f"),
        1,
        digest("8474720d2e8e0e85ba97e3a08f949dfa9b429b1538bff6a7b593bd32d6c9b2a1"),
        Some(digest(
            "98a2dd994ccba6bce296668d6d17914cb397da1848718d530394648dcab7aac1",
        )),
    ),
    RegistryEntry::new(
        Language::De,
        include_bytes!("../resources/models/multilingual-v2/de-sparse-v2.bin"),
        digest("9f8e110db887b1e452a75d067d64796a1bba4575ba5076c58d17af2c63bd1649"),
        1,
        digest("b48d46b1d9a84a9cf4781cd32d16f08acbeef426811919bdb6f870d52b837f96"),
        Some(digest(
            "5eb7f5e38ae33c182f99be27c72af3a5ab8b9620dc33a6efbde0c1e34c539aa1",
        )),
    ),
    RegistryEntry::new(
        Language::Tr,
        include_bytes!("../resources/models/multilingual-v2/tr-sparse-v2.bin"),
        digest("258fb502126105c02e1ce97998e7c95d3fdc8f6dfc44cb2ab384a8a3448c54a3"),
        1,
        digest("1ead20c976e68da4fdb683cdebbe680e40d7561bed08b807f595925fe663a932"),
        Some(digest(
            "518abd34e82fe58aa25bb7c24e35b8e17eac814b2a23766490cf32e3c1c32346",
        )),
    ),
    RegistryEntry::new(
        Language::Vi,
        include_bytes!("../resources/models/multilingual-v2/vi-sparse-v2.bin"),
        digest("f289b02c024075825706be7e3e0ecdfb60e73ecc2dc9604c74638a832f13b2ab"),
        1,
        digest("13fcb5ae0b82f081bd942acd347469290b369e39a159287a646ebcb4a835ff64"),
        Some(digest(
            "633a1f116b8ec2462eacec56f9a498d4387b4df10ef7e1c8ef47a076f6bf6914",
        )),
    ),
    RegistryEntry::new(
        Language::Ko,
        include_bytes!("../resources/models/multilingual-v2/ko-sparse-v2.bin"),
        digest("41627c430b64247febed8ca16c5c4297df23fbb1d4e30913f301be5775ab7fd3"),
        1,
        digest("51f02655fed2ffe57b000f4f557cf88ff03ee15fd5cb1e9b356b0fffae879a35"),
        Some(digest(
            "64fec4779ec7808ba08912a7a4fb022e6821ed4f701c0da64a1e360db528c47e",
        )),
    ),
    RegistryEntry::new(
        Language::It,
        include_bytes!("../resources/models/multilingual-v2/it-sparse-v2.bin"),
        digest("49b2e3fc9a4aea801d70bc84f96b90db13bc66ec8149dd4907f6437ce7bcfb85"),
        2,
        digest("c1e8cf9f3612964383114fe3108d5ae08c2d98c3662dceb24f56219c9dc8129e"),
        Some(digest(
            "043fb72213c2158b9c2b75651e7193bad249b8dc577c1f1e6c71a2b69d415eef",
        )),
    ),
];

pub fn language_spec(language: Language) -> &'static LanguageSpec {
    &LANGUAGE_SPECS[language.index()]
}

pub(crate) fn registry_entry(language: Language) -> &'static RegistryEntry {
    &REGISTRY_ENTRIES[language.index()]
}

const fn digest(value: &str) -> [u8; 32] {
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
    fn registry_embeds_exactly_one_valid_model_per_language() {
        assert_eq!(REGISTRY_ENTRIES.len(), Language::ALL.len());

        for (expected, entry) in Language::ALL.iter().copied().zip(&REGISTRY_ENTRIES) {
            assert_eq!(entry.language, expected);
            assert_eq!(registry_entry(expected).language, expected);

            let model = entry.model().expect("embedded model");
            assert_eq!(model.language(), expected);
            assert_eq!(model.feature_profile(), entry.feature_profile);
            assert_eq!(model.normalization_profile(), entry.normalization_profile);
            assert_eq!(model.feature_schema(), entry.feature_schema);
            entry.validate_rule_identity().expect("valid rule identity");
        }
    }

    #[test]
    fn registry_rejects_an_artifact_digest_mismatch() {
        let entry = entry_for(
            Language::En,
            Language::En,
            [0; 32],
            1,
            rule_digest(Language::En),
        );

        assert_eq!(
            entry.parse_model().expect_err("changed artifact"),
            "artifact digest mismatch"
        );
    }

    #[test]
    fn registry_rejects_artifact_metadata_for_another_language() {
        let artifact = english_artifact();
        let entry = entry_for(
            Language::Fr,
            Language::En,
            Sha256::digest(artifact).into(),
            1,
            rule_digest(Language::Fr),
        );

        assert_eq!(
            entry.parse_model().expect_err("wrong metadata"),
            "artifact metadata mismatch"
        );
    }

    #[test]
    fn registry_rejects_a_rule_pack_version_mismatch() {
        let artifact = english_artifact();
        let entry = entry_for(
            Language::En,
            Language::En,
            Sha256::digest(artifact).into(),
            99,
            rule_digest(Language::En),
        );

        let error = entry
            .validate_rule_identity()
            .expect_err("wrong rule version");
        assert!(error.to_string().contains("version mismatch"));
    }

    #[test]
    fn registry_rejects_a_rule_pack_digest_mismatch() {
        let artifact = english_artifact();
        let entry = entry_for(
            Language::En,
            Language::En,
            Sha256::digest(artifact).into(),
            1,
            [0; 32],
        );

        let error = entry
            .validate_rule_identity()
            .expect_err("wrong rule digest");
        assert!(error.to_string().contains("digest mismatch"));
    }

    fn english_artifact() -> &'static [u8] {
        include_bytes!("../resources/models/multilingual-v2/en-sparse-v2.bin")
    }

    fn rule_digest(language: Language) -> [u8; 32] {
        Sha256::digest(canonical_rule_identity(language)).into()
    }

    fn entry_for(
        declared_language: Language,
        profile_language: Language,
        artifact_sha256: [u8; 32],
        rule_pack_version: u16,
        rule_pack_sha256: [u8; 32],
    ) -> RegistryEntry {
        let profile = profile_language.profiles();
        let mut entry = RegistryEntry::new(
            declared_language,
            english_artifact(),
            artifact_sha256,
            rule_pack_version,
            rule_pack_sha256,
            None,
        );
        entry.feature_profile = profile.0;
        entry.normalization_profile = profile.1;
        entry.feature_schema = profile.2;
        entry
    }
}
