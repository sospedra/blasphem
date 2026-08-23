use std::sync::OnceLock;

use thiserror::Error;

use crate::{FeatureProfile, FeatureSchema, Language, NormalizationProfile, extract_feature_bins};

const V1_MAGIC: &[u8; 8] = b"TOXSPRS1";
const V2_MAGIC: &[u8; 8] = b"TOXSPRS2";
const V1_FORMAT_VERSION: u16 = 1;
const V2_FORMAT_VERSION: u16 = 2;
const V1_HEADER_LENGTH: usize = 32;
const V2_HEADER_LENGTH: usize = 40;
const BIN_COUNT: usize = 65_536;
const PAYLOAD_LENGTH: usize = BIN_COUNT * size_of::<i16>();
const WEIGHT_SCALE: u16 = 256;
const SPANISH_ARTIFACT: &[u8] = include_bytes!("../resources/models/es-chargram-v1.bin");
static SPANISH_MODEL: OnceLock<SparseModel> = OnceLock::new();

/// A validated fixed-table text scorer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparseModel {
    language: Language,
    feature_profile: FeatureProfile,
    normalization_profile: NormalizationProfile,
    feature_schema: FeatureSchema,
    weights: Box<[i16]>,
    bias: i32,
    decision_boundary: i32,
    score_scale: u32,
    max_false_warning_basis_points: u16,
}

/// The complete input for one version-two sparse artifact.
pub struct SparseV2Input<'a> {
    pub language: Language,
    pub feature_profile: FeatureProfile,
    pub normalization_profile: NormalizationProfile,
    pub feature_schema: FeatureSchema,
    pub bias: i32,
    pub decision_boundary: i32,
    pub score_scale: u32,
    pub max_false_warning_basis_points: u16,
    pub weights: &'a [i16],
}

/// Errors from a compiled sparse artifact.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SparseModelError {
    #[error("invalid sparse artifact length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("invalid sparse artifact magic")]
    InvalidMagic,
    #[error("unsupported sparse artifact version: {0}")]
    UnsupportedVersion(u16),
    #[error("invalid sparse artifact language")]
    InvalidLanguage,
    #[error("invalid sparse artifact bin count: {0}")]
    InvalidBinCount(u32),
    #[error("invalid sparse artifact weight scale: {0}")]
    InvalidWeightScale(u16),
    #[error("the sparse artifact score scale is zero")]
    ZeroScoreScale,
    #[error("invalid feature profile: {0}")]
    InvalidFeatureProfile(u8),
    #[error("invalid normalization profile: {0}")]
    InvalidNormalizationProfile(u8),
    #[error("invalid feature schema: {0}")]
    InvalidFeatureSchema(u16),
    #[error("the artifact profiles do not match its language")]
    ProfileMismatch,
    #[error("invalid sparse payload length: {0}")]
    InvalidPayloadLength(u32),
    #[error("a version-two artifact cannot declare Spanish")]
    VersionTwoSpanish,
    #[error("invalid false-warning limit: {0} basis points")]
    InvalidFalseWarningLimit(u16),
}

impl SparseModel {
    /// Parses one compiled sparse artifact.
    ///
    /// # Errors
    ///
    /// Returns an error when the header or table is invalid.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SparseModelError> {
        let magic = bytes
            .get(..V1_MAGIC.len())
            .ok_or(SparseModelError::InvalidLength {
                expected: V1_MAGIC.len(),
                actual: bytes.len(),
            })?;
        if magic == V1_MAGIC {
            return parse_v1(bytes);
        }
        if magic == V2_MAGIC {
            return parse_v2(bytes);
        }
        Err(SparseModelError::InvalidMagic)
    }

    /// Returns the two-letter language code stored in the artifact.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    #[must_use]
    pub const fn feature_profile(&self) -> FeatureProfile {
        self.feature_profile
    }

    #[must_use]
    pub const fn normalization_profile(&self) -> NormalizationProfile {
        self.normalization_profile
    }

    #[must_use]
    pub const fn feature_schema(&self) -> FeatureSchema {
        self.feature_schema
    }

    #[must_use]
    pub const fn score_scale(&self) -> u32 {
        self.score_scale
    }

    #[must_use]
    pub const fn max_false_warning_basis_points(&self) -> u16 {
        self.max_false_warning_basis_points
    }

    /// Returns an ordinal score from 0 through 100.
    #[must_use]
    pub fn score(&self, text: &str) -> u8 {
        score_from_raw(
            self.raw_score(text),
            self.decision_boundary,
            self.score_scale,
        )
    }

    pub fn raw_score(&self, text: &str) -> i32 {
        let bins = extract_feature_bins(self.feature_profile, self.normalization_profile, text)
            .expect("validated artifact profile");
        bins.into_iter()
            .fold(i64::from(self.bias), |sum, bin| {
                sum + i64::from(self.weights[bin])
            })
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }

    #[must_use]
    pub const fn raw_boundary(&self) -> i32 {
        self.decision_boundary
    }
}

/// Encodes one validated version-two sparse artifact.
///
/// # Errors
///
/// Returns an error when the input metadata or calibration is invalid.
pub fn encode_sparse_v2(input: &SparseV2Input<'_>) -> Result<Vec<u8>, SparseModelError> {
    if input.weights.len() != BIN_COUNT {
        return Err(SparseModelError::InvalidLength {
            expected: BIN_COUNT,
            actual: input.weights.len(),
        });
    }
    if input.score_scale == 0 {
        return Err(SparseModelError::ZeroScoreScale);
    }
    if input.max_false_warning_basis_points > 10_000 {
        return Err(SparseModelError::InvalidFalseWarningLimit(
            input.max_false_warning_basis_points,
        ));
    }
    if input.language == Language::Es {
        return Err(SparseModelError::VersionTwoSpanish);
    }
    if input.language.profiles()
        != (
            input.feature_profile,
            input.normalization_profile,
            input.feature_schema,
        )
    {
        return Err(SparseModelError::ProfileMismatch);
    }

    let mut output = Vec::with_capacity(V2_HEADER_LENGTH + PAYLOAD_LENGTH);
    output.extend_from_slice(V2_MAGIC);
    output.extend_from_slice(&V2_FORMAT_VERSION.to_le_bytes());
    output.extend_from_slice(input.language.storage_code().as_bytes());
    output.extend_from_slice(&(BIN_COUNT as u32).to_le_bytes());
    output.extend_from_slice(&input.bias.to_le_bytes());
    output.extend_from_slice(&input.decision_boundary.to_le_bytes());
    output.extend_from_slice(&input.score_scale.to_le_bytes());
    output.extend_from_slice(&input.max_false_warning_basis_points.to_le_bytes());
    output.extend_from_slice(&WEIGHT_SCALE.to_le_bytes());
    output.push(input.feature_profile as u8);
    output.push(input.normalization_profile as u8);
    output.extend_from_slice(&(input.feature_schema as u16).to_le_bytes());
    output.extend_from_slice(&(PAYLOAD_LENGTH as u32).to_le_bytes());
    for weight in input.weights {
        output.extend_from_slice(&weight.to_le_bytes());
    }
    Ok(output)
}

fn parse_v1(bytes: &[u8]) -> Result<SparseModel, SparseModelError> {
    validate_exact_length(bytes, V1_HEADER_LENGTH + PAYLOAD_LENGTH)?;
    validate_version(bytes, V1_FORMAT_VERSION)?;
    let language = parse_language(&bytes[10..12])?;
    if language != Language::Es {
        return Err(SparseModelError::InvalidLanguage);
    }
    parse_payload(
        bytes,
        V1_HEADER_LENGTH,
        language,
        FeatureProfile::EsLegacyWordChar35V1,
        NormalizationProfile::EsLegacyCharabiaV1,
        FeatureSchema::EsLegacyV1,
    )
}

fn parse_v2(bytes: &[u8]) -> Result<SparseModel, SparseModelError> {
    if bytes.len() < V2_HEADER_LENGTH {
        return Err(SparseModelError::InvalidLength {
            expected: V2_HEADER_LENGTH,
            actual: bytes.len(),
        });
    }
    validate_version(bytes, V2_FORMAT_VERSION)?;
    let language = parse_language(&bytes[10..12])?;
    if language == Language::Es {
        return Err(SparseModelError::VersionTwoSpanish);
    }
    let feature_profile = parse_feature_profile(bytes[32])?;
    let normalization_profile = parse_normalization_profile(bytes[33])?;
    let feature_schema = parse_feature_schema(read_u16(bytes, 34))?;
    if language.profiles() != (feature_profile, normalization_profile, feature_schema) {
        return Err(SparseModelError::ProfileMismatch);
    }
    let payload_length = read_u32(bytes, 36);
    if usize::try_from(payload_length).ok() != Some(PAYLOAD_LENGTH) {
        return Err(SparseModelError::InvalidPayloadLength(payload_length));
    }
    validate_exact_length(bytes, V2_HEADER_LENGTH + PAYLOAD_LENGTH)?;
    parse_payload(
        bytes,
        V2_HEADER_LENGTH,
        language,
        feature_profile,
        normalization_profile,
        feature_schema,
    )
}

fn parse_payload(
    bytes: &[u8],
    header_length: usize,
    language: Language,
    feature_profile: FeatureProfile,
    normalization_profile: NormalizationProfile,
    feature_schema: FeatureSchema,
) -> Result<SparseModel, SparseModelError> {
    let bin_count = read_u32(bytes, 12);
    if usize::try_from(bin_count).ok() != Some(BIN_COUNT) {
        return Err(SparseModelError::InvalidBinCount(bin_count));
    }
    let score_scale = read_u32(bytes, 24);
    if score_scale == 0 {
        return Err(SparseModelError::ZeroScoreScale);
    }
    let max_false_warning_basis_points = read_u16(bytes, 28);
    if max_false_warning_basis_points > 10_000 {
        return Err(SparseModelError::InvalidFalseWarningLimit(
            max_false_warning_basis_points,
        ));
    }
    let weight_scale = read_u16(bytes, 30);
    if weight_scale != WEIGHT_SCALE {
        return Err(SparseModelError::InvalidWeightScale(weight_scale));
    }
    let weights = bytes[header_length..]
        .chunks_exact(size_of::<i16>())
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok(SparseModel {
        language,
        feature_profile,
        normalization_profile,
        feature_schema,
        weights,
        bias: read_i32(bytes, 16),
        decision_boundary: read_i32(bytes, 20),
        score_scale,
        max_false_warning_basis_points,
    })
}

fn validate_exact_length(bytes: &[u8], expected: usize) -> Result<(), SparseModelError> {
    if bytes.len() != expected {
        return Err(SparseModelError::InvalidLength {
            expected,
            actual: bytes.len(),
        });
    }
    Ok(())
}

fn validate_version(bytes: &[u8], expected: u16) -> Result<(), SparseModelError> {
    let version = read_u16(bytes, 8);
    if version != expected {
        return Err(SparseModelError::UnsupportedVersion(version));
    }
    Ok(())
}

fn parse_language(bytes: &[u8]) -> Result<Language, SparseModelError> {
    if !bytes.iter().all(u8::is_ascii_uppercase) {
        return Err(SparseModelError::InvalidLanguage);
    }
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or(SparseModelError::InvalidLanguage)
}

fn parse_feature_profile(value: u8) -> Result<FeatureProfile, SparseModelError> {
    match value {
        1 => Ok(FeatureProfile::EsLegacyWordChar35V1),
        2 => Ok(FeatureProfile::WordChar35V2),
        3 => Ok(FeatureProfile::Char25V2),
        _ => Err(SparseModelError::InvalidFeatureProfile(value)),
    }
}

fn parse_normalization_profile(value: u8) -> Result<NormalizationProfile, SparseModelError> {
    match value {
        1 => Ok(NormalizationProfile::EsLegacyCharabiaV1),
        2 => Ok(NormalizationProfile::GenericV2),
        3 => Ok(NormalizationProfile::TurkishV2),
        4 => Ok(NormalizationProfile::VietnameseV2),
        5 => Ok(NormalizationProfile::ArabicV2),
        6 => Ok(NormalizationProfile::HindiV2),
        7 => Ok(NormalizationProfile::ChineseV2),
        8 => Ok(NormalizationProfile::JapaneseV2),
        9 => Ok(NormalizationProfile::KoreanV2),
        _ => Err(SparseModelError::InvalidNormalizationProfile(value)),
    }
}

fn parse_feature_schema(value: u16) -> Result<FeatureSchema, SparseModelError> {
    match value {
        1 => Ok(FeatureSchema::EsLegacyV1),
        2 => Ok(FeatureSchema::SparseV2),
        _ => Err(SparseModelError::InvalidFeatureSchema(value)),
    }
}

pub(crate) fn embedded_model(language: &str) -> Option<&'static SparseModel> {
    language.trim().eq_ignore_ascii_case("ES").then(|| {
        SPANISH_MODEL.get_or_init(|| {
            let model = SparseModel::from_bytes(SPANISH_ARTIFACT)
                .expect("the embedded Spanish sparse artifact must be valid");
            assert_eq!(
                model.language(),
                Language::Es,
                "the embedded Spanish sparse artifact must declare ES"
            );
            model
        })
    })
}

fn score_from_raw(raw: i32, decision_boundary: i32, scale: u32) -> u8 {
    let delta = i64::from(raw) - i64::from(decision_boundary);
    let scaled_delta = delta.unsigned_abs().saturating_mul(50);
    let scale = u64::from(scale);
    let magnitude = if delta < 0 {
        scaled_delta.div_ceil(scale)
    } else {
        scaled_delta / scale
    }
    .min(50) as u8;
    if delta >= 0 {
        50_u8.saturating_add(magnitude)
    } else {
        50_u8.saturating_sub(magnitude)
    }
}

fn read_u16(bytes: &[u8], start: usize) -> u16 {
    u16::from_le_bytes([bytes[start], bytes[start + 1]])
}

fn read_u32(bytes: &[u8], start: usize) -> u32 {
    u32::from_le_bytes([
        bytes[start],
        bytes[start + 1],
        bytes[start + 2],
        bytes[start + 3],
    ])
}

fn read_i32(bytes: &[u8], start: usize) -> i32 {
    i32::from_le_bytes([
        bytes[start],
        bytes[start + 1],
        bytes[start + 2],
        bytes[start + 3],
    ])
}

#[cfg(test)]
mod tests {
    use super::score_from_raw;

    #[test]
    fn a_raw_score_below_the_boundary_maps_below_the_nudge_threshold() {
        assert_eq!(score_from_raw(999, 1_000, 10_000), 49);
    }
}
