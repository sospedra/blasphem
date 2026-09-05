//! Sparse artifacts and clean-room lexica compiled into the binary.
//!
//! Present only with the `embedded` feature. The lexica are compiled from
//! multiple permissively licensed sources; see
//! `crates/blasphem-train/metadata/source-lock-v1.json` for the source-by-source licenses.

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
    lexicon_sha256: Option<[u8; 32]>,
}

impl EmbeddedArtifact {
    const fn new(
        artifact: &'static [u8],
        artifact_sha256: [u8; 32],
        lexicon_sha256: Option<[u8; 32]>,
    ) -> Self {
        Self {
            artifact,
            artifact_sha256,
            lexicon_sha256,
        }
    }
}

static ARTIFACTS: [Option<EmbeddedArtifact>; 15] = [
    #[cfg(feature = "en")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/en-sparse.bin"),
        digest("7a124d7016eaf218967a39c18fe2a4ecaad1454e2965c4a30f165f6e47f9271e"),
        Some(digest(
            "47e158ffb912b3f6fc6ada64bf6c1deeb05fe45d9d9e552c942a6cf87479b871",
        )),
    )),
    #[cfg(not(feature = "en"))]
    None,
    #[cfg(feature = "zh")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/zh-sparse.bin"),
        digest("605d1a49534b84e498e8978a1e61eda1ba489c0de4ac9904cce4c571a1e9060f"),
        Some(digest(
            "8f789c2eb2b8dd62a13ff91aafea6c7b84559bec43599be3150917ff5d920923",
        )),
    )),
    #[cfg(not(feature = "zh"))]
    None,
    #[cfg(feature = "es")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/es-sparse.bin"),
        digest("05f8cb6f13fdb0abfd2d161d5cf6a454cafb068eee2014440f2de6dac5c92772"),
        Some(digest(
            "7ac642a30c91308b8fd2bfcf75c827238999b776aae502dddf8c3dbb20cde7cc",
        )),
    )),
    #[cfg(not(feature = "es"))]
    None,
    #[cfg(feature = "ar")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/ar-sparse.bin"),
        digest("e49b28679917bc04a25a88bce4d7f14d75a4b825ee34564021e36567b62013c4"),
        Some(digest(
            "9e8967963e86214257a19f2cf8d914e8f442be4d8317097924e8b7df0b6deeaa",
        )),
    )),
    #[cfg(not(feature = "ar"))]
    None,
    #[cfg(feature = "ms")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/id-sparse.bin"),
        digest("050f03a790319df94421f0cced0ca2387f30ad7c6e530ddf22e704cc305730c9"),
        Some(digest(
            "d0c159269fa1fa00cae3c409f45608259a23687ccc384ca9eef0f50ecae5fe82",
        )),
    )),
    #[cfg(not(feature = "ms"))]
    None,
    #[cfg(feature = "pt")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/pt-sparse.bin"),
        digest("19e8186f6e9d3c9246a9b7002986d073f19b69f688d03317114506211bfa5adb"),
        Some(digest(
            "5fbd7342e88d89418ba5a993ce0578f3e5a84a6ce65f4a2deda89302d53db556",
        )),
    )),
    #[cfg(not(feature = "pt"))]
    None,
    #[cfg(feature = "fr")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/fr-sparse.bin"),
        digest("522d4c661000d6424807b53107ca384bb92f714db40c0243f0d5c0336dbac67a"),
        Some(digest(
            "b191619a61722f2fee05531c086a802f336c000cb5767ea252d333ae282b792c",
        )),
    )),
    #[cfg(not(feature = "fr"))]
    None,
    #[cfg(feature = "hi")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/hi-sparse.bin"),
        digest("55da5e7035bc7c307d1c3d45f3aa7c4f434040ce7f8f6119b2ac54549d6c2bee"),
        Some(digest(
            "3fc8e4c387b96a3634922d10322c51fa8dd2a15666f8caea6df65982395cf647",
        )),
    )),
    #[cfg(not(feature = "hi"))]
    None,
    #[cfg(feature = "ru")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/ru-sparse.bin"),
        digest("62441730f8bed25ee1dae97a0a511b57a2192983556361ce75d211d3d9e9e6c7"),
        Some(digest(
            "2c196c7b84b99e43ec8daacd064de7264726e5eee03bf166ce2821927d0e4d8a",
        )),
    )),
    #[cfg(not(feature = "ru"))]
    None,
    #[cfg(feature = "ja")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/ja-sparse.bin"),
        digest("ea688b3ec5848a14cfca0ac634abe631f8bd759312d1439773e2fc178c32738f"),
        Some(digest(
            "fd709e7d92472193bc30f8a18db179cf8ad8e5e4d8922604bfd605c287b7dd50",
        )),
    )),
    #[cfg(not(feature = "ja"))]
    None,
    #[cfg(feature = "de")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/de-sparse.bin"),
        digest("d6fe44ed8708c8f31789ac3b8e5de4804883495b067e10bf807150743884eda0"),
        Some(digest(
            "c03d37a2fea09c5bacdf19307387aca0c720c46ab6bb9280870e93a08da367a8",
        )),
    )),
    #[cfg(not(feature = "de"))]
    None,
    #[cfg(feature = "tr")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/tr-sparse.bin"),
        digest("31e71f13e970c0a456bae84ec0cd7a4c20fc6a358c2662b0a17695f0de2de70a"),
        Some(digest(
            "5398b7ae04964b79d9b69376d9aba85120a76b086942b3b2fafc6f17af8ff886",
        )),
    )),
    #[cfg(not(feature = "tr"))]
    None,
    #[cfg(feature = "vi")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/vi-sparse.bin"),
        digest("136379cf21528eccfcb2eabfda106da9fae629e6f7158685d3f421a35aa7b584"),
        Some(digest(
            "b03dc19155e27ad8327b79a25337ed6843d0f8c366d9eaff794b818a5c46dd6e",
        )),
    )),
    #[cfg(not(feature = "vi"))]
    None,
    #[cfg(feature = "ko")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/ko-sparse.bin"),
        digest("52a5917d5202113cf9bf5dc2c969fc4b91d0cb0357e1d021c4c106e2d63e255c"),
        Some(digest(
            "1fe1886792cc809f4685aa5de19872fa5b40b183e74e2791caca217f7984b824",
        )),
    )),
    #[cfg(not(feature = "ko"))]
    None,
    #[cfg(feature = "it")]
    Some(EmbeddedArtifact::new(
        include_bytes!("../../../resources/models/it-sparse.bin"),
        digest("899c57509ea75117db8c19a500cb5b6b883d07b0eab9e9f8e8fd9a3565f6140f"),
        Some(digest(
            "1904d7807553a0263c0472f17f983edba5bc2ebe7af69aa0e2ed95fa980879b8",
        )),
    )),
    #[cfg(not(feature = "it"))]
    None,
];

static MODELS: [OnceLock<Result<SparseModel, String>>; 15] = [const { OnceLock::new() }; 15];

/// The pinned clean-room lexicon rows for one language.
#[must_use]
#[allow(unreachable_patterns)]
pub const fn embedded_lexicon_bytes(language: Language) -> &'static [u8] {
    match language {
        #[cfg(feature = "en")]
        Language::En => include_bytes!("../../../resources/lexicon/EN.tsv"),
        #[cfg(feature = "zh")]
        Language::Zh => include_bytes!("../../../resources/lexicon/ZH.tsv"),
        #[cfg(feature = "es")]
        Language::Es => include_bytes!("../../../resources/lexicon/ES.tsv"),
        #[cfg(feature = "ar")]
        Language::Ar => include_bytes!("../../../resources/lexicon/AR.tsv"),
        #[cfg(feature = "ms")]
        Language::Ms => include_bytes!("../../../resources/lexicon/ID.tsv"),
        #[cfg(feature = "pt")]
        Language::Pt => include_bytes!("../../../resources/lexicon/PT.tsv"),
        #[cfg(feature = "fr")]
        Language::Fr => include_bytes!("../../../resources/lexicon/FR.tsv"),
        #[cfg(feature = "hi")]
        Language::Hi => include_bytes!("../../../resources/lexicon/HI.tsv"),
        #[cfg(feature = "ru")]
        Language::Ru => include_bytes!("../../../resources/lexicon/RU.tsv"),
        #[cfg(feature = "ja")]
        Language::Ja => include_bytes!("../../../resources/lexicon/JA.tsv"),
        #[cfg(feature = "de")]
        Language::De => include_bytes!("../../../resources/lexicon/DE.tsv"),
        #[cfg(feature = "tr")]
        Language::Tr => include_bytes!("../../../resources/lexicon/TR.tsv"),
        #[cfg(feature = "vi")]
        Language::Vi => include_bytes!("../../../resources/lexicon/VI.tsv"),
        #[cfg(feature = "ko")]
        Language::Ko => include_bytes!("../../../resources/lexicon/KO.tsv"),
        #[cfg(feature = "it")]
        Language::It => include_bytes!("../../../resources/lexicon/IT.tsv"),
        _ => &[],
    }
}

/// The digest the embedded Lexicon rows for one language must match.
pub(crate) fn embedded_lexicon_sha256(language: Language) -> Option<[u8; 32]> {
    ARTIFACTS[language.index()]
        .as_ref()
        .and_then(|entry| entry.lexicon_sha256)
}

/// The validated, compiled-in sparse model for one language.
pub(crate) fn embedded_model(language: Language) -> Result<&'static SparseModel, RuntimeInitError> {
    let artifact = ARTIFACTS[language.index()].as_ref().ok_or_else(|| {
        RuntimeInitError::InvalidEmbeddedModel {
            language,
            reason: "locale is not compiled into this build".to_owned(),
        }
    })?;
    MODELS[language.index()]
        .get_or_init(|| parse_artifact(language, artifact))
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
    NudgeDetector::from_lexicon_bytes(language, Some(embedded_lexicon_bytes(language)))
}

pub(crate) fn embedded_model_ref(
    language: Language,
) -> Result<Cow<'static, SparseModel>, RuntimeInitError> {
    embedded_model(language).map(Cow::Borrowed)
}

/// Returns the languages compiled into this build, in registry order.
#[must_use]
pub fn compiled_locales() -> Vec<Language> {
    Language::ALL
        .into_iter()
        .filter(|language| ARTIFACTS[language.index()].is_some())
        .collect()
}

/// Returns the compiled detection slice for one language.
#[cfg(feature = "language-detection")]
#[allow(unreachable_patterns)]
pub fn embedded_detect_bytes(language: Language) -> &'static [u8] {
    match language {
        #[cfg(feature = "en")]
        Language::En => include_bytes!("../../../resources/packs/en.detect"),
        #[cfg(feature = "zh")]
        Language::Zh => include_bytes!("../../../resources/packs/zh.detect"),
        #[cfg(feature = "es")]
        Language::Es => include_bytes!("../../../resources/packs/es.detect"),
        #[cfg(feature = "ar")]
        Language::Ar => include_bytes!("../../../resources/packs/ar.detect"),
        #[cfg(feature = "ms")]
        Language::Ms => include_bytes!("../../../resources/packs/ms.detect"),
        #[cfg(feature = "pt")]
        Language::Pt => include_bytes!("../../../resources/packs/pt.detect"),
        #[cfg(feature = "fr")]
        Language::Fr => include_bytes!("../../../resources/packs/fr.detect"),
        #[cfg(feature = "hi")]
        Language::Hi => include_bytes!("../../../resources/packs/hi.detect"),
        #[cfg(feature = "ru")]
        Language::Ru => include_bytes!("../../../resources/packs/ru.detect"),
        #[cfg(feature = "ja")]
        Language::Ja => include_bytes!("../../../resources/packs/ja.detect"),
        #[cfg(feature = "de")]
        Language::De => include_bytes!("../../../resources/packs/de.detect"),
        #[cfg(feature = "tr")]
        Language::Tr => include_bytes!("../../../resources/packs/tr.detect"),
        #[cfg(feature = "vi")]
        Language::Vi => include_bytes!("../../../resources/packs/vi.detect"),
        #[cfg(feature = "ko")]
        Language::Ko => include_bytes!("../../../resources/packs/ko.detect"),
        #[cfg(feature = "it")]
        Language::It => include_bytes!("../../../resources/packs/it.detect"),
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::{compiled_locales, embedded_model};

    #[test]
    fn every_embedded_artifact_parses_and_matches_its_registry_entry() {
        for language in compiled_locales() {
            let model = embedded_model(language).expect("embedded model");
            assert_eq!(model.language(), language);
        }
    }

    #[test]
    #[cfg(feature = "en")]
    fn embedded_rejects_an_artifact_digest_mismatch() {
        let entry = super::EmbeddedArtifact::new(
            super::ARTIFACTS[0]
                .as_ref()
                .expect("English feature")
                .artifact,
            [0; 32],
            None,
        );

        assert_eq!(
            super::parse_artifact(crate::Language::En, &entry).expect_err("changed artifact"),
            "artifact digest mismatch"
        );
    }

    #[test]
    #[cfg(feature = "en")]
    fn embedded_rejects_artifact_metadata_for_another_language() {
        use sha2::{Digest, Sha256};

        let artifact = super::ARTIFACTS[0]
            .as_ref()
            .expect("English feature")
            .artifact;
        let entry = super::EmbeddedArtifact::new(artifact, Sha256::digest(artifact).into(), None);

        assert_eq!(
            super::parse_artifact(crate::Language::Fr, &entry).expect_err("wrong metadata"),
            "artifact metadata mismatch"
        );
    }
}
