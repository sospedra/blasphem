use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Language {
    #[serde(rename = "EN")]
    En = 0,
    #[serde(rename = "ZH")]
    Zh = 1,
    #[serde(rename = "ES")]
    Es = 2,
    #[serde(rename = "AR")]
    Ar = 3,
    #[serde(rename = "MS", alias = "ID")]
    Ms = 4,
    #[serde(rename = "PT")]
    Pt = 5,
    #[serde(rename = "FR")]
    Fr = 6,
    #[serde(rename = "HI")]
    Hi = 7,
    #[serde(rename = "RU")]
    Ru = 8,
    #[serde(rename = "JA")]
    Ja = 9,
    #[serde(rename = "DE")]
    De = 10,
    #[serde(rename = "TR")]
    Tr = 11,
    #[serde(rename = "VI")]
    Vi = 12,
    #[serde(rename = "KO")]
    Ko = 13,
    #[serde(rename = "IT")]
    It = 14,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FeatureProfile {
    #[serde(rename = "EsLegacyWordChar35V1")]
    EsLegacyWordChar35V1 = 1,
    #[serde(rename = "WordChar35V2")]
    WordChar35V2 = 2,
    #[serde(rename = "Char25V2")]
    Char25V2 = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum NormalizationProfile {
    #[serde(rename = "EsLegacyCharabiaV1")]
    EsLegacyCharabiaV1 = 1,
    #[serde(rename = "GenericV2")]
    GenericV2 = 2,
    #[serde(rename = "TurkishV2")]
    TurkishV2 = 3,
    #[serde(rename = "VietnameseV2")]
    VietnameseV2 = 4,
    #[serde(rename = "ArabicV2")]
    ArabicV2 = 5,
    #[serde(rename = "HindiV2")]
    HindiV2 = 6,
    #[serde(rename = "ChineseV2")]
    ChineseV2 = 7,
    #[serde(rename = "JapaneseV2")]
    JapaneseV2 = 8,
    #[serde(rename = "KoreanV2")]
    KoreanV2 = 9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u16)]
pub enum FeatureSchema {
    #[serde(rename = "EsLegacyV1")]
    EsLegacyV1 = 1,
    #[serde(rename = "SparseV2")]
    SparseV2 = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unsupported language: {0}")]
pub struct UnsupportedLanguage(String);

impl Language {
    pub const ALL: [Self; 15] = [
        Self::En,
        Self::Zh,
        Self::Es,
        Self::Ar,
        Self::Ms,
        Self::Pt,
        Self::Fr,
        Self::Hi,
        Self::Ru,
        Self::Ja,
        Self::De,
        Self::Tr,
        Self::Vi,
        Self::Ko,
        Self::It,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Zh => "ZH",
            Self::Es => "ES",
            Self::Ar => "AR",
            Self::Ms => "MS",
            Self::Pt => "PT",
            Self::Fr => "FR",
            Self::Hi => "HI",
            Self::Ru => "RU",
            Self::Ja => "JA",
            Self::De => "DE",
            Self::Tr => "TR",
            Self::Vi => "VI",
            Self::Ko => "KO",
            Self::It => "IT",
        }
    }

    /// Returns the legacy code used by immutable on-disk resources.
    #[must_use]
    pub const fn storage_code(self) -> &'static str {
        match self {
            Self::Ms => "ID",
            _ => self.code(),
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::En => 0,
            Self::Zh => 1,
            Self::Es => 2,
            Self::Ar => 3,
            Self::Ms => 4,
            Self::Pt => 5,
            Self::Fr => 6,
            Self::Hi => 7,
            Self::Ru => 8,
            Self::Ja => 9,
            Self::De => 10,
            Self::Tr => 11,
            Self::Vi => 12,
            Self::Ko => 13,
            Self::It => 14,
        }
    }

    pub const fn profiles(self) -> (FeatureProfile, NormalizationProfile, FeatureSchema) {
        match self {
            Self::En | Self::Ms | Self::Pt | Self::Fr | Self::Ru | Self::De | Self::It => (
                FeatureProfile::WordChar35V2,
                NormalizationProfile::GenericV2,
                FeatureSchema::SparseV2,
            ),
            Self::Zh => (
                FeatureProfile::Char25V2,
                NormalizationProfile::ChineseV2,
                FeatureSchema::SparseV2,
            ),
            Self::Es => (
                FeatureProfile::EsLegacyWordChar35V1,
                NormalizationProfile::EsLegacyCharabiaV1,
                FeatureSchema::EsLegacyV1,
            ),
            Self::Ar => (
                FeatureProfile::WordChar35V2,
                NormalizationProfile::ArabicV2,
                FeatureSchema::SparseV2,
            ),
            Self::Hi => (
                FeatureProfile::WordChar35V2,
                NormalizationProfile::HindiV2,
                FeatureSchema::SparseV2,
            ),
            Self::Ja => (
                FeatureProfile::Char25V2,
                NormalizationProfile::JapaneseV2,
                FeatureSchema::SparseV2,
            ),
            Self::Tr => (
                FeatureProfile::WordChar35V2,
                NormalizationProfile::TurkishV2,
                FeatureSchema::SparseV2,
            ),
            Self::Vi => (
                FeatureProfile::WordChar35V2,
                NormalizationProfile::VietnameseV2,
                FeatureSchema::SparseV2,
            ),
            Self::Ko => (
                FeatureProfile::Char25V2,
                NormalizationProfile::KoreanV2,
                FeatureSchema::SparseV2,
            ),
        }
    }
}

impl FromStr for Language {
    type Err = UnsupportedLanguage;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_uppercase().as_str() {
            "EN" => Ok(Self::En),
            "ZH" => Ok(Self::Zh),
            "ES" => Ok(Self::Es),
            "AR" => Ok(Self::Ar),
            "MS" | "ID" => Ok(Self::Ms),
            "PT" => Ok(Self::Pt),
            "FR" => Ok(Self::Fr),
            "HI" => Ok(Self::Hi),
            "RU" => Ok(Self::Ru),
            "JA" => Ok(Self::Ja),
            "DE" => Ok(Self::De),
            "TR" => Ok(Self::Tr),
            "VI" => Ok(Self::Vi),
            "KO" => Ok(Self::Ko),
            "IT" => Ok(Self::It),
            _ => Err(UnsupportedLanguage(value.to_owned())),
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}
