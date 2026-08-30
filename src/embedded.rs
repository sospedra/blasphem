//! Sparse artifacts and HurtLex lexica compiled into the binary.
//!
//! Present only with the `embedded` feature. The lexica are CC-BY-SA-4.0.
//! See NOTICE for the attribution this obligation requires.

use std::borrow::Cow;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::language::Language;
use crate::registry::{digest, registry_entry};
use crate::runtime::{NudgeDetector, RuntimeInitError};
use crate::sparse::SparseModel;

struct EmbeddedArtifact {
    artifact: &'static [u8],
    artifact_sha256: [u8; 32],
    hurtlex_sha256: Option<[u8; 32]>,
}

impl EmbeddedArtifact {
    const fn new(
        artifact: &'static [u8],
        artifact_sha256: [u8; 32],
        hurtlex_sha256: Option<[u8; 32]>,
    ) -> Self {
        Self {
            artifact,
            artifact_sha256,
            hurtlex_sha256,
        }
    }
}

static ARTIFACTS: [EmbeddedArtifact; 15] = [
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/en-sparse-v2.bin"),
        digest("cb37986703724b067c82207c07e9208b8ebd6d13deb1537504baed0d1f2c0a98"),
        Some(digest(
            "a734820a63c87994781d182692e6dc7ec262c402016971a7fa31946ced0d470c",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/zh-sparse-v2.bin"),
        digest("ca0098bc453def36abc9069995819a0cdda575d13b4456fafa4dc8bbc9ea9c05"),
        Some(digest(
            "e37f5ae1c799fc9f135d27e6965459df13d594385ff6358304b2fe9c51782dd3",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/es-chargram-v1.bin"),
        digest("e06d2039147fa4a5c88451bef6d62d614d3659adcf7ae575cc8ab1cb2cfe59f6"),
        Some(digest(
            "5adadf7886ea332e6e07de1f5abb98a71a3dacbf3bea993b21100c9b4bffd4ba",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ar-sparse-v2.bin"),
        digest("04a2de1e4c85b27f4c86dd5284ca19407421f5b9062dd4fefa7aeec3290d69fc"),
        Some(digest(
            "02bef4384f6e365a2f52d1ea993de218252a75caad2a08f030b9401e8e6facf4",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/id-sparse-v2.bin"),
        digest("9b207f1b85a9a8da31554258dd5ccb49509119dc8db715149c467b2df6116cf8"),
        Some(digest(
            "947d3fa9f7ffb1fc65aabf73278c9484fa1769c32349ba6ff23727825b69e9b8",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/pt-sparse-v2.bin"),
        digest("7543698f1daa28fb72c7a84c90d161f756d778d7f1165fb437ec5d296df6c33c"),
        Some(digest(
            "157ed297e7f0f9dcf5ab69e8ad7e231dad64473e516d168bda1c9582c372c16d",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/fr-sparse-v2.bin"),
        digest("1aa9bde78833bb330d7f4852373ab9566144563aa8f2d1b56389e3b701d62694"),
        Some(digest(
            "8405124a1374b65d777cba39020f817e047598c6c8cf455fb9c106ee4cad4625",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/hi-sparse-v2.bin"),
        digest("84a9b478e45236509882c8b524cf1e08070c7f255804dceb7118ea55d0789547"),
        Some(digest(
            "12678d085fbac5d15a52e574e82a9937f05cb119804af56657698655c8036f02",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ru-sparse-v2.bin"),
        digest("3706ce98e530e9d6830c80195baad420c0fa0877990bcf8d21bedd618eab18a4"),
        Some(digest(
            "c0f022f03c10a1097f96ef5d346dedf9899083a4d6bc2e6c96112c6444943384",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ja-sparse-v2.bin"),
        digest("ea688b3ec5848a14cfca0ac634abe631f8bd759312d1439773e2fc178c32738f"),
        Some(digest(
            "98a2dd994ccba6bce296668d6d17914cb397da1848718d530394648dcab7aac1",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/de-sparse-v2.bin"),
        digest("9f8e110db887b1e452a75d067d64796a1bba4575ba5076c58d17af2c63bd1649"),
        Some(digest(
            "5eb7f5e38ae33c182f99be27c72af3a5ab8b9620dc33a6efbde0c1e34c539aa1",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/tr-sparse-v2.bin"),
        digest("258fb502126105c02e1ce97998e7c95d3fdc8f6dfc44cb2ab384a8a3448c54a3"),
        Some(digest(
            "518abd34e82fe58aa25bb7c24e35b8e17eac814b2a23766490cf32e3c1c32346",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/vi-sparse-v2.bin"),
        digest("f289b02c024075825706be7e3e0ecdfb60e73ecc2dc9604c74638a832f13b2ab"),
        Some(digest(
            "633a1f116b8ec2462eacec56f9a498d4387b4df10ef7e1c8ef47a076f6bf6914",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ko-sparse-v2.bin"),
        digest("41627c430b64247febed8ca16c5c4297df23fbb1d4e30913f301be5775ab7fd3"),
        Some(digest(
            "64fec4779ec7808ba08912a7a4fb022e6821ed4f701c0da64a1e360db528c47e",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/it-sparse-v2.bin"),
        digest("49b2e3fc9a4aea801d70bc84f96b90db13bc66ec8149dd4907f6437ce7bcfb85"),
        Some(digest(
            "043fb72213c2158b9c2b75651e7193bad249b8dc577c1f1e6c71a2b69d415eef",
        )),
    ),
];

static MODELS: [OnceLock<Result<SparseModel, String>>; 15] = [const { OnceLock::new() }; 15];

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

/// The digest the embedded HurtLex rows for one language must match.
pub(crate) fn embedded_hurtlex_sha256(language: Language) -> Option<[u8; 32]> {
    ARTIFACTS[language.index()].hurtlex_sha256
}

/// The validated, compiled-in sparse model for one language.
pub(crate) fn embedded_model(language: Language) -> Result<&'static SparseModel, RuntimeInitError> {
    MODELS[language.index()]
        .get_or_init(|| parse_artifact(language, &ARTIFACTS[language.index()]))
        .as_ref()
        .map_err(|reason| RuntimeInitError::InvalidEmbeddedModel {
            language,
            reason: reason.clone(),
        })
}

fn parse_artifact(language: Language, embedded: &EmbeddedArtifact) -> Result<SparseModel, String> {
    let actual: [u8; 32] = Sha256::digest(embedded.artifact).into();
    if actual != embedded.artifact_sha256 {
        return Err("artifact digest mismatch".to_owned());
    }
    let model = SparseModel::from_bytes(embedded.artifact).map_err(|error| error.to_string())?;
    registry_entry(language).check_model(&model)?;
    Ok(model)
}

/// Builds a detector from the compiled-in lexicon for one language.
///
/// # Errors
///
/// Returns an error when the embedded resources are missing or invalid.
pub fn embedded_detector(language: Language) -> Result<NudgeDetector, RuntimeInitError> {
    NudgeDetector::from_hurtlex_bytes(language, Some(embedded_hurtlex_bytes(language)))
}

pub(crate) fn embedded_model_ref(
    language: Language,
) -> Result<Cow<'static, SparseModel>, RuntimeInitError> {
    embedded_model(language).map(Cow::Borrowed)
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{ARTIFACTS, EmbeddedArtifact, embedded_model, parse_artifact};
    use crate::Language;

    #[test]
    fn every_embedded_artifact_parses_and_matches_its_registry_entry() {
        for language in Language::ALL {
            let model = embedded_model(language).expect("embedded model");
            assert_eq!(model.language(), language);
        }
    }

    #[test]
    fn embedded_rejects_an_artifact_digest_mismatch() {
        let entry = EmbeddedArtifact::new(ARTIFACTS[0].artifact, [0; 32], None);

        assert_eq!(
            parse_artifact(Language::En, &entry).expect_err("changed artifact"),
            "artifact digest mismatch"
        );
    }

    #[test]
    fn embedded_rejects_artifact_metadata_for_another_language() {
        let artifact = ARTIFACTS[0].artifact;
        let entry = EmbeddedArtifact::new(artifact, Sha256::digest(artifact).into(), None);

        assert_eq!(
            parse_artifact(Language::Fr, &entry).expect_err("wrong metadata"),
            "artifact metadata mismatch"
        );
    }
}
