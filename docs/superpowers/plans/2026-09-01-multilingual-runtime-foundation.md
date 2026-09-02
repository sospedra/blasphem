# Multilingual runtime foundation implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add typed languages, immutable Unicode profiles, feature schemas, and version-two sparse artifacts without changing Spanish behavior.

**Architecture:** Keep the Spanish version-one path byte-exact. Add isolated version-two normalization and feature modules. Make later model and data work depend on typed shared interfaces.

**Tech stack:** Rust 2024, Charabia 0.9.9, Unicode normalization, Unicode general categories, SHA-256, and fixed dense `i16` tables.

**Spec:** `docs/superpowers/specs/2026-09-01-multilingual-sparse-nudge-detector-design.md`

## Global constraints

The Rust runtime shall not call a network service or run an AI model.

The Spanish artifact SHA-256 shall remain `3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36`.

The Spanish audit SHA-256 shall remain `8313713f8e18e5c066f6f320efb6ee340b7580cba4739fc4612e1dfe4a8a7575`.

The public threshold shall remain `50`.

The score shall remain ordinal and shall range from zero through 100.

The project directory is not a Git repository. Each task ends with a verification checkpoint instead of a commit.

---

## File structure

- Create `src/language.rs` for the 15 language codes and immutable profile identifiers.
- Create `src/normalization.rs` for every version-two Unicode transformation.
- Create `src/features.rs` for the legacy extractor and two version-two extractors.
- Modify `src/sparse.rs` for version-one and version-two artifact parsing and scoring.
- Modify `src/lib.rs` for stable public exports.
- Create `tests/spanish_compatibility.rs` for frozen Spanish assets and product outputs.
- Create `tests/profile_contract.rs` for language and normalization contracts.
- Create `tests/sparse_v2.rs` for version-two header and score contracts.

### Task 1: Freeze Spanish assets and outputs

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `tests/spanish_compatibility.rs`
- Read: `resources/models/es-chargram-v1.bin`
- Read: `samples/spanish-audit.tsv`

**Interfaces:**

- Consumes: The current `toxcheck` binary and current Spanish assets.
- Produces: Immutable hashes and product-output regression tests.

- [ ] **Step 1: Write the asset hash test**

```rust
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn spanish_assets_are_frozen() {
    let model = include_bytes!("../resources/models/es-chargram-v1.bin");
    let audit = include_bytes!("../samples/spanish-audit.tsv");
    assert_eq!(
        sha256_hex(model),
        "3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36"
    );
    assert_eq!(
        sha256_hex(audit),
        "8313713f8e18e5c066f6f320efb6ee340b7580cba4739fc4612e1dfe4a8a7575"
    );
}
```

- [ ] **Step 2: Run the hash test and confirm the missing dependency failure**

Run: `cargo test --test spanish_compatibility spanish_assets_are_frozen`

Expected: FAIL because `sha2` is not declared.

- [ ] **Step 3: Add the runtime SHA-256 dependency**

```toml
[dependencies]
sha2 = "0.10"
```

- [ ] **Step 4: Add exact Spanish CLI regression cases**

```rust
use std::process::Command;

fn check(text: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .args([
            "check",
            "--language",
            "ES",
            "--data-dir",
            "data/hurtlex",
            "--text",
            text,
        ])
        .output()
        .expect("run toxcheck");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

#[test]
fn spanish_product_results_are_frozen() {
    assert!(check("Te voy a matar")
        .starts_with("ok=false score=95 threshold=50 should_nudge=true"));
    assert!(check("No te voy a matar")
        .starts_with("ok=true score=24 threshold=50 should_nudge=false"));
}
```

- [ ] **Step 5: Run the Spanish compatibility test**

Run: `cargo test --test spanish_compatibility`

Expected: PASS with two tests.

### Task 2: Add typed languages and immutable profile identifiers

**Files:**

- Create: `src/language.rs`
- Modify: `src/lib.rs`
- Create: `tests/profile_contract.rs`

**Interfaces:**

- Consumes: Two-letter detector codes from the specification.
- Produces: `Language`, `FeatureProfile`, `NormalizationProfile`, and `FeatureSchema`.

- [ ] **Step 1: Write the failing language coverage test**

```rust
use std::str::FromStr;
use toxcheck::{FeatureProfile, FeatureSchema, Language, NormalizationProfile};

#[test]
fn language_contract_contains_exactly_fifteen_codes() {
    let expected = [
        "EN", "ZH", "ES", "AR", "ID", "PT", "FR", "HI", "RU", "JA", "DE", "TR",
        "VI", "KO", "IT",
    ];
    let actual = Language::ALL.map(Language::code);
    assert_eq!(actual, expected);
    for code in expected {
        assert_eq!(Language::from_str(code).expect("supported").code(), code);
        assert_eq!(Language::from_str(&code.to_ascii_lowercase()).expect("supported").code(), code);
    }
    assert!(Language::from_str("HINGLISH").is_err());
}

#[test]
fn language_profiles_and_indexes_match_the_exact_table() {
    let cases = [
        (Language::En, 0, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
        (Language::Zh, 1, FeatureProfile::Char25V2, NormalizationProfile::ChineseV2, FeatureSchema::SparseV2),
        (Language::Es, 2, FeatureProfile::EsLegacyWordChar35V1, NormalizationProfile::EsLegacyCharabiaV1, FeatureSchema::EsLegacyV1),
        (Language::Ar, 3, FeatureProfile::WordChar35V2, NormalizationProfile::ArabicV2, FeatureSchema::SparseV2),
        (Language::Id, 4, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
        (Language::Pt, 5, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
        (Language::Fr, 6, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
        (Language::Hi, 7, FeatureProfile::WordChar35V2, NormalizationProfile::HindiV2, FeatureSchema::SparseV2),
        (Language::Ru, 8, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
        (Language::Ja, 9, FeatureProfile::Char25V2, NormalizationProfile::JapaneseV2, FeatureSchema::SparseV2),
        (Language::De, 10, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
        (Language::Tr, 11, FeatureProfile::WordChar35V2, NormalizationProfile::TurkishV2, FeatureSchema::SparseV2),
        (Language::Vi, 12, FeatureProfile::WordChar35V2, NormalizationProfile::VietnameseV2, FeatureSchema::SparseV2),
        (Language::Ko, 13, FeatureProfile::Char25V2, NormalizationProfile::KoreanV2, FeatureSchema::SparseV2),
        (Language::It, 14, FeatureProfile::WordChar35V2, NormalizationProfile::GenericV2, FeatureSchema::SparseV2),
    ];

    for (language, index, feature, normalization, schema) in cases {
        assert_eq!(Language::ALL[index], language);
        assert_eq!(language.index(), index);
        assert_eq!(language.profiles(), (feature, normalization, schema));
    }
}

#[test]
fn language_json_uses_only_uppercase_codes() {
    for language in Language::ALL {
        let json = serde_json::to_string(&language).expect("serialize language");
        assert_eq!(json, format!("\"{}\"", language.code()));
        assert_eq!(
            serde_json::from_str::<Language>(&json).expect("deserialize language"),
            language
        );
    }
    assert!(serde_json::from_str::<Language>("\"es\"").is_err());
}

#[test]
fn profile_json_names_match_the_exact_tables() {
    let feature_cases = [
        (FeatureProfile::EsLegacyWordChar35V1, "EsLegacyWordChar35V1"),
        (FeatureProfile::WordChar35V2, "WordChar35V2"),
        (FeatureProfile::Char25V2, "Char25V2"),
    ];
    for (profile, name) in feature_cases {
        let json = format!("\"{name}\"");
        assert_eq!(serde_json::to_string(&profile).expect("serialize feature profile"), json);
        assert_eq!(
            serde_json::from_str::<FeatureProfile>(&json).expect("deserialize feature profile"),
            profile
        );
    }

    let normalization_cases = [
        (NormalizationProfile::EsLegacyCharabiaV1, "EsLegacyCharabiaV1"),
        (NormalizationProfile::GenericV2, "GenericV2"),
        (NormalizationProfile::TurkishV2, "TurkishV2"),
        (NormalizationProfile::VietnameseV2, "VietnameseV2"),
        (NormalizationProfile::ArabicV2, "ArabicV2"),
        (NormalizationProfile::HindiV2, "HindiV2"),
        (NormalizationProfile::ChineseV2, "ChineseV2"),
        (NormalizationProfile::JapaneseV2, "JapaneseV2"),
        (NormalizationProfile::KoreanV2, "KoreanV2"),
    ];
    for (profile, name) in normalization_cases {
        let json = format!("\"{name}\"");
        assert_eq!(serde_json::to_string(&profile).expect("serialize normalization profile"), json);
        assert_eq!(
            serde_json::from_str::<NormalizationProfile>(&json)
                .expect("deserialize normalization profile"),
            profile
        );
    }

    let schema_cases = [
        (FeatureSchema::EsLegacyV1, "EsLegacyV1"),
        (FeatureSchema::SparseV2, "SparseV2"),
    ];
    for (schema, name) in schema_cases {
        let json = format!("\"{name}\"");
        assert_eq!(serde_json::to_string(&schema).expect("serialize feature schema"), json);
        assert_eq!(
            serde_json::from_str::<FeatureSchema>(&json).expect("deserialize feature schema"),
            schema
        );
    }
}
```

- [ ] **Step 2: Run the language test and confirm the missing type failure**

Run: `cargo test --test profile_contract language_contract_contains_exactly_fifteen_codes`

Expected: FAIL because `Language` does not exist.

- [ ] **Step 3: Add the typed identifiers**

```rust
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
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
    #[serde(rename = "ID")]
    Id = 4,
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
```

- [ ] **Step 4: Add exact code parsing and profile mapping**

```rust
impl Language {
    pub const ALL: [Self; 15] = [
        Self::En, Self::Zh, Self::Es, Self::Ar, Self::Id,
        Self::Pt, Self::Fr, Self::Hi, Self::Ru, Self::Ja,
        Self::De, Self::Tr, Self::Vi, Self::Ko, Self::It,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Zh => "ZH",
            Self::Es => "ES",
            Self::Ar => "AR",
            Self::Id => "ID",
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

    pub const fn index(self) -> usize {
        match self {
            Self::En => 0,
            Self::Zh => 1,
            Self::Es => 2,
            Self::Ar => 3,
            Self::Id => 4,
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
            Self::En | Self::Id | Self::Pt | Self::Fr | Self::Ru | Self::De | Self::It => (
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
            "ID" => Ok(Self::Id),
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
```

- [ ] **Step 5: Export the identifiers and run the focused test**

Run: `cargo test --test profile_contract`

Expected: PASS with all four language and profile tests.

### Task 3: Implement version-two normalization

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Create: `src/normalization.rs`
- Modify: `src/lib.rs`
- Modify: `tests/profile_contract.rs`

**Interfaces:**

- Consumes: `NormalizationProfile` and raw UTF-8 text.
- Produces: `normalize_v2(profile, text) -> Result<String, NormalizationError>`.

- [ ] **Step 1: Add frozen normalization vectors**

```rust
use toxcheck::{NormalizationError, NormalizationProfile, normalize_v2};

#[test]
fn normalization_profiles_match_frozen_vectors() {
    let cases = [
        (NormalizationProfile::GenericV2, "ＦＯＯ Straße", "foo straße"),
        (NormalizationProfile::TurkishV2, "I İ ı i", "ı i ı i"),
        (NormalizationProfile::VietnameseV2, "Tôi rất tệ", "tôi rất tệ"),
        (NormalizationProfile::ArabicV2, "إِنَّ ـآدم فتاة", "ان ادم فتاة"),
        (NormalizationProfile::HindiV2, "क्\u{200d}ष", "क्\u{200d}ष"),
        (NormalizationProfile::ChineseV2, "ＡＢＣ你", "abc你"),
        (NormalizationProfile::JapaneseV2, "ガＡ", "ガa"),
        (NormalizationProfile::KoreanV2, "한글Ａ", "한글a"),
    ];
    for (profile, input, expected) in cases {
        assert_eq!(normalize_v2(profile, input).expect("normalize"), expected);
    }

    assert_eq!(
        normalize_v2(NormalizationProfile::EsLegacyCharabiaV1, "texto"),
        Err(NormalizationError::LegacyProfile)
    );
}
```

- [ ] **Step 2: Run the vector test and confirm the missing function failure**

Run: `cargo test --test profile_contract normalization_profiles_match_frozen_vectors`

Expected: FAIL because `normalize_v2` does not exist.

- [ ] **Step 3: Add Unicode dependencies**

```toml
unicode-normalization = "0.1"
unicode-general-category = "1.0"
unicode-script = "0.5"
```

- [ ] **Step 4: Implement the normalization dispatcher**

```rust
use thiserror::Error;
use unicode_normalization::UnicodeNormalization;

use crate::NormalizationProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum NormalizationError {
    #[error("the version-one Spanish profile cannot use the version-two normalizer")]
    LegacyProfile,
}

pub fn normalize_v2(
    profile: NormalizationProfile,
    text: &str,
) -> Result<String, NormalizationError> {
    match profile {
        NormalizationProfile::EsLegacyCharabiaV1 => Err(NormalizationError::LegacyProfile),
        NormalizationProfile::GenericV2 => Ok(nfkc_lower(text)),
        NormalizationProfile::TurkishV2 => Ok(turkish(text)),
        NormalizationProfile::VietnameseV2 => Ok(nfkc_lower(text)),
        NormalizationProfile::ArabicV2 => Ok(arabic(text)),
        NormalizationProfile::HindiV2 => Ok(text.nfkc().collect()),
        NormalizationProfile::ChineseV2
        | NormalizationProfile::JapaneseV2
        | NormalizationProfile::KoreanV2 => Ok(nfkc_lower(text)),
    }
}

fn nfkc_lower(text: &str) -> String {
    text.nfkc().flat_map(char::to_lowercase).collect()
}
```

- [ ] **Step 5: Implement exact Turkish and Arabic transforms**

```rust
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
```

Export `normalize_v2` and `NormalizationError` from `src/lib.rs`.

- [ ] **Step 6: Run normalization and Spanish tests**

Run: `cargo test --test profile_contract --test spanish_compatibility`

Expected: PASS.

### Task 4: Isolate the three feature profiles

**Files:**

- Create: `src/features.rs`
- Modify: `src/sparse.rs`
- Modify: `src/lib.rs`
- Test: `src/features.rs`
- Test: `tests/spanish_compatibility.rs`

**Interfaces:**

- Consumes: `FeatureProfile`, `NormalizationProfile`, and raw text.
- Produces: `extract_feature_bins(profile, normalization, text) -> Result<Vec<usize>, FeatureError>`.

- [ ] **Step 1: Freeze the current Spanish feature bins before the move**

Extend the existing `src/sparse.rs` test module with this table.

```rust
use super::{feature_bins, score_from_raw};

#[test]
fn spanish_feature_bins_match_frozen_tables_before_move() {
    let cases: &[(&str, &[usize])] = &[
        ("tox", &[1722, 1731, 8133, 26526, 42498, 44885, 64854]),
        (
            "eres basura",
            &[
                173, 1571, 4768, 7139, 8537, 9657, 10926, 13214, 15622,
                16407, 16691, 18105, 24303, 29095, 29407, 29533, 31647,
                33951, 37144, 40126, 41186, 46864, 48597, 50768, 54925,
                57782, 63971,
            ],
        ),
    ];

    for &(text, expected) in cases {
        let actual = feature_bins(text);
        assert_eq!(actual.as_slice(), expected);
    }
}
```

- [ ] **Step 2: Run the pre-move Spanish feature test**

Run: `cargo test --lib sparse::tests::spanish_feature_bins_match_frozen_tables_before_move`

Expected: PASS against the current private `feature_bins` function.

- [ ] **Step 3: Move the Spanish extractor without edits**

Move the current function body without edits. Rename only `feature_bins` to `es_legacy_feature_bins`.

```rust
use std::collections::BTreeSet;

use thiserror::Error;
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_script::{Script, UnicodeScript};

use crate::{
    FeatureProfile, NormalizationError, NormalizationProfile, normalize_text, normalize_v2,
};

const BIN_COUNT: usize = 65_536;
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FeatureError {
    #[error(transparent)]
    Normalization(#[from] NormalizationError),
    #[error("feature profile {feature:?} cannot use normalization profile {normalization:?}")]
    ProfileMismatch {
        feature: FeatureProfile,
        normalization: NormalizationProfile,
    },
}

pub(crate) fn es_legacy_feature_bins(text: &str) -> Vec<usize> {
    let normalized = normalize_text(text);
    let words = normalized.split_whitespace().collect::<Vec<_>>();
    let mut bins = BTreeSet::new();

    for word in &words {
        bins.insert(feature_hash(b'W', 1, [word.as_bytes()]) & (BIN_COUNT - 1));
        let mut characters = Vec::with_capacity(word.chars().count() + 2);
        characters.push('\u{2}');
        characters.extend(word.chars());
        characters.push('\u{3}');
        for length in 3..=5 {
            for gram in characters.windows(length) {
                bins.insert(character_feature_hash(length as u8, gram) & (BIN_COUNT - 1));
            }
        }
    }
    for pair in words.windows(2) {
        bins.insert(
            feature_hash(b'W', 2, [pair[0].as_bytes(), pair[1].as_bytes()]) & (BIN_COUNT - 1),
        );
    }

    bins.into_iter().collect()
}

fn feature_hash<'a>(
    namespace: u8,
    arity: u8,
    parts: impl IntoIterator<Item = &'a [u8]>,
) -> usize {
    let mut hash = FNV_OFFSET;
    update_hash(&mut hash, &[namespace, arity]);
    for part in parts {
        update_hash(&mut hash, &[0]);
        update_hash(&mut hash, part);
    }
    hash as usize
}

fn character_feature_hash(length: u8, characters: &[char]) -> usize {
    let mut hash = FNV_OFFSET;
    update_hash(&mut hash, &[b'C', length]);
    for character in characters {
        let mut buffer = [0_u8; 4];
        update_hash(&mut hash, character.encode_utf8(&mut buffer).as_bytes());
    }
    hash as usize
}

fn update_hash(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}
```

Import `es_legacy_feature_bins` in `src/sparse.rs`. Replace every old `feature_bins` call with the new name.

Move the pre-move test into `src/features.rs`. Rename the test and its function call only.

```rust
#[test]
fn spanish_feature_bins_match_frozen_tables_after_move() {
    let cases: &[(&str, &[usize])] = &[
        ("tox", &[1722, 1731, 8133, 26526, 42498, 44885, 64854]),
        (
            "eres basura",
            &[
                173, 1571, 4768, 7139, 8537, 9657, 10926, 13214, 15622,
                16407, 16691, 18105, 24303, 29095, 29407, 29533, 31647,
                33951, 37144, 40126, 41186, 46864, 48597, 50768, 54925,
                57782, 63971,
            ],
        ),
    ];

    for &(text, expected) in cases {
        let actual = es_legacy_feature_bins(text);
        assert_eq!(actual.as_slice(), expected);
    }
}
```

Run: `cargo test --lib features::tests::spanish_feature_bins_match_frozen_tables_after_move`

Expected: PASS with the same two Spanish vectors.

Run: `cargo test --test spanish_compatibility`

Expected: PASS with the same Spanish product outputs.

- [ ] **Step 4: Add exact table-driven profile tests**

Add these tests to the `src/features.rs` test module.

```rust
#[test]
fn feature_profiles_match_exact_bin_tables() {
    let cases: &[(FeatureProfile, NormalizationProfile, &str, &[usize])] = &[
        (
            FeatureProfile::EsLegacyWordChar35V1,
            NormalizationProfile::EsLegacyCharabiaV1,
            "tox",
            &[1722, 1731, 8133, 26526, 42498, 44885, 64854],
        ),
        (
            FeatureProfile::EsLegacyWordChar35V1,
            NormalizationProfile::EsLegacyCharabiaV1,
            "eres basura",
            &[
                173, 1571, 4768, 7139, 8537, 9657, 10926, 13214, 15622,
                16407, 16691, 18105, 24303, 29095, 29407, 29533, 31647,
                33951, 37144, 40126, 41186, 46864, 48597, 50768, 54925,
                57782, 63971,
            ],
        ),
        (
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2,
            "ab cd. ef",
            &[
                3680, 10476, 13789, 21170, 23008, 35036, 36904, 36952,
                40269, 43645, 45500, 45548, 59368,
            ],
        ),
        (
            FeatureProfile::Char25V2,
            NormalizationProfile::ChineseV2,
            "你 去死。",
            &[
                1283, 1579, 15489, 22698, 26691, 32640, 47167, 50706,
                51814, 59498,
            ],
        ),
    ];

    for &(feature, normalization, text, expected) in cases {
        let actual = extract_feature_bins(feature, normalization, text).expect("features");
        assert_eq!(actual.as_slice(), expected);
    }
}

#[test]
fn char_profile_emits_only_character_namespace_events() {
    let mut namespaces = Vec::new();
    compact_char_25_with(
        NormalizationProfile::ChineseV2,
        "你 去死。",
        |namespace, _| namespaces.push(namespace),
    )
    .expect("features");

    assert!(!namespaces.is_empty());
    assert!(namespaces.iter().all(|namespace| *namespace == b'C'));
}
```

The four vectors are fixed outputs from the shown 64-bit FNV-1a implementation.

Run: `cargo test --lib features::tests::feature_profiles_match_exact_bin_tables`

Expected: FAIL because the version-two feature functions do not exist.

- [ ] **Step 5: Add the version-two token iterator**

```rust
struct NormalizedToken {
    text: String,
    clause: u32,
}

fn word_tokens(
    profile: NormalizationProfile,
    text: &str,
) -> Result<Vec<NormalizedToken>, FeatureError> {
    let normalized = normalize_v2(profile, text)?;
    let characters = normalized.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut clause = 0_u32;

    for (index, character) in characters.iter().copied().enumerate() {
        if is_word_character(character) || is_hindi_joiner(&characters, index, profile) {
            current.push(character);
            continue;
        }
        if !current.is_empty() {
            tokens.push(NormalizedToken {
                text: std::mem::take(&mut current),
                clause,
            });
        }
        if is_clause_boundary(character) {
            clause = clause.saturating_add(1);
        }
    }
    if !current.is_empty() {
        tokens.push(NormalizedToken {
            text: current,
            clause,
        });
    }
    Ok(tokens)
}

fn is_word_character(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::DecimalNumber
            | GeneralCategory::LetterNumber
            | GeneralCategory::OtherNumber
            | GeneralCategory::NonspacingMark
            | GeneralCategory::SpacingMark
            | GeneralCategory::EnclosingMark
    )
}

fn is_hindi_joiner(
    characters: &[char],
    index: usize,
    profile: NormalizationProfile,
) -> bool {
    if profile != NormalizationProfile::HindiV2
        || !matches!(characters[index], '\u{200c}' | '\u{200d}')
    {
        return false;
    }
    let previous = index.checked_sub(1).and_then(|value| characters.get(value));
    let next = characters.get(index + 1);
    previous.is_some_and(|character| character.script() == Script::Devanagari)
        && next.is_some_and(|character| character.script() == Script::Devanagari)
}

fn is_clause_boundary(character: char) -> bool {
    matches!(
        character,
        '.' | '!' | '?' | ';' | ':' | '。' | '！' | '？' | '；' | '：' | '؟' | '؛' | '।'
            | '\n' | '\r'
    )
}
```

- [ ] **Step 6: Add word and compact feature extraction**

```rust
pub fn extract_feature_bins(
    feature: FeatureProfile,
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    match (feature, normalization) {
        (
            FeatureProfile::EsLegacyWordChar35V1,
            NormalizationProfile::EsLegacyCharabiaV1,
        ) => Ok(es_legacy_feature_bins(text)),
        (
            FeatureProfile::WordChar35V2,
            NormalizationProfile::GenericV2
            | NormalizationProfile::TurkishV2
            | NormalizationProfile::VietnameseV2
            | NormalizationProfile::ArabicV2
            | NormalizationProfile::HindiV2,
        ) => word_char_35(normalization, text),
        (
            FeatureProfile::Char25V2,
            NormalizationProfile::ChineseV2
            | NormalizationProfile::JapaneseV2
            | NormalizationProfile::KoreanV2,
        ) => compact_char_25(normalization, text),
        _ => Err(FeatureError::ProfileMismatch {
            feature,
            normalization,
        }),
    }
}

fn word_char_35(
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    let tokens = word_tokens(normalization, text)?;
    let mut bins = BTreeSet::new();
    for token in &tokens {
        bins.insert(feature_hash(b'W', 1, [token.text.as_bytes()]) & (BIN_COUNT - 1));
        emit_character_grams(&token.text.chars().collect::<Vec<_>>(), 3, 5, |_, bin| {
            bins.insert(bin);
        });
    }
    for pair in tokens.windows(2).filter(|pair| pair[0].clause == pair[1].clause) {
        bins.insert(
            feature_hash(
                b'W',
                2,
                [pair[0].text.as_bytes(), pair[1].text.as_bytes()],
            ) & (BIN_COUNT - 1),
        );
    }
    Ok(bins.into_iter().collect())
}

fn compact_char_25(
    normalization: NormalizationProfile,
    text: &str,
) -> Result<Vec<usize>, FeatureError> {
    let mut bins = BTreeSet::new();
    compact_char_25_with(normalization, text, |_, bin| {
        bins.insert(bin);
    })?;
    Ok(bins.into_iter().collect())
}

fn compact_char_25_with(
    normalization: NormalizationProfile,
    text: &str,
    mut emit: impl FnMut(u8, usize),
) -> Result<(), FeatureError> {
    let normalized = normalize_v2(normalization, text)?;
    let mut segment = Vec::new();
    for character in normalized.chars() {
        if is_compact_boundary(character) {
            emit_character_grams(&segment, 2, 5, &mut emit);
            segment.clear();
        } else if !character.is_whitespace() {
            segment.push(character);
        }
    }
    emit_character_grams(&segment, 2, 5, emit);
    Ok(())
}

fn emit_character_grams(
    content: &[char],
    minimum: usize,
    maximum: usize,
    mut emit: impl FnMut(u8, usize),
) {
    if content.is_empty() {
        return;
    }
    let mut characters = Vec::with_capacity(content.len() + 2);
    characters.push('\u{2}');
    characters.extend_from_slice(content);
    characters.push('\u{3}');
    for length in minimum..=maximum {
        for gram in characters.windows(length) {
            emit(
                b'C',
                character_feature_hash(length as u8, gram) & (BIN_COUNT - 1),
            );
        }
    }
}

fn is_compact_boundary(character: char) -> bool {
    matches!(
        get_general_category(character),
        GeneralCategory::ConnectorPunctuation
            | GeneralCategory::DashPunctuation
            | GeneralCategory::OpenPunctuation
            | GeneralCategory::ClosePunctuation
            | GeneralCategory::InitialPunctuation
            | GeneralCategory::FinalPunctuation
            | GeneralCategory::OtherPunctuation
            | GeneralCategory::MathSymbol
            | GeneralCategory::CurrencySymbol
            | GeneralCategory::ModifierSymbol
            | GeneralCategory::OtherSymbol
            | GeneralCategory::LineSeparator
            | GeneralCategory::ParagraphSeparator
            | GeneralCategory::Control
            | GeneralCategory::Format
    )
}
```

Export `extract_feature_bins` and `FeatureError` from `src/lib.rs`. The separate `toxtrain` crate shall call this function directly.

- [ ] **Step 7: Route `SparseModel::raw_score` through the stored profile**

Import `extract_feature_bins`, `FeatureProfile`, `FeatureSchema`, and `NormalizationProfile` in `src/sparse.rs`.

Add these fields to the current `SparseModel`.

```rust
feature_profile: FeatureProfile,
normalization_profile: NormalizationProfile,
feature_schema: FeatureSchema,
max_false_warning_basis_points: u16,
```

Add these entries to the V1 loader initializer.

```rust
feature_profile: FeatureProfile::EsLegacyWordChar35V1,
normalization_profile: NormalizationProfile::EsLegacyCharabiaV1,
feature_schema: FeatureSchema::EsLegacyV1,
max_false_warning_basis_points: read_u16(bytes, 28),
```

Add these entries to the provisional compiler initializer.

```rust
feature_profile: FeatureProfile::EsLegacyWordChar35V1,
normalization_profile: NormalizationProfile::EsLegacyCharabiaV1,
feature_schema: FeatureSchema::EsLegacyV1,
max_false_warning_basis_points: max_false_positive_basis_points,
```

```rust
pub fn raw_score(&self, text: &str) -> i32 {
    let bins = extract_feature_bins(self.feature_profile, self.normalization_profile, text)
        .expect("validated artifact profile");
    bins.into_iter()
        .fold(i64::from(self.bias), |sum, bin| {
            sum + i64::from(self.weights[bin])
        })
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

pub const fn raw_boundary(&self) -> i32 {
    self.decision_boundary
}
```

Export `SparseModel`. Keep `decision_boundary` as the stored V1 and V2 field name.

- [ ] **Step 8: Run profile and Spanish regression tests**

Run: `cargo test --lib features::tests`

Expected: PASS.

Run: `cargo test --test spanish_compatibility`

Expected: PASS with unchanged Spanish hashes and first lines.

### Task 5: Add the version-two sparse artifact

**Files:**

- Modify: `src/sparse.rs`
- Modify: `src/lib.rs`
- Create: `tests/sparse_v2.rs`
- Modify: `tests/sparse.rs`

**Interfaces:**

- Consumes: One language, profile IDs, calibration fields, and 65,536 `i16` weights.
- Produces: `SparseModel::from_bytes`, `encode_sparse_v2`, and validated metadata.

- [ ] **Step 1: Write V1 and V2 parser tests**

```rust
use toxcheck::{
    FeatureProfile, FeatureSchema, Language, NormalizationProfile, SparseModel,
    SparseModelError, SparseV2Input, encode_sparse_v2,
};

fn fixture_v2_input(language: Language, weights: &[i16]) -> SparseV2Input<'_> {
    let (feature_profile, normalization_profile, feature_schema) = language.profiles();
    SparseV2Input {
        language,
        feature_profile,
        normalization_profile,
        feature_schema,
        bias: -64,
        decision_boundary: 128,
        score_scale: 256,
        max_false_warning_basis_points: 300,
        weights,
    }
}

#[test]
fn v1_infers_only_the_spanish_legacy_profiles() {
    let model = SparseModel::from_bytes(include_bytes!("../resources/models/es-chargram-v1.bin"))
        .expect("Spanish model");
    assert_eq!(model.language(), Language::Es);
    assert_eq!(model.feature_profile(), FeatureProfile::EsLegacyWordChar35V1);
    assert_eq!(model.normalization_profile(), NormalizationProfile::EsLegacyCharabiaV1);
    assert_eq!(model.feature_schema(), FeatureSchema::EsLegacyV1);
}

#[test]
fn v2_round_trip_preserves_every_header_field() {
    let weights = vec![0_i16; 65_536];
    let input = fixture_v2_input(Language::Tr, &weights);
    let artifact = encode_sparse_v2(&input).expect("encode");
    assert_eq!(artifact.len(), 131_112);
    let model = SparseModel::from_bytes(&artifact).expect("parse");
    assert_eq!(model.language(), Language::Tr);
    assert_eq!(model.feature_profile(), FeatureProfile::WordChar35V2);
    assert_eq!(model.normalization_profile(), NormalizationProfile::TurkishV2);
    assert_eq!(model.feature_schema(), FeatureSchema::SparseV2);
    assert_eq!(model.raw_score(""), -64);
    assert_eq!(model.raw_boundary(), 128);
    assert_eq!(model.score_scale(), 256);
    assert_eq!(model.max_false_warning_basis_points(), 300);
}

#[test]
fn v2_encoder_rejects_spanish_and_invalid_calibration() {
    let weights = vec![0_i16; 65_536];

    let spanish = fixture_v2_input(Language::Es, &weights);
    assert_eq!(
        encode_sparse_v2(&spanish),
        Err(SparseModelError::VersionTwoSpanish)
    );

    let mut invalid_scale = fixture_v2_input(Language::Tr, &weights);
    invalid_scale.score_scale = 0;
    assert_eq!(
        encode_sparse_v2(&invalid_scale),
        Err(SparseModelError::ZeroScoreScale)
    );

    let mut invalid_limit = fixture_v2_input(Language::Tr, &weights);
    invalid_limit.max_false_warning_basis_points = 10_001;
    assert_eq!(
        encode_sparse_v2(&invalid_limit),
        Err(SparseModelError::InvalidFalseWarningLimit(10_001))
    );
}
```

- [ ] **Step 2: Run the parser tests and confirm the missing V2 API failure**

Run: `cargo test --test sparse_v2`

Expected: FAIL because the version-two API does not exist.

- [ ] **Step 3: Add the fixed V2 header layout**

Import `FeatureProfile`, `FeatureSchema`, `Language`, and `NormalizationProfile` in `src/sparse.rs`.

```rust
const V1_MAGIC: &[u8; 8] = b"TOXSPRS1";
const V2_MAGIC: &[u8; 8] = b"TOXSPRS2";
const V1_FORMAT_VERSION: u16 = 1;
const V2_FORMAT_VERSION: u16 = 2;
const V1_HEADER_LENGTH: usize = 32;
const V2_HEADER_LENGTH: usize = 40;
const BIN_COUNT: usize = 65_536;
const PAYLOAD_LENGTH: usize = BIN_COUNT * size_of::<i16>();
const WEIGHT_SCALE: u16 = 256;

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
    output.extend_from_slice(input.language.code().as_bytes());
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
```

- [ ] **Step 4: Parse each version through separate helpers**

```rust
pub fn from_bytes(bytes: &[u8]) -> Result<Self, SparseModelError> {
    let magic = bytes.get(..8).ok_or(SparseModelError::InvalidLength {
        expected: 8,
        actual: bytes.len(),
    })?;
    if magic == &V1_MAGIC[..] {
        return parse_v1(bytes);
    }
    if magic == &V2_MAGIC[..] {
        return parse_v2(bytes);
    }
    Err(SparseModelError::InvalidMagic)
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
```

Keep the current `read_u16`, `read_u32`, and `read_i32` helpers unchanged.

Replace the current model fields with this typed layout.

```rust
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

impl SparseModel {
    pub const fn language(&self) -> Language {
        self.language
    }

    pub const fn feature_profile(&self) -> FeatureProfile {
        self.feature_profile
    }

    pub const fn normalization_profile(&self) -> NormalizationProfile {
        self.normalization_profile
    }

    pub const fn feature_schema(&self) -> FeatureSchema {
        self.feature_schema
    }

    pub const fn score_scale(&self) -> u32 {
        self.score_scale
    }

    pub const fn max_false_warning_basis_points(&self) -> u16 {
        self.max_false_warning_basis_points
    }
}
```

Keep the public `raw_score` and `raw_boundary` methods from Task 4.

Change `normalized_language` to return `Language`. Pass that value into each model initializer.

```rust
fn normalized_language(language: &str) -> Result<Language, SparseCompileError> {
    let language = language
        .parse()
        .map_err(|_| SparseCompileError::InvalidLanguage)?;
    if language != Language::Es {
        return Err(SparseCompileError::InvalidLanguage);
    }
    Ok(language)
}
```

The legacy `compile_sparse_model` function shall remain Spanish-only. The separate `toxtrain` crate shall call `encode_sparse_v2` for new languages.

Pass the copied `Language` value to `validate_split`, each model initializer, and `encode_artifact`.

Use this comparison in `validate_split`. Change its `expected_language` parameter to `Language`.

```rust
let actual = row.language.trim().to_ascii_uppercase();
if actual != expected_language.code() {
    return Err(SparseCompileError::LanguageMismatch {
        split: name,
        expected: expected_language.code().to_owned(),
        actual,
    });
}
```

Change `encode_artifact` to accept `Language`. Use this header write.

```rust
output.extend_from_slice(language.code().as_bytes());
```

Change the embedded-model assertion and the existing sparse test to compare with `Language::Es`.

Add these variants to the existing `SparseModelError` enum.

```rust
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
```

Export `SparseModel`, `SparseModelError`, `SparseV2Input`, and `encode_sparse_v2` from `src/lib.rs`.

- [ ] **Step 5: Reject every profile mismatch and payload mismatch**

```rust
use std::ops::Range;

type ErrorCheck = fn(&SparseModelError) -> bool;

#[test]
fn v2_rejects_each_invalid_header_field() {
    let weights = vec![0_i16; 65_536];
    let input = fixture_v2_input(Language::Tr, &weights);
    let artifact = encode_sparse_v2(&input).expect("encode");
    let cases: &[(&str, Range<usize>, &[u8], ErrorCheck)] = &[
        ("magic", 0..8, b"BADMAGIC", |error| {
            matches!(error, &SparseModelError::InvalidMagic)
        }),
        ("version", 8..10, &[1, 0], |error| {
            matches!(error, &SparseModelError::UnsupportedVersion(1))
        }),
        ("language", 10..12, b"XX", |error| {
            matches!(error, &SparseModelError::InvalidLanguage)
        }),
        ("lowercase language", 10..12, b"tr", |error| {
            matches!(error, &SparseModelError::InvalidLanguage)
        }),
        ("bin count", 12..16, &[0, 0, 0, 0], |error| {
            matches!(error, &SparseModelError::InvalidBinCount(0))
        }),
        ("score scale", 24..28, &[0, 0, 0, 0], |error| {
            matches!(error, &SparseModelError::ZeroScoreScale)
        }),
        ("false-warning limit", 28..30, &[17, 39], |error| {
            matches!(error, &SparseModelError::InvalidFalseWarningLimit(10_001))
        }),
        ("weight scale", 30..32, &[0, 0], |error| {
            matches!(error, &SparseModelError::InvalidWeightScale(0))
        }),
        ("feature profile", 32..33, &[255], |error| {
            matches!(error, &SparseModelError::InvalidFeatureProfile(255))
        }),
        ("normalization profile", 33..34, &[255], |error| {
            matches!(error, &SparseModelError::InvalidNormalizationProfile(255))
        }),
        ("feature schema", 34..36, &[255, 255], |error| {
            matches!(error, &SparseModelError::InvalidFeatureSchema(65_535))
        }),
        ("payload length", 36..40, &[0, 0, 0, 0], |error| {
            matches!(error, &SparseModelError::InvalidPayloadLength(0))
        }),
        ("language profile", 32..33, &[3], |error| {
            matches!(error, &SparseModelError::ProfileMismatch)
        }),
    ];

    for (name, range, replacement, check) in cases {
        let mut damaged = artifact.clone();
        damaged[range.clone()].copy_from_slice(replacement);
        let error = SparseModel::from_bytes(&damaged).expect_err(name);
        assert!(check(&error), "{name}: {error}");
    }
}

#[test]
fn v2_rejects_spanish_and_nonexact_payload_sizes() {
    let weights = vec![0_i16; 65_536];
    let input = fixture_v2_input(Language::Tr, &weights);
    let artifact = encode_sparse_v2(&input).expect("encode");

    let mut spanish = artifact.clone();
    spanish[10..12].copy_from_slice(b"ES");
    assert_eq!(
        SparseModel::from_bytes(&spanish),
        Err(SparseModelError::VersionTwoSpanish)
    );

    let truncated = &artifact[..artifact.len() - 1];
    assert!(matches!(
        SparseModel::from_bytes(truncated),
        Err(SparseModelError::InvalidLength { expected: 131_112, actual: 131_111 })
    ));

    let mut extended = artifact;
    extended.push(0);
    assert!(matches!(
        SparseModel::from_bytes(&extended),
        Err(SparseModelError::InvalidLength { expected: 131_112, actual: 131_113 })
    ));
}
```

- [ ] **Step 6: Run all sparse and Spanish tests**

Run: `cargo test --test sparse --test sparse_v2 --test spanish_compatibility`

Expected: PASS.

### Task 6: Publish runtime metadata for later plans

**Files:**

- Create: `src/registry.rs`
- Modify: `src/lib.rs`
- Modify: `tests/profile_contract.rs`

**Interfaces:**

- Consumes: `Language::ALL` and each language profile tuple.
- Produces: `LanguageSpec` and `language_spec(Language) -> &'static LanguageSpec`.

- [ ] **Step 1: Add a failing registry metadata test**

```rust
use toxcheck::{Language, language_spec};

#[test]
fn every_language_has_one_immutable_runtime_spec() {
    for language in Language::ALL {
        let spec = language_spec(language);
        assert_eq!(spec.language, language);
        assert_eq!(
            (spec.feature_profile, spec.normalization_profile, spec.feature_schema),
            language.profiles()
        );
    }
}
```

- [ ] **Step 2: Run the registry test and confirm the missing function failure**

Run: `cargo test --test profile_contract every_language_has_one_immutable_runtime_spec`

Expected: FAIL because `language_spec` does not exist.

- [ ] **Step 3: Add immutable metadata without new artifacts**

```rust
use crate::{FeatureProfile, FeatureSchema, Language, NormalizationProfile};

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

static LANGUAGE_SPECS: [LanguageSpec; 15] = [
    LanguageSpec::new(Language::En),
    LanguageSpec::new(Language::Zh),
    LanguageSpec::new(Language::Es),
    LanguageSpec::new(Language::Ar),
    LanguageSpec::new(Language::Id),
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

pub fn language_spec(language: Language) -> &'static LanguageSpec {
    &LANGUAGE_SPECS[language.index()]
}
```

Export `language_spec` and `LanguageSpec` from `src/lib.rs`.

This task shall not add fake model bytes. The model plan shall extend this metadata after it generates all 14 artifacts.

- [ ] **Step 4: Run the runtime foundation test set**

Run: `cargo test --test spanish_compatibility --test profile_contract --test sparse --test sparse_v2`

Expected: PASS.

- [ ] **Step 5: Run formatting and Clippy**

Run: `cargo fmt --check`

Expected: PASS.

Run: `cargo clippy --all-targets -- -D warnings`

Expected: PASS.
