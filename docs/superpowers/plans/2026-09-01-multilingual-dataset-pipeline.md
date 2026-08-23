# Multilingual dataset pipeline implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a reproducible offline Rust pipeline that acquires, converts, splits, and publishes training data for 14 new languages.

**Architecture:** Move all network and dataset code into a separate `toxtrain` workspace crate.

Convert each source through one typed adapter.

Publish prepared rows and provenance through atomic no-replace directories.

**Tech stack:** Rust 2024, Reqwest blocking, CSV, Serde JSON, SHA-256, Rustix atomic rename, and deterministic FNV-1a splitting.

**Spec:** `docs/superpowers/specs/2026-09-01-multilingual-sparse-nudge-detector-design.md`

## Global constraints

Complete `2026-09-01-multilingual-runtime-foundation.md` first.

Use the shared `toxcheck::Language` type. Do not define another language enum.

Keep every file under `data/textdetox/es-prepared` unchanged.

Keep `resources/models/es-chargram-v1.bin` unchanged.

Preserve source text exactly. Use normalized text only for grouping and split hashing.

The project directory is not a Git repository. Each task ends with a verification checkpoint instead of a commit.

---

## File structure

- Create `crates/toxtrain` as the offline package and binary.
- Move acquisition, preparation, evaluation parsing, and atomic publication from the runtime package.
- Create `crates/toxtrain/src/datasets` for six source adapters and shared preparation.
- Create `resources/datasets/source-catalog-v1.json` for source requests.
- Generate an observed source manifest before freezing `resources/datasets/source-lock-v1.json`.
- Create focused crate tests under `crates/toxtrain/tests`.

### Task 1: Split the runtime and offline packages

**Files:**

- Modify: `Cargo.toml`
- Modify through Cargo resolution: `Cargo.lock`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `src/evaluation.rs`
- Create: `crates/toxtrain/Cargo.toml`
- Create: `crates/toxtrain/src/lib.rs`
- Create: `crates/toxtrain/src/main.rs`
- Create: `crates/toxtrain/src/evaluation.rs`
- Create: `crates/toxtrain/src/compiler.rs`
- Create: `crates/toxtrain/src/datasets/mod.rs`
- Move: `src/textdetox.rs` to `crates/toxtrain/src/datasets/textdetox.rs`
- Move: `src/textdetox_acquisition.rs` to `crates/toxtrain/src/acquisition.rs`
- Move: `src/textdetox_publication.rs` to `crates/toxtrain/src/publication.rs`
- Move: `src/atomic_publish.rs` to `crates/toxtrain/src/atomic_publish.rs`
- Move: `tests/textdetox.rs` to `crates/toxtrain/tests/textdetox.rs`
- Move: `tests/compile_sparse_cli.rs` to `crates/toxtrain/tests/compiler.rs`
- Modify: `tests/sparse.rs`
- Modify: `tests/evaluation.rs`
- Create: `tests/toxcheck_cli.rs`
- Remove after split: `tests/cli.rs`
- Create: `crates/toxtrain/tests/evaluation.rs`
- Create: `crates/toxtrain/tests/cli.rs`
- Remove after migration: `src/bin/compile_sparse.rs`

**Interfaces:**

- Consumes: The runtime crate and shared `Language` type.
- Produces: A network-free `toxcheck` package and an offline `toxtrain` package.

- [ ] **Step 1: Add a failing workspace package check**

Run: `cargo check -p toxtrain`

Expected: FAIL because package `toxtrain` does not exist.

- [ ] **Step 2: Add the workspace definition**

```toml
[workspace]
members = [".", "crates/toxtrain"]
default-members = [".", "crates/toxtrain"]
resolver = "2"
```

- [ ] **Step 3: Create the offline package manifest**

```toml
[package]
name = "toxtrain"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
publish = false

[dependencies]
toxcheck = { path = "../.." }
anyhow = "1.0"
clap = { version = "4.5", features = ["derive"] }
csv = "1.3"
reqwest = { version = "0.12", default-features = false, features = ["blocking", "rustls-tls"] }
rustix = { version = "1.1", features = ["fs"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sha2 = "0.10"
thiserror = "2.0"
zip = { version = "=5.1.1", default-features = false, features = ["deflate"] }

[dev-dependencies]
tempfile = "3.20"
```

Let Cargo update `Cargo.lock` from manifest resolution.

Keep zip on 5.1.1. Its documented Rust 1.83 MSRV is below the project Rust 1.85 floor.

Do not edit `Cargo.lock` by hand.

- [ ] **Step 4: Move offline modules into the new crate**

Use these new module paths.

```rust
pub mod acquisition;
pub mod atomic_publish;
pub mod compiler;
pub mod datasets;
pub mod evaluation;
pub mod publication;
pub use datasets::textdetox::*;
```

Create `crates/toxtrain/src/datasets/mod.rs` with this content.

```rust
pub mod textdetox;
```

Keep `EvalLabel`, `EvalRow`, `ConfusionMatrix`, and `Metrics` in `toxcheck`.

Add `Serialize` and `Deserialize` to `EvalLabel`.

Use `#[serde(rename_all = "lowercase")]` for the `clean` and `toxic` values.

Do not add offline copies of `ConfusionMatrix` or `Metrics`.

Require the runtime foundation to serialize `Language` with its uppercase detector code.

- [ ] **Step 5: Preserve every migrated test**

Keep each current test name and assertion during the move.

Move the runtime `check` tests into `tests/toxcheck_cli.rs`.

Move the `setup`, `eval`, `fetch-textdetox`, and `prepare-textdetox` tests into `crates/toxtrain/tests/cli.rs`.

Move only evaluation-parser tests into `crates/toxtrain/tests/evaluation.rs`.

Keep the metric tests in `tests/evaluation.rs`.

Move the current compile wrapper into `crates/toxtrain/src/compiler.rs`.

Move the compile CLI test into `crates/toxtrain/tests/compiler.rs`.

Move all four current `compile_sparse_model` tests from `tests/sparse.rs` into `crates/toxtrain/tests/compiler.rs`.

Keep `sparse_model_rejects_a_truncated_artifact` in the runtime `tests/sparse.rs` file.

The migrated compiler can call `toxcheck::compile_sparse_model` until the model plan moves that function.

Run: `cargo test -p toxcheck --test toxcheck_cli --test evaluation --test sparse`

Expected: PASS.

Run: `cargo test -p toxtrain --test textdetox --test cli --test evaluation --test compiler`

Expected: PASS.

- [ ] **Step 6: Keep only `check` in the shipping CLI**

```rust
#[derive(Debug, Subcommand)]
enum Command {
    Check(CheckArgs),
}
```

Move `setup`, `eval`, `fetch-textdetox`, `prepare-textdetox`, and `compile` into `toxtrain`.

- [ ] **Step 7: Remove offline dependencies from the runtime package**

Remove `reqwest`, `rustix`, and `serde_json` from the root dependencies after the moved code compiles.

Keep `csv` and `serde` in the runtime package for HurtLex parsing.

Add `serde_json` to the root development dependencies for the profile contract tests.

- [ ] **Step 8: Run both package checks**

Run: `cargo check -p toxcheck`

Expected: PASS.

Run: `cargo check -p toxtrain`

Expected: PASS.

### Task 2: Add common dataset and source-lock types

**Files:**

- Modify: `crates/toxtrain/src/lib.rs`
- Modify: `crates/toxtrain/src/datasets/mod.rs`
- Create: `crates/toxtrain/src/datasets/types.rs`
- Create: `crates/toxtrain/src/evidence.rs`
- Create: `crates/toxtrain/src/source_manifest.rs`
- Create: `resources/datasets/source-catalog-v1.json`
- Create after source review: `resources/datasets/source-lock-v1.json`
- Create: `crates/toxtrain/tests/source_manifest.rs`

**Interfaces:**

- Consumes: `toxcheck::{EvalLabel, Language}`.
- Produces: Typed rows, split policies, source observations, a frozen source lock, and provenance rows.

- [ ] **Step 1: Write source-lock and type parsing tests**

```rust
#[test]
fn frozen_source_lock_rejects_a_missing_hash_and_unknown_dataset() {
    let mut missing_hash = valid_frozen_lock_json();
    missing_hash["sources"][0]
        .as_object_mut()
        .expect("source object")
        .remove("file_sha256");
    let missing_hash = serde_json::to_vec(&missing_hash).expect("JSON");
    assert!(parse_frozen_source_lock(&missing_hash[..]).is_err());

    let mut unknown = valid_frozen_lock_json();
    unknown["sources"][0]["dataset"] = serde_json::json!("other");
    let unknown = serde_json::to_vec(&unknown).expect("JSON");
    assert!(parse_frozen_source_lock(&unknown[..]).is_err());
}

#[test]
fn source_observation_and_frozen_lock_are_distinct_schemas() {
    let observation = valid_source_observation_json();
    let bytes = serde_json::to_vec(&observation).expect("JSON");
    assert!(parse_frozen_source_lock(&bytes[..]).is_err());
}
```

The test helpers shall supply every required field and a 64-character lowercase hexadecimal test hash.

- [ ] **Step 2: Run the source-lock test and confirm the missing module failure**

Run: `cargo test -p toxtrain --test source_manifest`

Expected: FAIL because the source manifest module does not exist.

- [ ] **Step 3: Add the dataset state types**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DatasetId {
    #[serde(rename = "hurtlex")]
    HurtLex,
    #[serde(rename = "textdetox")]
    TextDetox,
    #[serde(rename = "ibrohim-budi")]
    IbrohimBudi,
    #[serde(rename = "told-br")]
    ToldBr,
    #[serde(rename = "offenseval-tr")]
    OffensEvalTr,
    #[serde(rename = "vihos")]
    ViHos,
    #[serde(rename = "k-mhas")]
    KMHas,
}

impl std::fmt::Display for DatasetId {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(match self {
            Self::HurtLex => "hurtlex",
            Self::TextDetox => "textdetox",
            Self::IbrohimBudi => "ibrohim-budi",
            Self::ToldBr => "told-br",
            Self::OffensEvalTr => "offenseval-tr",
            Self::ViHos => "vihos",
            Self::KMHas => "k-mhas",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSplit { Unsplit, Train, Development, Validation, Test }

impl std::fmt::Display for SourceSplit {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(match self {
            Self::Unsplit => "unsplit",
            Self::Train => "train",
            Self::Development => "development",
            Self::Validation => "validation",
            Self::Test => "test",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit { Development, Validation, Test }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitPolicy { Hash70_15_15, TurkishOfficialTest, PreserveOfficial }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageStatus { Resolved, Unresolved }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InclusionStatus { Included, Excluded }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    AmbiguousLabel, AuditOnly, Duplicate, EmptyText, LabelConflict, UnsupportedLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowDisposition {
    Candidate(EvalLabel),
    Excluded(ExclusionReason),
}
```

- [ ] **Step 4: Add the common adapter interface**

```rust
pub struct SourceInput<'a> {
    pub source_file_id: &'a str,
    pub source_split: SourceSplit,
    pub reader: &'a mut dyn std::io::Read,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedRow {
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub source_id: String,
    pub source_language_code: String,
    pub detector_language: Option<Language>,
    pub detector_language_code: Option<String>,
    pub source_label: String,
    pub text: String,
    pub source_split: SourceSplit,
    pub disposition: RowDisposition,
}

pub trait DatasetAdapter {
    fn dataset_id(&self) -> DatasetId;
    fn label_conversion_version(&self) -> &'static str;
    fn import(&self, inputs: &mut [SourceInput<'_>]) -> Result<Vec<ImportedRow>, ImportError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("cannot parse CSV data: {0}")]
    Csv(#[from] csv::Error),
    #[error("cannot parse JSON data: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot read source data: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing required column: {0}")]
    MissingColumn(&'static str),
    #[error("invalid binary label for source row: {0}")]
    InvalidBinaryLabel(String),
    #[error("missing joined label for source row: {0}")]
    MissingJoinedLabel(String),
    #[error("unused joined label for source row: {0}")]
    UnusedJoinedLabel(String),
    #[error("invalid harmful span for source row: {0}")]
    InvalidSpan(String),
    #[error("invalid Korean label set for source row: {0}")]
    InvalidKoreanLabel(String),
    #[error("invalid source row: {0}")]
    InvalidSource(String),
}
```

- [ ] **Step 5: Add observed and frozen source types**

Add these declarations to `crates/toxtrain/src/lib.rs`.

```rust
pub mod evidence;
pub mod source_manifest;
```

Add these declarations to `crates/toxtrain/src/datasets/mod.rs`.

```rust
mod types;
pub use types::*;
```

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRequest {
    pub dataset: DatasetId,
    pub detector_language: Language,
    pub source_file_id: String,
    pub requested_url: String,
    pub revision_url: Option<String>,
    pub requested_revision: Option<String>,
    pub archive_member: Option<String>,
    pub file_path: String,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenSource {
    pub dataset: DatasetId,
    pub detector_language: Language,
    pub source_file_id: String,
    pub immutable_source_url: String,
    pub archive_member: Option<String>,
    pub revision: Option<String>,
    pub file_path: String,
    pub file_sha256: Sha256Digest,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub dataset: DatasetId,
    pub detector_language: Language,
    pub source_file_id: String,
    pub immutable_source_url: String,
    pub archive_member: Option<String>,
    pub revision: Option<String>,
    pub file_path: String,
    pub file_sha256: Sha256Digest,
    pub acquired_at_unix_seconds: u64,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCatalog {
    pub schema_version: String,
    pub sources: Vec<SourceRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceObservation {
    pub schema_version: String,
    pub sources: Vec<SourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenSourceLock {
    pub schema_version: String,
    pub sources: Vec<FrozenSource>,
}
```

Define `Sha256Digest` in `crates/toxtrain/src/evidence.rs`.

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Digest(String);
```

Export `Sha256Digest` from `toxtrain::evidence`.

Import `crate::evidence::Sha256Digest` in `source_manifest.rs` and the dataset types.

Do not define another digest type in the offline crate.

Implement `TryFrom<String>`, `Display`, and `as_str` for `Sha256Digest`.

Implement `From<Sha256Digest> for String` for Serde output.

Reject a digest unless it has exactly 64 lowercase hexadecimal characters.

Stage one reads `source-catalog-v1.json` and writes `source-observation-v1.json`.

Stage one records each actual immutable URL, revision, hash, and acquisition time.

Stage one does not create or change the frozen source lock.

Stage two validates the observation after human review.

Stage two creates `source-lock-v1.json` with no overwrite.

Stage two removes acquisition times and keeps every observed source identity unchanged.

Frozen acquisition downloads each source again and verifies its revision and hash.

Frozen acquisition writes a new observed manifest beside the raw files.

Require exact schema values `source-catalog-v1`, `source-observation-v1`, and `source-lock-v1`.

Use these exact dataset entries in `source-catalog-v1.json`.

Set `detector_language` to the uppercase language identified by each catalog row.

Map Ibrohim-Budi to `ID`, ToLD-Br to `PT`, OffensEval to `TR`, ViHOS to `VI`, and K-MHaS to `KO`.

Map each `textdetox-*` entry to `DatasetId::TextDetox`.

Map the ID, PT, TR, VI, and KO entries to their matching `DatasetId` variants.

| `source_file_id` | Requested URL | Requested revision | Archive member |
|---|---|---|---|
| `textdetox-en` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=en&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-zh` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=zh&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-ar` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=ar&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-fr` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=fr&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-hi` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=hi&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-ru` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=ru&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-ja` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=ja&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-de` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=de&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `textdetox-it` | `https://datasets-server.huggingface.co/rows?dataset=textdetox%2Fmultilingual_toxicity_dataset&config=default&split=it&offset={offset}&length={length}` | `01907546324b0330d2d8b7669648cc18823323e5` | None |
| `ibrohim-budi-re-dataset` | `https://raw.githubusercontent.com/okkyibrohim/id-multi-label-hate-speech-and-abusive-language-detection/be98de98e974b65838d2b5145ee2c89e9bf53a6b/re_dataset.csv` | `be98de98e974b65838d2b5145ee2c89e9bf53a6b` | None |
| `told-br-alpha` | `https://raw.githubusercontent.com/joaoaleite/ToLD-Br/6b325d26a9d25b321a3e9ba98ef98832b56729f5/ToLD-BR_alpha.csv` | `6b325d26a9d25b321a3e9ba98ef98832b56729f5` | None |
| `offenseval-tr-training` | `https://coltekin.github.io/offensive-turkish/offenseval2020-turkish.zip` | None | `offenseval2020-turkish/offenseval-tr-training-v1/offenseval-tr-training-v1.tsv` |
| `offenseval-tr-test` | `https://coltekin.github.io/offensive-turkish/offenseval2020-turkish.zip` | None | `offenseval2020-turkish/offenseval-tr-testset-v1/offenseval-tr-testset-v1.tsv` |
| `offenseval-tr-test-labels` | `https://coltekin.github.io/offensive-turkish/offenseval2020-turkish.zip` | None | `offenseval2020-turkish/offenseval-tr-testset-v1/offenseval-tr-labela-v1.tsv` |
| `vihos-train` | `https://raw.githubusercontent.com/phusroyal/ViHOS/fe31c4b304650d62bb0cb668e2fb2060fc6f98fd/data/Span_Extraction_based_version/train.csv` | `fe31c4b304650d62bb0cb668e2fb2060fc6f98fd` | None |
| `vihos-development` | `https://raw.githubusercontent.com/phusroyal/ViHOS/fe31c4b304650d62bb0cb668e2fb2060fc6f98fd/data/Span_Extraction_based_version/dev.csv` | `fe31c4b304650d62bb0cb668e2fb2060fc6f98fd` | None |
| `vihos-test` | `https://raw.githubusercontent.com/phusroyal/ViHOS/fe31c4b304650d62bb0cb668e2fb2060fc6f98fd/data/Test_data/test.csv` | `fe31c4b304650d62bb0cb668e2fb2060fc6f98fd` | None |
| `kmhas-train` | `https://raw.githubusercontent.com/adlnlp/K-MHaS/ec7a7e775d650b825872f6f538fc717822cdfc1a/data/kmhas_train.txt` | `ec7a7e775d650b825872f6f538fc717822cdfc1a` | None |
| `kmhas-validation` | `https://raw.githubusercontent.com/adlnlp/K-MHaS/ec7a7e775d650b825872f6f538fc717822cdfc1a/data/kmhas_valid.txt` | `ec7a7e775d650b825872f6f538fc717822cdfc1a` | None |
| `kmhas-test` | `https://raw.githubusercontent.com/adlnlp/K-MHaS/ec7a7e775d650b825872f6f538fc717822cdfc1a/data/kmhas_test.txt` | `ec7a7e775d650b825872f6f538fc717822cdfc1a` | None |

Use `https://huggingface.co/api/datasets/textdetox/multilingual_toxicity_dataset/revision/main` as each TextDetox `revision_url`.

Use these exact HurtLex entries in the same catalog.

Map every `hurtlex-*` entry to `DatasetId::HurtLex`.

| `source_file_id` | Immutable URL | Requested revision |
|---|---|---|
| `hurtlex-en-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/EN/1.2/hurtlex_EN.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-zh-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/ZH/1.2/hurtlex_ZH.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-es-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/ES/1.2/hurtlex_ES.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-ar-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/AR/1.2/hurtlex_AR.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-id-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/ID/1.2/hurtlex_ID.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-pt-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/PT/1.2/hurtlex_PT.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-fr-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/FR/1.2/hurtlex_FR.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-hi-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/HI/1.2/hurtlex_HI.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-ru-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/RU/1.2/hurtlex_RU.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-ja-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/JA/1.2/hurtlex_JA.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-de-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/DE/1.2/hurtlex_DE.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-tr-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/TR/1.2/hurtlex_TR.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-vi-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/VI/1.2/hurtlex_VI.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-ko-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/KO/1.2/hurtlex_KO.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |
| `hurtlex-it-1.2` | `https://raw.githubusercontent.com/valeriobasile/hurtlex/d4d5cf1199c09868486f978fcea58af0e8936a1e/lexica/IT/1.2/hurtlex_IT.tsv` | `d4d5cf1199c09868486f978fcea58af0e8936a1e` |

Set every GitHub raw URL as the immutable source URL in its observation.

Set the OffensEval URL plus archive member as one immutable file identity.

Record the TextDetox rows URL template and the observed revision in each observation.

Store TextDetox files as `textdetox/{code}.tsv` under the raw root.

Store the other dataset files under `datasets/{source_file_id}/`.

Store HurtLex files as `hurtlex/{CODE}/1.2/hurtlex_{CODE}.tsv`.

- [ ] **Step 6: Add the complete provenance row**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRow {
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub source_id: String,
    pub immutable_source_url: String,
    pub archive_member: Option<String>,
    pub revision: Option<String>,
    pub file_path: String,
    pub file_sha256: Sha256Digest,
    pub acquired_at_unix_seconds: u64,
    pub license_id: String,
    pub license_url: String,
    pub citation: String,
    pub upstream_lineage: Vec<String>,
    pub lineage_status: LineageStatus,
    pub source_language_code: String,
    pub detector_language_code: Option<String>,
    pub source_label: String,
    pub detector_label: Option<EvalLabel>,
    pub label_conversion_version: String,
    pub split_version: String,
    pub normalization_version: String,
    pub canonical_group_id: Option<String>,
    pub representative_source_id: Option<String>,
    pub source_split: SourceSplit,
    pub detector_split: Option<DatasetSplit>,
    pub inclusion_status: InclusionStatus,
    pub exclusion_reason: Option<ExclusionReason>,
}
```

Create exactly one provenance row for each imported source row.

Require an included row to have a detector label, detector split, and representative source identifier.

Require an excluded row to have an exclusion reason.

- [ ] **Step 7: Add source identifier formatting**

```rust
pub fn source_id(
    dataset: DatasetId,
    revision_or_hash: &str,
    split: SourceSplit,
    native_id: &str,
) -> String {
    format!("{dataset}@{revision_or_hash}/{split}/{native_id}")
}
```

- [ ] **Step 8: Run the source manifest tests**

Run: `cargo test -p toxtrain --test source_manifest`

Expected: PASS.

### Task 3: Implement TextDetox, Indonesian, and Portuguese adapters

**Files:**

- Modify: `crates/toxtrain/src/datasets/textdetox.rs`
- Create: `crates/toxtrain/src/datasets/ibrohim_budi.rs`
- Create: `crates/toxtrain/src/datasets/told_br.rs`
- Create: `crates/toxtrain/tests/dataset_adapters.rs`
- Create: `crates/toxtrain/tests/fixtures/textdetox.json`
- Create: `crates/toxtrain/tests/fixtures/ibrohim_budi.csv`
- Create: `crates/toxtrain/tests/fixtures/told_br_alpha.csv`

**Interfaces:**

- Consumes: Raw source files and common `DatasetAdapter` types.
- Produces: `ImportedRow` values with exact label conversions.

- [ ] **Step 1: Write exact conversion tests**

```rust
#[test]
fn indonesian_uses_hate_or_abusive() {
    let csv = "Tweet,HS,Abusive\nhello,0,0\nhate,1,0\ncurse,0,1\n";
    let rows = import_indonesian(csv.as_bytes()).expect("import");
    assert_eq!(labels(&rows), [Clean, Toxic, Toxic]);
}

#[test]
fn portuguese_counts_toxic_annotators_across_categories() {
    let rows = import_told_br(include_bytes!("fixtures/told_br_alpha.csv")).expect("import");
    assert_eq!(dispositions(&rows), [Clean, Toxic, AmbiguousLabel]);
}
```

- [ ] **Step 2: Run the adapter tests and confirm missing adapters**

Run: `cargo test -p toxtrain --test dataset_adapters`

Expected: FAIL because the adapter functions do not exist.

- [ ] **Step 3: Migrate the pinned TextDetox parser**

Keep the current revision checks before, during, and after acquisition.

```rust
pub const TEXTDETOX_REVISION: &str = "01907546324b0330d2d8b7669648cc18823323e5";
pub const TEXTDETOX_CODES: &[&str] = &["en", "zh", "ar", "fr", "hi", "ru", "ja", "de", "it"];
```

Reject `hin` when the requested detector language is `HI`.

Store the raw `0` or `1` value in `source_label`.

- [ ] **Step 4: Implement Indonesian conversion**

```rust
let disposition = match (row.hs, row.abusive) {
    (0, 0) => RowDisposition::Candidate(EvalLabel::Clean),
    (0 | 1, 0 | 1) => RowDisposition::Candidate(EvalLabel::Toxic),
    _ => return Err(ImportError::InvalidBinaryLabel(row.source_id)),
};
```

Require this exact Indonesian header.

```text
Tweet,HS,Abusive,HS_Individual,HS_Group,HS_Religion,HS_Race,HS_Physical,HS_Gender,HS_Other,HS_Weak,HS_Moderate,HS_Strong
```

Store `HS=<value>;Abusive=<value>` in `source_label`.

- [ ] **Step 5: Implement Portuguese annotator consensus**

Use these six category stems for annotators one through three.

```rust
const TOLD_BR_HEADER: [&str; 22] = [
    "text",
    "homophobia_1", "homophobia_2", "homophobia_3",
    "obscene_1", "obscene_2", "obscene_3",
    "insult_1", "insult_2", "insult_3",
    "racism_1", "racism_2", "racism_3",
    "misogyny_1", "misogyny_2", "misogyny_3",
    "xenophobia_1", "xenophobia_2", "xenophobia_3",
    "obs_1", "obs_2", "obs_3",
];

const CATEGORIES: [&str; 6] = [
    "homophobia", "obscene", "insult", "racism", "misogyny", "xenophobia",
];

let toxic_votes = (1..=3)
    .filter(|annotator| CATEGORIES.iter().any(|category| row.value(category, *annotator) == 1.0))
    .count();
let disposition = match toxic_votes {
    0 => RowDisposition::Candidate(EvalLabel::Clean),
    1 => RowDisposition::Excluded(ExclusionReason::AmbiguousLabel),
    2 | 3 => RowDisposition::Candidate(EvalLabel::Toxic),
    _ => return Err(ImportError::InvalidSource(row.source_id)),
};
```

Require the 22 raw columns in this exact order.

Accept only `0.0` and `1.0` in the 18 category fields.

Ignore the three `obs` fields during label conversion.

Store `toxic_annotator_votes=<count>` in `source_label`.

- [ ] **Step 6: Run all three adapter tests**

Run: `cargo test -p toxtrain --test dataset_adapters`

Expected: PASS.

### Task 4: Implement Turkish, Vietnamese, and Korean adapters

**Files:**

- Create: `crates/toxtrain/src/datasets/offenseval_tr.rs`
- Create: `crates/toxtrain/src/datasets/vihos.rs`
- Create: `crates/toxtrain/src/datasets/kmhas.rs`
- Modify: `crates/toxtrain/tests/dataset_adapters.rs`
- Create: `crates/toxtrain/tests/fixtures/offenseval_tr/offenseval-tr-training-v1.tsv`
- Create: `crates/toxtrain/tests/fixtures/offenseval_tr/offenseval-tr-testset-v1.tsv`
- Create: `crates/toxtrain/tests/fixtures/offenseval_tr/offenseval-tr-labela-v1.tsv`
- Create: `crates/toxtrain/tests/fixtures/vihos/train.csv`
- Create: `crates/toxtrain/tests/fixtures/vihos/dev.csv`
- Create: `crates/toxtrain/tests/fixtures/vihos/test.csv`
- Create: `crates/toxtrain/tests/fixtures/kmhas/kmhas_train.txt`
- Create: `crates/toxtrain/tests/fixtures/kmhas/kmhas_valid.txt`
- Create: `crates/toxtrain/tests/fixtures/kmhas/kmhas_test.txt`

**Interfaces:**

- Consumes: Official source splits.
- Produces: Typed rows that preserve official test assignments.

- [ ] **Step 1: Write exact label and malformed-row tests**

```rust
#[test]
fn korean_requires_eight_alone_for_clean() {
    assert_eq!(kmhas_label(&[8]).unwrap(), Candidate(Clean));
    assert_eq!(kmhas_label(&[0, 7]).unwrap(), Candidate(Toxic));
    assert!(kmhas_label(&[8, 2]).is_err());
    assert!(kmhas_label(&[]).is_err());
}

#[test]
fn vietnamese_rejects_invalid_spans() {
    assert_eq!(vihos_label("[]", 10).unwrap(), Candidate(Clean));
    assert_eq!(vihos_label("[1,2]", 10).unwrap(), Candidate(Toxic));
    assert!(vihos_label("[12]", 10).is_err());
}
```

- [ ] **Step 2: Run the adapter tests and confirm missing functions**

Run: `cargo test -p toxtrain --test dataset_adapters`

Expected: FAIL.

- [ ] **Step 3: Join Turkish test rows and labels by ID**

```rust
let labels = read_turkish_labels(label_reader)?;
for row in read_turkish_texts(text_reader)? {
    let label = labels.get(&row.id).ok_or_else(|| ImportError::MissingJoinedLabel(row.id.clone()))?;
    output.push(convert_turkish(row, *label, SourceSplit::Test)?);
}
ensure_no_unused_labels(labels, &output)?;
```

Parse the training and test text files as tab-separated files.

Parse the test label file as headerless comma-separated `id,label` records.

Store the exact `OFF` or `NOT` value in `source_label`.

- [ ] **Step 4: Parse Vietnamese spans as an integer array**

Require every span index to reference a valid Unicode scalar position in `content`.

Require the exact ViHOS header `,content,index_spans` for all three official splits.

Store `has-span` or `no-span` in `source_label`.

- [ ] **Step 5: Parse Korean multi-label sets**

Require a nonempty intersection with `{0,1,2,3,4,5,6,7}` for toxic rows.

Reject every set that combines `8` with another label.

Require the exact tab-separated K-MHaS header `document\tlabel`.

Store the sorted comma-separated label set in `source_label`.

- [ ] **Step 6: Run the full adapter test file**

Run: `cargo test -p toxtrain --test dataset_adapters`

Expected: PASS.

### Task 5: Generalize grouping, splitting, and representative selection

**Files:**

- Create: `crates/toxtrain/src/datasets/prepare.rs`
- Modify: `crates/toxtrain/src/datasets/mod.rs`
- Create: `crates/toxtrain/tests/preparation.rs`
- Modify: `crates/toxtrain/tests/textdetox.rs`

**Interfaces:**

- Consumes: `Vec<ImportedRow>` and one `PreparationPolicy`.
- Produces: `PreparedLanguage`, `PreparedRow`, `PreparedCounts`, and one provenance row per source row.

- [ ] **Step 1: Write locked hash and duplicate tests**

```rust
#[test]
fn hash_policies_use_the_exact_uppercase_byte_contract() {
    assert_eq!(split_for_key(Language::En, "you are an idiot"), Development);
    assert_eq!(split_for_key(Language::En, "message 1"), Validation);
    assert_eq!(split_for_key(Language::En, "message 14"), Test);
}

#[test]
fn highest_split_and_smallest_source_id_select_the_representative() {
    let prepared = prepare_language(cross_split_duplicates(), &preserve_official()).unwrap();
    assert_eq!(prepared.test[0].source_id, "textdetox@v1/test/001");
    assert_eq!(prepared.counts.duplicates, 2);
    assert_eq!(
        prepared.provenance.iter().filter(|row| {
            row.exclusion_reason == Some(ExclusionReason::Duplicate)
        }).count(),
        2,
    );
}
```

Add one table test for development, validation, test, unknown, and duplicate audit identifiers.

Require only the development case to produce `ExclusionReason::AuditOnly`.

- [ ] **Step 2: Run preparation tests and confirm the missing engine failure**

Run: `cargo test -p toxtrain --test preparation`

Expected: FAIL.

- [ ] **Step 3: Add the prepared output types**

```rust
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedRow {
    pub detector_language: Language,
    pub label: EvalLabel,
    pub source_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedCounts {
    pub development: usize,
    pub validation: usize,
    pub test: usize,
    pub duplicates: usize,
    pub conflicts: usize,
    pub excluded: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedLanguage {
    pub language: Language,
    pub development: Vec<PreparedRow>,
    pub validation: Vec<PreparedRow>,
    pub test: Vec<PreparedRow>,
    pub provenance: Vec<ProvenanceRow>,
    pub counts: PreparedCounts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationPolicy {
    pub language: Language,
    pub split_policy: SplitPolicy,
    pub split_version: &'static str,
    pub normalization_version: &'static str,
    pub audit_only_source_ids: BTreeSet<String>,
}
```

Export `PreparedRow`, `PreparedCounts`, `PreparedLanguage`, and `PreparationPolicy` from `toxtrain::datasets`.

Use `BTreeSet::new()` in every existing policy fixture without audit exclusions.

Count every included row in exactly one split field.

Count every excluded provenance row in `excluded`.

Treat `duplicates` and `conflicts` as subsets of `excluded`.

Mark every listed development source identifier as `ExclusionReason::AuditOnly` before grouping.

Reject an unknown audit identifier or an identifier assigned to validation or test.

Count duplicate and conflict source rows. Do not count groups in these fields.

Reject an imported candidate when its detector language differs from `PreparationPolicy::language`.

- [ ] **Step 4: Freeze split normalization and FNV-1a**

```rust
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub fn split_hash(language: Language, normalized: &str) -> u64 {
    let bytes = language
        .code()
        .bytes()
        .chain(std::iter::once(0))
        .chain(normalized.bytes());
    bytes.fold(FNV_OFFSET, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME))
}
```

Use `Language::code()` because the runtime foundation returns uppercase detector codes.

- [ ] **Step 5: Implement the three split policies**

Use `0..=69`, `70..=84`, and `85..=99` for unsplit sources.

Use `0..=84` and `85..=99` for Turkish training rows.

Preserve Vietnamese and Korean official splits.

- [ ] **Step 6: Implement global conflict and duplicate selection**

Group by uppercase language and split-normalized text.

Exclude every conflicting-label group before representative selection.

Choose `test > validation > development`, then the smallest source identifier.

- [ ] **Step 7: Preserve the migrated TextDetox tests**

Keep all 18 current TextDetox test names and assertions.

Move the split, grouping, conflict, filter, precedence, and sorting tests into `preparation.rs`.

Keep the source-language, parser, page, source-TSV, and source-identifier tests in `textdetox.rs`.

Move the evaluation TSV round-trip test into `crates/toxtrain/tests/evaluation.rs`.

Move the provenance TSV test into `crates/toxtrain/tests/provenance.rs` during Task 6.

- [ ] **Step 8: Run preparation tests**

Run: `cargo test -p toxtrain --test preparation`

Expected: PASS.

### Task 6: Publish provenance and prepared data atomically

**Files:**

- Modify: `crates/toxtrain/src/publication.rs`
- Modify: `crates/toxtrain/src/atomic_publish.rs`
- Modify: `crates/toxtrain/src/datasets/types.rs`
- Create: `crates/toxtrain/tests/provenance.rs`
- Create: `crates/toxtrain/tests/publication.rs`

**Interfaces:**

- Consumes: Prepared languages and the frozen acquisition observation.
- Produces: One complete prepared directory through no-replace publication.

- [ ] **Step 1: Write full provenance and collision tests**

```rust
#[test]
fn publication_writes_one_provenance_row_per_source_row() {
    let output = publish_fixture().expect("publish");
    let source_count = output.manifest.source_rows;
    let provenance_count = read_provenance(output.path.join("provenance.tsv")).unwrap().len();
    assert_eq!(provenance_count, source_count);
}

#[test]
fn existing_destination_survives_failed_publication() {
    let directory = fixture_with_existing_destination();
    assert!(publish_prepared(&directory.input, &directory.output).is_err());
    assert_eq!(std::fs::read_to_string(directory.output.join("owner.txt")).unwrap(), "owner");
}
```

- [ ] **Step 2: Run publication tests and confirm missing publisher failures**

Run: `cargo test -p toxtrain --test provenance --test publication`

Expected: FAIL.

- [ ] **Step 3: Write every provenance field**

Write every prepared split with this exact tab-separated header.

```text
detector_language	label	source_id	text
```

Use the CSV crate with a tab delimiter for both writing and reading.

Add `prepared_rows_round_trip_source_ids_and_text` for tabs, quotes, Unicode, and line breaks.

Use this exact TSV header for the `ProvenanceRow` type from Task 2.

```rust
const PROVENANCE_HEADER: [&str; 27] = [
    "dataset",
    "source_file_id",
    "source_id",
    "immutable_source_url",
    "archive_member",
    "revision",
    "file_path",
    "file_sha256",
    "acquired_at_unix_seconds",
    "license_id",
    "license_url",
    "citation",
    "upstream_lineage",
    "lineage_status",
    "source_language_code",
    "detector_language_code",
    "source_label",
    "detector_label",
    "label_conversion_version",
    "split_version",
    "normalization_version",
    "canonical_group_id",
    "representative_source_id",
    "source_split",
    "detector_split",
    "inclusion_status",
    "exclusion_reason",
];
```

Encode `upstream_lineage` as one compact JSON array in its TSV field.

Write an empty field for each absent optional value.

Write all enum values as their fixed lowercase identifiers.

Preserve the migrated `provenance_tsv_sorts_source_ids_and_writes_lowercase_values` test.

- [ ] **Step 4: Add the prepared manifest**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedFileIdentity {
    pub relative_path: String,
    pub sha256: Sha256Digest,
    pub rows: usize,
    pub clean_rows: usize,
    pub toxic_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreparedManifest {
    pub schema_version: String,
    pub sources: Vec<SourceRecord>,
    pub language_sources: BTreeMap<String, Vec<String>>,
    pub language_counts: BTreeMap<String, PreparedCounts>,
    pub source_rows: usize,
    pub source_label_counts: BTreeMap<String, usize>,
    pub detector_label_counts: BTreeMap<String, usize>,
    pub source_split_counts: BTreeMap<String, usize>,
    pub detector_split_counts: BTreeMap<String, usize>,
    pub inclusion_status_counts: BTreeMap<String, usize>,
    pub exclusion_reason_counts: BTreeMap<String, usize>,
    pub prepared_files: BTreeMap<String, PreparedFileIdentity>,
}
```

Export `PreparedFileIdentity` and `PreparedManifest` from `toxtrain::datasets`.

Key `language_counts` by uppercase detector language code.

Key `language_sources` by uppercase detector language code.

Build each `language_sources` value from matching `SourceRecord.source_file_id` values.

Sort and deduplicate each source-file identifier list.

Require one `language_sources` entry for every `Language::ALL` value.

Reject an unknown source-file identifier and a missing required language entry.

Build each `language_counts` value from all included and excluded provenance rows for that language.

Key `prepared_files` by the same value as `PreparedFileIdentity::relative_path`.

Require `rows` to equal `clean_rows + toxic_rows`.

Add every development, validation, and test TSV to `prepared_files`.

Key source-label counts as `<dataset>/<source-language-code>/<source-label>`.

Key detector-label counts as `<detector-language-code>/<detector-label>`.

Key source-split counts as `<dataset>/<source-split>`.

Key detector-split counts as `<detector-language-code>/<detector-split>`.

Publish this exact prepared tree.

```text
data/prepared-v1/
├── manifest.json
├── provenance.tsv
└── {CODE}/
    ├── development.tsv
    ├── validation.tsv
    └── test.tsv
```

Use uppercase detector codes for every `{CODE}` directory.

Create directories for `EN`, `ZH`, `AR`, `ID`, `PT`, `FR`, `HI`, `RU`, `JA`, `DE`, `TR`, `VI`, `KO`, and `IT`.

Keep the Spanish prepared source and artifact outside this new publication.

- [ ] **Step 5: Write sorted files and sync them**

Write languages, source rows, and manifest maps in stable sorted order.

Flush and sync every staged file before publication.

Compute each prepared file hash and class count while writing its bytes.

Do not reopen a test TSV to build `manifest.json`.

Reject publication unless every development class contains at least one row.

Reject publication unless every validation and test class has at least 300 rows.

Add a publication test for an empty clean or toxic development class.

- [ ] **Step 6: Reuse atomic no-replace rename**

Move the current Rustix implementation without changing its concurrent-destination behavior.

Sync the staging directory after all staged files are durable.

Publish with `RenameFlags::NOREPLACE`.

Sync the parent directory after the rename succeeds.

Remove only the staging directory after any failure.

Keep an existing or concurrent destination unchanged.

- [ ] **Step 7: Run provenance and publication tests**

Run: `cargo test -p toxtrain --test provenance --test publication`

Expected: PASS.

### Task 7: Wire acquisition, preparation, and CLI commands

**Files:**

- Modify: `crates/toxtrain/src/acquisition.rs`
- Modify: `crates/toxtrain/src/main.rs`
- Modify: `crates/toxtrain/tests/cli.rs`
- Create: `crates/toxtrain/tests/acquisition.rs`
- Modify: `README.md`

**Interfaces:**

- Consumes: The source catalog, reviewed observation, frozen source lock, and raw sources.
- Produces: Observe, freeze, acquire, prepare, setup, compile, and evaluation commands.

- [ ] **Step 1: Write CLI contract tests**

```rust
#[test]
fn observe_reads_the_catalog_and_refuses_overwrite() {
    let server = fixture_source_server();
    let catalog = write_source_catalog(server.url());
    let first = toxtrain(&["observe", "--source-catalog", catalog(), "--output", observation()]);
    assert!(first.status.success());
    let second = toxtrain(&["observe", "--source-catalog", catalog(), "--output", observation()]);
    assert!(!second.status.success());
}

#[test]
fn freeze_sources_requires_review_and_refuses_overwrite() {
    let rejected = toxtrain(&[
        "freeze-sources", "--observation", observation(), "--output", lock(),
    ]);
    assert!(!rejected.status.success());

    let first = toxtrain(&[
        "freeze-sources", "--observation", observation(), "--reviewed", "--output", lock(),
    ]);
    assert!(first.status.success());
    let second = toxtrain(&[
        "freeze-sources", "--observation", observation(), "--reviewed", "--output", lock(),
    ]);
    assert!(!second.status.success());
}

#[test]
fn prepare_requires_the_source_lock_and_refuses_overwrite() {
    let first = toxtrain(&[
        "prepare", "--source-lock", lock(), "--raw-root", raw(), "--output", output(),
    ]);
    assert!(first.status.success());
    let second = toxtrain(&[
        "prepare", "--source-lock", lock(), "--raw-root", raw(), "--output", output(),
    ]);
    assert!(!second.status.success());
}
```

Add a CLI case with `--audit-exclusions` and one draft development source identifier.

Assert that the final manifest increments both `excluded` and the audit-only exclusion count.

- [ ] **Step 2: Run CLI tests and confirm missing commands**

Run: `cargo test -p toxtrain --test cli`

Expected: FAIL.

- [ ] **Step 3: Add command arguments**

```rust
enum Command {
    Observe { source_catalog: PathBuf, output: PathBuf },
    FreezeSources { observation: PathBuf, reviewed: bool, output: PathBuf },
    Acquire { source_lock: PathBuf, output: PathBuf },
    Prepare {
        source_lock: PathBuf,
        raw_root: PathBuf,
        audit_exclusions: Option<PathBuf>,
        output: PathBuf,
    },
    Setup { source_lock: PathBuf, output: PathBuf },
    Compile {
        language: Language,
        development: PathBuf,
        validation: PathBuf,
        output: PathBuf,
        max_false_positive_basis_points: u16,
    },
    Eval {
        input: PathBuf,
        data_dir: PathBuf,
        minimum_action: MinimumActionArg,
    },
}
```

Clap shall expose `FreezeSources` as `freeze-sources`.

The `eval` command shall load conservative HurtLex entries only.

Add a CLI test that rejects `--include-inclusive` as an unknown argument.

Reject `freeze-sources` unless the caller supplies `--reviewed`.

Use this exact audit-exclusion header when `--audit-exclusions` is present.

```text
detector_language	source_id	reason
```

Reject duplicate identifiers, unknown languages, empty reasons, and non-development rows.

- [ ] **Step 4: Validate every observed revision and hash**

Decode each `archive_member` with `zip::ZipArchive<Cursor<Vec<u8>>>`.

Match the complete member name exactly. Reject a missing member and duplicate matching names.

Reject a member above 67,108,864 uncompressed bytes before allocation.

Read at most 67,108,865 bytes. Reject a result above the same limit.

Never extract an archive directory. Write the selected member bytes to the catalog `file_path`.

Hash the selected member bytes. Store that digest in `SourceRecord` and `FrozenSource`.

Repeat the same extraction during frozen acquisition before digest comparison.

Add tests for one exact member, a missing member, duplicate names, and an oversized member.

Fail before publication when any source differs from its locked identity.

Require the acquired source set to equal the frozen source set.

Reject missing, extra, or duplicate `source_file_id` values.

Allow unresolved lineage only for the Chinese and French TextDetox entries.

- [ ] **Step 5: Document the exact offline workflow**

```bash
cargo run --release --locked -p toxtrain -- observe \
  --source-catalog resources/datasets/source-catalog-v1.json \
  --output data/source-observation-v1
```

Review `data/source-observation-v1/source-observation-v1.json` and every recorded license before freezing.

```bash
cargo run --release --locked -p toxtrain -- freeze-sources \
  --observation data/source-observation-v1/source-observation-v1.json \
  --reviewed \
  --output resources/datasets/source-lock-v1.json

cargo run --release --locked -p toxtrain -- acquire \
  --source-lock resources/datasets/source-lock-v1.json \
  --output data/raw-v1

cargo run --release --locked -p toxtrain -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --output data/prepared-draft-v1
```

- [ ] **Step 6: Preserve all current CLI tests**

Preserve all 28 current CLI test names and assertions across the two packages.

Keep every `check` test in the runtime `tests/toxcheck_cli.rs` file.

Move each `setup`, `eval`, fetch, prepare, and offline help test into the `toxtrain` tests.

Move the acquisition failure tests into `crates/toxtrain/tests/acquisition.rs`.

Adapt command invocations to the new `toxtrain` binary without reducing assertions.

- [ ] **Step 7: Run the complete offline test set**

Run:

```bash
cargo test -p toxtrain --test source_manifest --test dataset_adapters --test preparation --test provenance --test publication --test acquisition --test evaluation --test textdetox --test compiler --test cli
```

Expected: PASS.

- [ ] **Step 8: Run workspace verification**

Run: `cargo fmt --all --check`

Expected: PASS.

Run: `cargo test --workspace --all-targets`

Expected: PASS.

### Task 8: Freeze sources and publish prepared data

**Files:**

- Create: `data/source-observation-v1/`
- Create after source review: `resources/datasets/source-lock-v1.json`
- Create: `data/raw-v1/`
- Create: `data/prepared-draft-v1/`

**Interfaces:**

- Consumes: The reviewed source catalog and the tested offline commands.
- Produces: One frozen source lock and one immutable draft publication for rule evidence selection.

- [ ] **Step 1: Observe every catalog source**

Run:

```bash
cargo run --release --locked -p toxtrain -- observe \
  --source-catalog resources/datasets/source-catalog-v1.json \
  --output data/source-observation-v1
```

Expected: Every catalog entry has one SHA-256 value and acquisition metadata.

Each entry with a requested revision shall have the same locked revision.

- [ ] **Step 2: Review and freeze the observed identities**

Compare every observed revision with the requested revision.

Review every recorded license identifier, URL, citation, and upstream lineage value.

Run:

```bash
cargo run --release --locked -p toxtrain -- freeze-sources \
  --observation data/source-observation-v1/source-observation-v1.json \
  --reviewed \
  --output resources/datasets/source-lock-v1.json
```

Expected: The command publishes one immutable source lock.

- [ ] **Step 3: Acquire the frozen source set**

Run:

```bash
cargo run --release --locked -p toxtrain -- acquire \
  --source-lock resources/datasets/source-lock-v1.json \
  --output data/raw-v1
```

Expected: Every acquired file matches the frozen SHA-256 value.

- [ ] **Step 4: Publish the draft language splits**

Run:

```bash
cargo run --release --locked -p toxtrain -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --output data/prepared-draft-v1
```

Expected: The publication contains 14 language directories, one manifest, and one provenance file.

Expected: Each validation and test class has at least 300 rows.

Run: `cargo clippy --workspace --all-targets -- -D warnings`

Expected: PASS.
