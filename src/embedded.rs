//! Sparse artifacts and clean-room lexica compiled into the binary.
//!
//! Present only with the `embedded` feature. The lexica are compiled from
//! multiple permissively licensed sources; see
//! `resources/datasets/source-lock-v1.json` for the source-by-source licenses.

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
            "47e158ffb912b3f6fc6ada64bf6c1deeb05fe45d9d9e552c942a6cf87479b871",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/zh-sparse-v2.bin"),
        digest("ca0098bc453def36abc9069995819a0cdda575d13b4456fafa4dc8bbc9ea9c05"),
        Some(digest(
            "8f789c2eb2b8dd62a13ff91aafea6c7b84559bec43599be3150917ff5d920923",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/es-chargram-v1.bin"),
        digest("dda54a36d0b3230edbfe37feff80488fdf3272b92341d342d5711bf4e5881250"),
        Some(digest(
            "7ac642a30c91308b8fd2bfcf75c827238999b776aae502dddf8c3dbb20cde7cc",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ar-sparse-v2.bin"),
        digest("04a2de1e4c85b27f4c86dd5284ca19407421f5b9062dd4fefa7aeec3290d69fc"),
        Some(digest(
            "9e8967963e86214257a19f2cf8d914e8f442be4d8317097924e8b7df0b6deeaa",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/id-sparse-v2.bin"),
        digest("9b207f1b85a9a8da31554258dd5ccb49509119dc8db715149c467b2df6116cf8"),
        Some(digest(
            "d0c159269fa1fa00cae3c409f45608259a23687ccc384ca9eef0f50ecae5fe82",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/pt-sparse-v2.bin"),
        digest("7543698f1daa28fb72c7a84c90d161f756d778d7f1165fb437ec5d296df6c33c"),
        Some(digest(
            "5fbd7342e88d89418ba5a993ce0578f3e5a84a6ce65f4a2deda89302d53db556",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/fr-sparse-v2.bin"),
        digest("1aa9bde78833bb330d7f4852373ab9566144563aa8f2d1b56389e3b701d62694"),
        Some(digest(
            "b191619a61722f2fee05531c086a802f336c000cb5767ea252d333ae282b792c",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/hi-sparse-v2.bin"),
        digest("84a9b478e45236509882c8b524cf1e08070c7f255804dceb7118ea55d0789547"),
        Some(digest(
            "3fc8e4c387b96a3634922d10322c51fa8dd2a15666f8caea6df65982395cf647",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ru-sparse-v2.bin"),
        digest("3706ce98e530e9d6830c80195baad420c0fa0877990bcf8d21bedd618eab18a4"),
        Some(digest(
            "2c196c7b84b99e43ec8daacd064de7264726e5eee03bf166ce2821927d0e4d8a",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ja-sparse-v2.bin"),
        digest("ea688b3ec5848a14cfca0ac634abe631f8bd759312d1439773e2fc178c32738f"),
        Some(digest(
            "fd709e7d92472193bc30f8a18db179cf8ad8e5e4d8922604bfd605c287b7dd50",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/de-sparse-v2.bin"),
        digest("9f8e110db887b1e452a75d067d64796a1bba4575ba5076c58d17af2c63bd1649"),
        Some(digest(
            "c03d37a2fea09c5bacdf19307387aca0c720c46ab6bb9280870e93a08da367a8",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/tr-sparse-v2.bin"),
        digest("258fb502126105c02e1ce97998e7c95d3fdc8f6dfc44cb2ab384a8a3448c54a3"),
        Some(digest(
            "5398b7ae04964b79d9b69376d9aba85120a76b086942b3b2fafc6f17af8ff886",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/vi-sparse-v2.bin"),
        digest("f289b02c024075825706be7e3e0ecdfb60e73ecc2dc9604c74638a832f13b2ab"),
        Some(digest(
            "b03dc19155e27ad8327b79a25337ed6843d0f8c366d9eaff794b818a5c46dd6e",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/ko-sparse-v2.bin"),
        digest("41627c430b64247febed8ca16c5c4297df23fbb1d4e30913f301be5775ab7fd3"),
        Some(digest(
            "1fe1886792cc809f4685aa5de19872fa5b40b183e74e2791caca217f7984b824",
        )),
    ),
    EmbeddedArtifact::new(
        include_bytes!("../resources/models/multilingual-v2/it-sparse-v2.bin"),
        digest("49b2e3fc9a4aea801d70bc84f96b90db13bc66ec8149dd4907f6437ce7bcfb85"),
        Some(digest(
            "1904d7807553a0263c0472f17f983edba5bc2ebe7af69aa0e2ed95fa980879b8",
        )),
    ),
];

static MODELS: [OnceLock<Result<SparseModel, String>>; 15] = [const { OnceLock::new() }; 15];

/// The pinned clean-room lexicon rows for one language.
#[must_use]
pub const fn embedded_hurtlex_bytes(language: Language) -> &'static [u8] {
    match language {
        Language::En => include_bytes!("../data/clean-room-v1/EN.tsv"),
        Language::Zh => include_bytes!("../data/clean-room-v1/ZH.tsv"),
        Language::Es => include_bytes!("../data/clean-room-v1/ES.tsv"),
        Language::Ar => include_bytes!("../data/clean-room-v1/AR.tsv"),
        Language::Ms => include_bytes!("../data/clean-room-v1/ID.tsv"),
        Language::Pt => include_bytes!("../data/clean-room-v1/PT.tsv"),
        Language::Fr => include_bytes!("../data/clean-room-v1/FR.tsv"),
        Language::Hi => include_bytes!("../data/clean-room-v1/HI.tsv"),
        Language::Ru => include_bytes!("../data/clean-room-v1/RU.tsv"),
        Language::Ja => include_bytes!("../data/clean-room-v1/JA.tsv"),
        Language::De => include_bytes!("../data/clean-room-v1/DE.tsv"),
        Language::Tr => include_bytes!("../data/clean-room-v1/TR.tsv"),
        Language::Vi => include_bytes!("../data/clean-room-v1/VI.tsv"),
        Language::Ko => include_bytes!("../data/clean-room-v1/KO.tsv"),
        Language::It => include_bytes!("../data/clean-room-v1/IT.tsv"),
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
