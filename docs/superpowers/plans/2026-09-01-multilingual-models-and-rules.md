# Multilingual models and rules implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Train, calibrate, embed, and integrate deterministic toxicity models and conservative rule packs for the 14 non-Spanish languages.

**Architecture:** Train one independent 65,536-bin table per language. Set a clean-control floor before validation calibration. Keep Spanish on its legacy paths.

**Tech stack:** Rust 2024, fixed Bernoulli log-odds tables, language-specific Unicode profiles, static rule packs, TSV behavior panels, and atomic manifests.

**Spec:** `docs/superpowers/specs/2026-09-01-multilingual-sparse-nudge-detector-design.md`

## Global constraints

Complete the runtime foundation and dataset pipeline plans first.

Execute tasks in this order: 1, 3, 4, 5, 6, 2, 7, and 8.

Complete Task 1 of the verification plan after Task 8 here and before Task 9 here.

Do not open any untouched test split during this plan.

Use development rows for weights and rule work.

Use authored clean controls to set the minimum boundary.

Use validation rows for final boundary selection and gate checks.

Treat clean controls as product contracts. Do not treat them as accuracy evidence.

Each new validation result shall have false warnings at or below three percent.

Each new validation result shall have ordinary precision at or above 90 percent.

Set no validation recall floor.

Use conservative HurtLex entries in every compiler, evaluator, benchmark, and product runtime path.

Expose no configuration that changes the HurtLex level selection on these paths.

Keep Spanish assets and outputs unchanged.

The project directory is not a Git repository. Each task ends with a verification checkpoint instead of a commit.

---

## File structure

- Create `crates/toxtrain/src/calibration.rs` for final-path boundary selection.
- Extend `crates/toxtrain/src/compiler.rs` with development training and batch artifact compilation.
- Create `crates/toxtrain/src/model_manifest.rs` for generated model metadata and hashes.
- Create `src/rules` for non-Spanish word and compact rule packs.
- Extend `src/registry.rs` with artifacts, hashes, rule versions, and caches.
- Create behavior fixtures under `tests/fixtures/behavior`.
- Publish one complete 15-artifact set under `resources/models/multilingual-v2`.

### Task 1: Add exact validation gates and final-path calibration

**Files:**

- Create: `crates/toxtrain/src/calibration.rs`
- Create: `crates/toxtrain/tests/calibration.rs`
- Modify: `crates/toxtrain/src/lib.rs`

**Interfaces:**

- Consumes: Validation labels, sparse raw scores, and frozen rule decisions.
- Produces: `CalibrationResult` with one admissible raw boundary and matrix.

- [ ] **Step 1: Write exact gate boundary tests**

```rust
#[test]
fn gate_boundaries_use_exact_integer_math() {
    let nine_of_300 = ConfusionMatrix {
        true_positive: 9,
        true_negative: 291,
        false_positive: 9,
        false_negative: 0,
    };
    let ten_of_300 = ConfusionMatrix {
        false_positive: 10,
        true_negative: 290,
        ..nine_of_300
    };
    let ninety_percent = ConfusionMatrix {
        true_positive: 9,
        true_negative: 299,
        false_positive: 1,
        false_negative: 0,
    };
    let below_ninety_percent = ConfusionMatrix {
        true_positive: 8,
        ..ninety_percent
    };
    let no_predictions = ConfusionMatrix {
        true_positive: 0,
        true_negative: 300,
        false_positive: 0,
        false_negative: 9,
    };
    let low_recall = ConfusionMatrix {
        true_positive: 1,
        true_negative: 300,
        false_positive: 0,
        false_negative: 299,
    };

    assert!(gates(nine_of_300).false_warning_passed);
    assert!(!gates(ten_of_300).false_warning_passed);
    assert!(gates(ninety_percent).precision_passed);
    assert!(!gates(below_ninety_percent).precision_passed);
    assert!(!gates(no_predictions).precision_passed);
    assert!(!gates(no_predictions).passed());
    assert!(gates(low_recall).passed());
}
```

- [ ] **Step 2: Run the gate test and confirm the missing calibration module failure**

Run: `cargo test -p toxtrain --test calibration gate_boundaries_use_exact_integer_math`

Expected: FAIL.

- [ ] **Step 3: Add calibration types and integer gates**

```rust
use serde::{Deserialize, Serialize};

pub struct CalibrationRow {
    pub label: EvalLabel,
    pub sparse_raw_score: i32,
    pub rule_should_nudge: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub false_warning_passed: bool,
    pub precision_passed: bool,
    pub has_true_positive: bool,
}

impl GateResult {
    pub fn passed(self) -> bool {
        self.false_warning_passed && self.precision_passed && self.has_true_positive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationResult {
    pub language: Language,
    pub boundary: i32,
    pub matrix: ConfusionMatrix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryEvaluation {
    pub boundary: i32,
    pub matrix: ConfusionMatrix,
}

#[derive(Debug, thiserror::Error)]
pub enum CalibrationError {
    #[error("the frozen rule channel fails a validation gate for {0}")]
    RuleChannelGateFailure(Language),
    #[error("no admissible validation boundary exists for {0}")]
    NoAdmissibleBoundary(Language),
}

pub fn calibrate(
    language: Language,
    rows: &[CalibrationRow],
) -> Result<CalibrationResult, CalibrationError>;

pub fn select_best(
    language: Language,
    candidates: &[BoundaryEvaluation],
) -> Result<BoundaryEvaluation, CalibrationError>;

pub fn gates(matrix: ConfusionMatrix) -> GateResult {
    let tp = u128::from(matrix.true_positive);
    let fp = u128::from(matrix.false_positive);
    let tn = u128::from(matrix.true_negative);
    let predicted_toxic = tp + fp;
    GateResult {
        false_warning_passed: 10_000 * fp <= 300 * (fp + tn),
        precision_passed: predicted_toxic > 0 && 100 * tp >= 90 * predicted_toxic,
        has_true_positive: tp > 0,
    }
}
```

- [ ] **Step 4: Write boundary search and tie-break tests**

```rust
#[test]
fn calibration_maximizes_true_positives_then_minimizes_false_positives() {
    let result = calibrate(Language::En, &fixture_rows()).expect("calibrate");
    assert_eq!(result.boundary, 11);
    assert_eq!(result.matrix.true_positive, 9);
    assert_eq!(result.matrix.false_positive, 1);
}

#[test]
fn rule_channel_gate_failure_cannot_be_hidden_by_sparse_threshold() {
    let error = calibrate(Language::En, &rule_only_false_warnings()).expect_err("gate failure");
    assert!(matches!(error, CalibrationError::RuleChannelGateFailure(Language::En)));
}

#[test]
fn searches_all_distinct_and_adjacent_boundaries() {
    let rows = [
        toxic(-2),
        clean(0),
        toxic(0),
        clean(7),
    ];
    assert_eq!(candidate_boundaries(&rows), vec![-2, -1, 0, 1, 7, 8]);
}

#[test]
fn rejects_no_admissible_boundary() {
    let mut rows = Vec::new();
    rows.extend((0..300).map(|_| toxic(5)));
    rows.extend((0..300).map(|_| clean(5)));
    assert!(matches!(
        calibrate(Language::En, &rows),
        Err(CalibrationError::NoAdmissibleBoundary(Language::En))
    ));
}

#[test]
fn breaks_ties_with_false_warnings_then_boundary() {
    let matrix = |false_positive| ConfusionMatrix {
        true_positive: 99,
        true_negative: 300 - false_positive,
        false_positive,
        false_negative: 1,
    };
    let fewer_false_warnings = select_best(Language::En, &[
        BoundaryEvaluation { boundary: 20, matrix: matrix(1) },
        BoundaryEvaluation { boundary: 10, matrix: matrix(0) },
    ]).expect("candidate");
    assert_eq!(fewer_false_warnings.boundary, 10);

    let higher_boundary = select_best(Language::En, &[
        BoundaryEvaluation { boundary: 20, matrix: matrix(0) },
        BoundaryEvaluation { boundary: 21, matrix: matrix(0) },
    ]).expect("candidate");
    assert_eq!(higher_boundary.boundary, 21);
}

fn fixture_rows() -> Vec<CalibrationRow> {
    let mut rows = Vec::new();
    rows.extend((0..9).map(|_| toxic(11)));
    rows.push(toxic(10));
    rows.push(clean(11));
    rows.extend((0..4).map(|_| clean(10)));
    rows.extend((0..95).map(|_| clean(0)));
    rows
}

fn rule_only_false_warnings() -> Vec<CalibrationRow> {
    let mut rows = vec![toxic(0)];
    rows.extend((0..10).map(|_| rule_clean(0)));
    rows.extend((0..290).map(|_| clean(0)));
    rows
}
```

Use these free test helpers in `crates/toxtrain/tests/calibration.rs`.

```rust
fn toxic(sparse_raw_score: i32) -> CalibrationRow {
    CalibrationRow { label: EvalLabel::Toxic, sparse_raw_score, rule_should_nudge: false }
}

fn clean(sparse_raw_score: i32) -> CalibrationRow {
    CalibrationRow { label: EvalLabel::Clean, sparse_raw_score, rule_should_nudge: false }
}

fn rule_clean(sparse_raw_score: i32) -> CalibrationRow {
    CalibrationRow { label: EvalLabel::Clean, sparse_raw_score, rule_should_nudge: true }
}
```

- [ ] **Step 5: Implement complete candidate search**

```rust
use std::cmp::Reverse;

pub fn candidate_boundaries(rows: &[CalibrationRow]) -> Vec<i32> {
    let mut candidates = rows.iter()
        .flat_map(|row| [row.sparse_raw_score, row.sparse_raw_score.saturating_add(1)])
        .collect::<Vec<_>>();
    candidates.sort_unstable();
    candidates.dedup();
    candidates
}

let predicted = |row: &CalibrationRow, boundary: i32| {
    row.rule_should_nudge || row.sparse_raw_score >= boundary
};
```

Rank eligible candidates by descending true positives, ascending false positives, then descending boundary.

Use `max_by_key(|item| (item.matrix.true_positive, Reverse(item.matrix.false_positive), item.boundary))`.

- [ ] **Step 6: Run calibration tests**

Run: `cargo test -p toxtrain --test calibration`

Expected: PASS.

### Task 2: Add development training and batch model manifests

**Files:**

- Modify: `crates/toxtrain/src/compiler.rs`
- Create: `crates/toxtrain/src/model_manifest.rs`
- Modify: `crates/toxtrain/tests/compiler.rs`
- Create: `crates/toxtrain/tests/model_manifest.rs`
- Create: `resources/models/es-legacy-input-v1.json`
- Modify: `crates/toxtrain/src/main.rs`
- Modify: `src/evaluation.rs`
- Modify: `src/sparse.rs`
- Modify: `src/lib.rs`

**Interfaces:**

- Consumes: Prepared development and validation rows, language profiles, and frozen rule decisions.
- Produces: `SparseV2Input`, 14 artifacts, and one atomic 15-language manifest.

```rust
use crate::datasets::PreparedRow;

pub struct CompileRequest {
    pub language: Language,
    pub development: Vec<PreparedRow>,
    pub validation: Vec<PreparedRow>,
    pub rule_channel: RuleChannel,
    pub clean_controls: Vec<String>,
}

pub struct BatchCompileOptions {
    pub prepared_root: PathBuf,
    pub hurtlex_root: PathBuf,
    pub behavior_root: Option<PathBuf>,
    pub spanish_legacy: PathBuf,
    pub output: PathBuf,
}

#[derive(PartialEq)]
pub struct CompiledLanguage {
    pub artifact: Vec<u8>,
    pub calibration: CalibrationResult,
    pub score_scale: u32,
    pub validation_predictions: Vec<bool>,
}

pub fn compile_language(request: &CompileRequest) -> Result<CompiledLanguage, CompileError>;
```

- [ ] **Step 1: Write deterministic compiler tests**

```rust
#[test]
fn compiler_is_deterministic_and_never_accepts_test_rows() {
    let first_request = fixture_request();
    let second_request = fixture_request();
    let first = compile_language(&first_request).expect("first");
    let second = compile_language(&second_request).expect("second");
    assert_eq!(first.artifact, second.artifact);
    assert_eq!(first.calibration, second.calibration);
}

fn fixture_request() -> CompileRequest {
    let row = |label, text: &str| PreparedRow {
        detector_language: Language::En,
        label,
        source_id: format!("fixture/{text}"),
        text: text.to_owned(),
    };
    let mut validation = Vec::new();
    validation.extend((0..300).map(|index| {
        row(EvalLabel::Toxic, &format!("I will kill you {index}"))
    }));
    validation.extend((0..300).map(|index| {
        row(EvalLabel::Clean, &format!("have a nice day {index}"))
    }));
    CompileRequest {
        language: Language::En,
        development: vec![
            row(EvalLabel::Toxic, "I will kill you"),
            row(EvalLabel::Toxic, "I hope you die"),
            row(EvalLabel::Clean, "have a nice day"),
            row(EvalLabel::Clean, "I hope you win"),
        ],
        validation,
        rule_channel: fixture_rule_channel(Language::En),
    }
}

#[test]
fn serialized_model_matches_validation_predictions() {
    let request = fixture_request();
    let compiled = compile_language(&request).expect("compile");
    let model = SparseModel::from_bytes(&compiled.artifact).expect("parse");
    for (row, expected) in request.validation.iter()
        .zip(&compiled.validation_predictions)
    {
        let rules = request.rule_channel.analyze(&row.text, ReplyTarget::Unknown);
        let actual = rules.should_nudge
            || model.raw_score(&row.text) >= model.raw_boundary();
        assert_eq!(actual, *expected, "{}", row.text);
    }
}

fn fixture_rule_channel(language: Language) -> RuleChannel {
    let bytes = b"id\tpos\tcategory\tstereotype\tlemma\tlevel\n";
    RuleChannel::from_hurtlex_bytes(language, Some(bytes))
        .expect("fixture rule channel")
}
```

Add a CLI test that passes `--test` to `toxtrain compile` and expects an unknown-argument error.

Add this batch CLI help contract to `crates/toxtrain/tests/compiler.rs`.

```rust
#[test]
fn compile_help_exposes_only_batch_inputs() {
    let output = Command::new(env!("CARGO_BIN_EXE_toxtrain"))
        .args(["compile", "--help"])
        .output()
        .expect("compile help");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(stdout.contains("--prepared-root"));
    assert!(stdout.contains("--hurtlex-root"));
    assert!(stdout.contains("--behavior-root"));
    assert!(stdout.contains("--spanish-legacy"));
    assert!(stdout.contains("--output"));
    assert!(!stdout.contains("--test"));
}
```

- [ ] **Step 2: Run compiler tests and confirm the missing compiler failure**

Run: `cargo test -p toxtrain --test compiler --test model_manifest`

Expected: FAIL.

- [ ] **Step 3: Implement Bernoulli document log odds**

```rust
pub fn train_weights(
    profile: FeatureProfile,
    normalization: NormalizationProfile,
    development: &[PreparedRow],
) -> Result<TrainedWeights, CompileError> {
    let mut clean = vec![0_u32; 65_536];
    let mut toxic = vec![0_u32; 65_536];
    let document_counts = count_document_bins(
        profile,
        normalization,
        development,
        &mut clean,
        &mut toxic,
    )?;
    quantize_log_odds(clean, toxic, document_counts, 256, 2)
}
```

`count_document_bins` shall count each deduplicated bin once per document.

`document_counts` shall contain separate clean and toxic document totals.

`quantize_log_odds` shall use Laplace smoothing of one and signed 16-bit saturation.

- [ ] **Step 4: Remove offline compilation from the runtime crate**

Move the remaining training helpers from `src/sparse.rs` into `toxtrain::compiler`.

Remove `compile_sparse_model`, `SparseCompilation`, and `SparseCompileError` from `toxcheck`.

Keep artifact parsing, encoding, raw scoring, and ordinal score mapping in `toxcheck`.

- [ ] **Step 5: Calibrate through the frozen rule scorer**

The compiler shall build `CalibrationRow` values from validation data.

The compiler shall build clean-control rows from the versioned behavior panels.

Each new language shall supply 16 frozen clean controls.

The largest clean-control sparse score shall set the minimum candidate boundary.

The validation search shall maximize recall at or above that minimum.

The rule channel shall allow every clean control before compilation continues.

The compiler shall build one `RuleChannel` from each locked HurtLex file.

The compiler and shipping detector shall call the same `RuleChannel::analyze` method.

Calculate one score scale from that language's validation sparse raw scores.

```rust
pub fn validation_score_scale(
    raw_scores: &[i32],
    boundary: i32,
) -> Result<u32, CompileError>;
```

Sort every raw score in ascending order. Reject an empty score set.

Use index `(len - 1) / 10` for the lower value.

Use index `(len - 1) * 9 / 10` for the upper value.

Calculate both absolute distances from the selected raw boundary with `i64` arithmetic.

Use the larger distance. Clamp the result to `1..=u32::MAX`.

Store this value in `CompiledLanguage`, `SparseV2Input`, and `ModelManifestEntry`.

Add exact tests for an asymmetric range, an all-boundary range, and the full `i32` range.

Use boundary `100` and scores `0,10,20,30,40,50,60,70,80,90,100` in the asymmetric test.

Require scale `90` in that test and scale `1` when every score equals the boundary.

Require scale `u32::MAX` for nine minimum scores, two maximum scores, and boundary `i32::MIN`.

Parse the emitted artifact. Require its scale to equal `CompiledLanguage.score_scale`.

- [ ] **Step 6: Generate complete per-language metadata**

```rust
use std::path::Path;

use crate::datasets::PreparedCounts;
use crate::evidence::Sha256Digest;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelManifestEntry {
    pub language: Language,
    pub artifact_relative_path: String,
    pub dataset_inputs: Vec<DatasetInput>,
    pub feature_profile: FeatureProfile,
    pub normalization_profile: NormalizationProfile,
    pub feature_schema: FeatureSchema,
    pub rule_pack_version: u16,
    pub rule_pack_sha256: Sha256Digest,
    pub hurtlex_sha256: Option<Sha256Digest>,
    pub clean_control_rows: usize,
    pub clean_control_sha256: Option<Sha256Digest>,
    pub development_rows: usize,
    pub validation_rows: usize,
    pub test_rows: usize,
    pub duplicate_rows: usize,
    pub conflict_rows: usize,
    pub excluded_rows: usize,
    pub boundary: i32,
    pub score_scale: u32,
    pub false_warning_limit_basis_points: u16,
    pub validation: ConfusionMatrix,
    pub validation_metrics: Metrics,
    pub validation_gates: Option<GateResult>,
    pub artifact_bytes: usize,
    pub artifact_sha256: Sha256Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatasetInput {
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub revision: Option<String>,
    pub file_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestInputs {
    pub dataset_inputs: Vec<DatasetInput>,
    pub prepared_counts: PreparedCounts,
    pub rule_pack_version: u16,
    pub rule_pack_sha256: Sha256Digest,
    pub hurtlex_sha256: Option<Sha256Digest>,
    pub clean_control_rows: usize,
    pub clean_control_sha256: Option<Sha256Digest>,
}

pub fn build_manifest_entry(
    compiled: &CompiledLanguage,
    inputs: ManifestInputs,
) -> Result<ModelManifestEntry, ModelSetError>;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FrozenFileReference {
    pub relative_path: String,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SpanishLegacyInput {
    pub schema_version: u16,
    pub artifact: FrozenFileReference,
    pub metadata: FrozenFileReference,
    pub source: FrozenFileReference,
    pub hurtlex: FrozenFileReference,
    pub proof_report: FrozenFileReference,
    pub behavior_panel: FrozenFileReference,
    pub dataset: DatasetId,
    pub source_file_id: String,
    pub dataset_revision: String,
    pub source_rows: usize,
    pub development_rows: usize,
    pub validation_rows: usize,
    pub test_rows: usize,
    pub duplicate_rows: usize,
    pub conflict_rows: usize,
    pub excluded_rows: usize,
    pub boundary: i32,
    pub score_scale: u32,
    pub false_warning_limit_basis_points: u16,
    pub validation: ConfusionMatrix,
    pub test: ConfusionMatrix,
    pub behavior: ConfusionMatrix,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelManifest {
    pub schema_version: u16,
    pub entries: Vec<ModelManifestEntry>,
}

pub fn validate_model_set(
    root: &Path,
    manifest: &ModelManifest,
) -> Result<(), ModelSetError>;
```

Write `es-legacy-input-v1.json` with `serde_json::to_vec` and these frozen identities.

| Input | SHA-256 |
|---|---|
| `resources/models/es-chargram-v1.bin` | `3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36` |
| `resources/models/es-chargram-v1.json` | `b5c334f79334b20843409ef9bbebdd4fcbce9580239ae6f9f496f14bcf4ba582` |
| `data/textdetox/es-source.tsv` | `8e3c8078d7406e7b695ffb943e0439240ada11d6abc9d12ac313efdb6d2f1da9` |
| `data/raw-v1/hurtlex/ES/1.2/hurtlex_ES.tsv` | `5adadf7886ea332e6e07de1f5abb98a71a3dacbf3bea993b21100c9b4bffd4ba` |
| `docs/spanish-proof-report.md` | `7634a66dfd43e22aac8d729ce5d06cbd1384aafe5f4ddb24c77a087039337d42` |
| `samples/spanish-audit.tsv` | `8313713f8e18e5c066f6f320efb6ee340b7580cba4739fc4612e1dfe4a8a7575` |

Record revision `01907546324b0330d2d8b7669648cc18823323e5`.

Record dataset `textdetox` and source-file identifier `textdetox-es-legacy`.

Record counts `5000, 3418, 762, 819, 1, 0, 1` in the declared field order.

Record boundary `10962`, score scale `27695`, and false-warning limit `300`.

Record validation `TP=159, TN=382, FP=18, FN=203`.

Record test `TP=177, TN=386, FP=14, FN=242`.

Record behavior `TP=39, TN=46, FP=0, FN=3`.

Reject any Spanish file identity mismatch before staging the 15-entry manifest.

Add `manifest_rejects_a_missing_artifact` to remove one declared file from a temporary complete set.

Require `ModelSetError::MissingArtifact(Language::En)`.

Add `manifest_rejects_an_artifact_digest_mismatch` to change one declared artifact byte.

Require `ModelSetError::ArtifactDigestMismatch(Language::En)`.

Keep `CompiledLanguage` free of source, count, rule, and HurtLex identity metadata.

Read the test count from prepared metadata. Do not open a test TSV.

Read `PreparedManifest.language_sources` before building each new manifest entry.

Join each listed source-file identifier to exactly one `PreparedManifest.sources` record.

Build `dataset_inputs` from the non-HurtLex source records.

Require exactly one HurtLex source record for each new language.

Verify the HurtLex bytes against that source record before passing its digest to `ManifestInputs`.

Derive `artifact_relative_path` from the language through one total mapping.

Read that language only from `compiled.calibration.language`.

Build each `ModelManifestEntry` in the batch publisher after `compile_language` succeeds.

Add tests for an unknown source-file identifier, duplicate source identity, and HurtLex digest mismatch.

Set `validation_gates` to `None` for Spanish and a passing value for every new language.

Add `Serialize` and `Deserialize` to `ConfusionMatrix` and `Metrics`.

- [ ] **Step 7: Publish the artifact set atomically**

Replace the earlier per-language `Compile` arguments with this batch form.

```rust
Compile {
    prepared_root: PathBuf,
    hurtlex_root: PathBuf,
    spanish_legacy: PathBuf,
    output: PathBuf,
},
```

Require entries for every `Language::ALL` value.

Load and verify `SpanishLegacyInput` before constructing the Spanish entry.

Copy its frozen Spanish artifact without recompilation.

Sort manifest entries by `Language::ALL` order.

Sort each `dataset_inputs` value by dataset, source-file identifier, revision, and file hash.

Hash a canonical rule identity for every language.

The identity shall include the version, match profile, and every ordered phrase list.

Write all new artifacts and `manifest.json` through one sibling staging directory.

Add `model_set_publication_preserves_an_existing_destination`.

Require the test to compare every pre-existing byte after the failed publish.

- [ ] **Step 8: Run compiler and manifest tests**

Run: `cargo test -p toxtrain --test calibration --test compiler --test model_manifest`

Expected: PASS.

### Task 3: Add the non-Spanish semantic rule engine

**Files:**

- Create: `src/rules/mod.rs`
- Create: `src/rules/word.rs`
- Create: `src/rules/compact.rs`
- Create: `src/rules/identity.rs`
- Modify: `src/policy.rs`
- Modify: `src/lib.rs`
- Create: `tests/rules_v2.rs`

**Interfaces:**

- Consumes: One `LanguageRules` pack and normalized text.
- Produces: High-confidence rule scores for four event types and suppression evidence.

```rust
pub struct RuleOutcome {
    pub score: u8,
    pub should_nudge: bool,
    pub evidence: Vec<RuleEvidence>,
}

pub fn analyze_with_rules(
    rules: &LanguageRules,
    text: &str,
    reply_target: ReplyTarget,
) -> RuleOutcome;

pub fn canonical_rule_identity_for(rules: &LanguageRules) -> Vec<u8>;
```

- [ ] **Step 1: Write full-frame and suppression tests**

```rust
#[test]
fn harm_words_without_speaker_intent_do_not_form_a_threat() {
    let rules = fixture_rules();
    let result = analyze_with_rules(&rules, "Did you kill him?", ReplyTarget::Unknown);
    assert!(!result.should_nudge);
}

#[test]
fn negation_quote_and_report_suppress_one_linked_event() {
    let rules = fixture_rules();
    assert!(!analyze_with_rules(
        &rules,
        "I will not kill you",
        ReplyTarget::Unknown,
    ).should_nudge);
    assert!(!analyze_with_rules(
        &rules,
        "\"I will kill you,\" she wrote",
        ReplyTarget::Unknown,
    ).should_nudge);
}

#[test]
fn complete_fixture_threat_nudges() {
    let rules = fixture_rules();
    let result = analyze_with_rules(&rules, "I will kill you", ReplyTarget::Unknown);
    assert_eq!(result.score, DIRECT_THREAT_SCORE);
    assert!(result.should_nudge);
}
```

Build `fixture_rules` with exact English entries for `you`, `him`, `kill`, `I will`, `not`, and `wrote`.

Include `I hope`, `die`, `kill yourself`, `idiot`, and `it is wrong to say`.

Use empty `PhraseSet` values for fields that these three tests do not use.

- [ ] **Step 2: Run rule tests and confirm the missing engine failure**

Run: `cargo test --test rules_v2`

Expected: FAIL.

- [ ] **Step 3: Add immutable rule types**

```rust
pub struct PhraseSet(&'static [&'static str]);

impl PhraseSet {
    pub const fn new(phrases: &'static [&'static str]) -> Self {
        Self(phrases)
    }

    pub const fn empty() -> Self {
        Self(&[])
    }
}

pub struct LanguageRules {
    pub language: Language,
    pub version: u16,
    pub targets: PhraseSet,
    pub harm_predicates: PhraseSet,
    pub intent_markers: PhraseSet,
    pub implicit_target_threats: PhraseSet,
    pub wish_markers: PhraseSet,
    pub harm_outcomes: PhraseSet,
    pub implicit_target_harm_wishes: PhraseSet,
    pub self_harm_commands: PhraseSet,
    pub strong_insults: PhraseSet,
    pub implicit_target_directed_insults: PhraseSet,
    pub negative_sentiment: PhraseSet,
    pub copulas_or_vocatives: PhraseSet,
    pub negators: PhraseSet,
    pub reports: PhraseSet,
    pub counterspeech_markers: PhraseSet,
    pub proposition_boundaries: PhraseSet,
    pub matching: RuleMatchProfile,
}

pub enum RuleMatchProfile { WordClauses, CompactClauses }

pub const RULE_NUDGE_THRESHOLD: u8 = 50;
pub const DIRECT_THREAT_SCORE: u8 = 95;
pub const HARM_WISH_SCORE: u8 = 85;
pub const SELF_HARM_COMMAND_SCORE: u8 = 95;
pub const DIRECTED_INSULT_SCORE: u8 = 70;
pub const HURTLEX_SCORE: u8 = 30;
pub const NEGATIVE_SENTIMENT_SCORE: u8 = 20;
```

- [ ] **Step 4: Require complete event frames**

```rust
let direct_threat = clause.has_target()
    && clause.has_harm_predicate()
    && (clause.has_speaker_intent() || clause.has_imperative());
let harm_wish = clause.has_target() && clause.has_wish_marker() && clause.has_harm_outcome();
let self_harm = clause.has_self_harm_command() && clause.has_direct_or_reflexive_target();
let insult = clause.has_target() && clause.has_strong_insult() && clause.has_copula_or_vocative();
```

- [ ] **Step 5: Apply event-local suppression**

Suppress only a balanced quoted span, a linked report span, a negated event, or explicit counterspeech.

Link counterspeech only within the same clause and matched event span.

Use explicit phrases such as `do not say`, `stop saying`, and `it is wrong to say`.

Use configured proposition boundaries such as `but`. Do not hardcode one language.

Allow the three implicit-target fields only as exact whole-proposition matches.

Do not treat a bare `stop` token as counterspeech.

Do not suppress an entire message from one report word.

Set `should_nudge` from `score >= RULE_NUDGE_THRESHOLD` after event-local suppression.

Score a direct threat at `DIRECT_THREAT_SCORE`.

Score a harm wish at `HARM_WISH_SCORE`.

Score a self-harm command at `SELF_HARM_COMMAND_SCORE`.

Score a directed insult at `DIRECTED_INSULT_SCORE`.

Keep sentiment support below `RULE_NUDGE_THRESHOLD`. Sentiment alone shall return `false`.

Score a targeted negative-sentiment cue at `NEGATIVE_SENTIMENT_SCORE` when no complete event frame exists.

Do not score untargeted negative sentiment.

Treat `ReplyTarget::Person` and `ReplyTarget::ProtectedGroup` as target cues.

Treat `ReplyTarget::Unknown` as no target cue.

A reply target shall not supply intent, an imperative, a wish, a harmful outcome, or a strong insult.

A reply target shall not bypass event-local suppression.

Use the maximum unsuppressed event score. Never add event scores.

Add tests for every event score, both concrete reply targets, unknown targets, and suppression with a reply target.

Add a counterspeech test for `It is wrong to say I will kill you`.

Add a test that changes only `counterspeech_markers` and changes the canonical rule hash.

- [ ] **Step 6: Encode one deterministic rule identity**

Start with `TOXRULE1`, the language code, the version, and the match-profile identifier.

Encode each phrase field in `LanguageRules` declaration order.

Include `counterspeech_markers` after `reports` in the encoded phrase-field sequence.

Encode each field's zero-based `u8` ordinal and little-endian `u32` phrase count first.

Encode each phrase as a little-endian `u32` byte length followed by exact UTF-8 bytes.

Add a test that moves one phrase between fields and changes the canonical rule hash.

Test one fixed fixture identity SHA-256 value here.

- [ ] **Step 7: Keep the new engine isolated**

Do not change the Spanish dispatch or the shipping detector in this task.

The complete language dispatcher belongs to Task 6.

- [ ] **Step 8: Run rule and Spanish tests**

Run: `cargo test --test rules_v2 --test policy --test spanish_compatibility`

Expected: PASS.

### Task 4: Add word-script rule packs and behavior panels

**Files:**

- Create: `src/rules/packs/word.rs`
- Create: `tests/fixtures/behavior/en.tsv`
- Create: `tests/fixtures/behavior/id.tsv`
- Create: `tests/fixtures/behavior/pt.tsv`
- Create: `tests/fixtures/behavior/fr.tsv`
- Create: `tests/fixtures/behavior/ru.tsv`
- Create: `tests/fixtures/behavior/de.tsv`
- Create: `tests/fixtures/behavior/tr.tsv`
- Create: `tests/fixtures/behavior/vi.tsv`
- Create: `tests/fixtures/behavior/it.tsv`
- Create: `tests/fixtures/behavior/native-review-v1.tsv`
- Create: `tests/fixtures/behavior/authored-v1.tsv`
- Create: `resources/datasets/rule-audit-v1.tsv`
- Create: `crates/toxtrain/src/behavior_panel.rs`
- Create: `crates/toxtrain/tests/behavior_panels.rs`
- Modify: `crates/toxtrain/src/lib.rs`

**Interfaces:**

- Consumes: Word-clause rules and development identifiers from `data/prepared-draft-v1`.
- Produces: Nine versioned packs and nine 24-row contract panels.

- [ ] **Step 1: Add the panel parser and shape test**

Use this exact TSV header.

```text
case_id	language	expected_nudge	event_type	pair_id	control_kind	evidence_kind	evidence_ref	text
```

```rust
#[derive(Debug, Deserialize)]
pub struct BehaviorRow {
    pub case_id: String,
    pub language: Language,
    pub expected_nudge: bool,
    pub event_type: EventType,
    pub pair_id: String,
    pub control_kind: ControlKind,
    pub evidence_kind: EvidenceKind,
    pub evidence_ref: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType { Threat, HarmWish, SelfHarmCommand, DirectedInsult, None }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlKind {
    None, Negation, Quotation, Reporting, Counterspeech, ViolenceQuestion,
    Replacement, Context, Collision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind { Dataset, NativeReview, Authored }

pub fn load_panel(
    root: &Path,
    language: Language,
) -> Result<Vec<BehaviorRow>, BehaviorPanelError>;

#[test]
fn every_new_panel_has_eight_toxic_and_at_least_sixteen_clean_rows() {
    for language in Language::ALL.into_iter().filter(|value| *value != Language::Es) {
        let root = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/behavior",
        ));
        let panel = load_panel(root, language)
            .expect("panel");
        assert_eq!(panel.len(), 24);
        assert_eq!(panel.iter().filter(|row| row.expected_nudge).count(), 8);
        assert_eq!(panel.iter().filter(|row| !row.expected_nudge).count(), 16);
        validate_event_distribution(&panel).expect("distribution");
    }
}
```

Reject duplicate case identifiers, unknown enum values, missing evidence, invalid pairs, and a wrong language.

Require each native review reference to exist in `native-review-v1.tsv`.

Require each authored reference to exist in `authored-v1.tsv`.

Do not represent an authored case as native-speaker review.

Create the rule-audit file with this exact tab-separated header.

```text
detector_language	source_id	reason
```

Sort its rows by language and source identifier after every change.

- [ ] **Step 2: Add word-language audit seeds**

Use these high-confidence seeds as the first case for each event type.

| Language | Threat | Harm wish | Self-harm command | Directed insult frame |
|---|---|---|---|---|
| EN | `I will kill you` | `I hope you die` | `kill yourself` | `you are a piece of shit` |
| ID | `aku akan membunuhmu` | `semoga kamu mati` | `mati saja kau` | `kamu bajingan` |
| PT | `vou te matar` | `tomara que você morra` | `mate-se` | `você é um filho da puta` |
| FR | `je vais te tuer` | `j’espère que tu crèves` | `tue-toi` | `tu es une ordure` |
| RU | `я тебя убью` | `чтоб ты сдох` | `убей себя` | `ты мразь` |
| DE | `ich werde dich töten` | `hoffentlich stirbst du` | `bring dich um` | `du bist ein Arschloch` |
| TR | `seni öldüreceğim` | `umarım ölürsün` | `kendini öldür` | `sen şerefsizsin` |
| VI | `tao sẽ giết mày` | `mong mày chết` | `tự tử đi` | `mày là đồ chó` |
| IT | `ti ucciderò` | `spero che tu muoia` | `ammazzati` | `sei una merda` |

- [ ] **Step 3: Add one dataset-backed variant per event type**

Each language shall have two toxic cases for each event type.

Use a development-data variant when a credible example exists.

Use clearly labeled authored evidence when the available development data has no credible event example.

Record the prepared source identifier in `evidence_ref` for every dataset-backed row.

Use development rows only. Reject validation and test identifiers.

Mark each rule-shaping dataset row as audit-only before metric preparation.

Record every inspected draft development row in `resources/datasets/rule-audit-v1.tsv`.

Record each inspected dataset row that changes a rule decision, including a rejected broad cue.

- [ ] **Step 4: Add clean twins and context controls**

Add two negations, two quotations, two linked reports, two counterspeech cases, and two violence questions.

Add six collision, Unicode, mixed-script, fiction, news, medical, or benign sentiment controls.

Record dataset, native-review, or clearly labeled authored evidence for every clean control.

- [ ] **Step 5: Implement conservative word packs**

List only the surface cues needed by the 24-row contract and validated development failures.

Add Turkish case-safe forms and Vietnamese tone-preserving phrases.

Do not add general stemming.

- [ ] **Step 6: Run word-pack behavior tests**

Evaluate these panels with `analyze_with_rules(word_rules(language), text, ReplyTarget::Unknown)`.

Run: `cargo test -p toxtrain --test behavior_panels word_languages`

Run: `cargo test --test rules_v2`

Expected: PASS for all nine word languages.

### Task 5: Add Arabic and Hindi rule packs and behavior panels

**Files:**

- Create: `src/rules/packs/arabic_hindi.rs`
- Create: `tests/fixtures/behavior/ar.tsv`
- Create: `tests/fixtures/behavior/hi.tsv`
- Modify: `crates/toxtrain/tests/behavior_panels.rs`
- Modify: `resources/datasets/rule-audit-v1.tsv`

**Interfaces:**

- Consumes: Arabic and Hindi normalization profiles.
- Produces: Two word-clause packs and two 24-row panels.

- [ ] **Step 1: Add Arabic and Hindi toxic seeds**

| Language | Threat | Harm wish | Self-harm command | Directed insult frame |
|---|---|---|---|---|
| AR | `سأقتلك` | `أتمنى أن تموت` | `اقتل نفسك` | `أنت حقير` |
| HI | `मैं तुम्हें मार दूंगा` | `काश तुम मर जाओ` | `आत्महत्या कर ले` | `तू हरामी है` |

- [ ] **Step 2: Add dataset-backed variants and 16 clean controls**

Arabic controls shall include MSA, one common social-Arabic form, and one Arabizi collision.

Both panels shall include at least one counterspeech control.

Use authored evidence until a native speaker reviews that exact control.

Hindi controls shall preserve Devanagari marks, ZWJ, and ZWNJ.

Record every inspected Arabic and Hindi draft development row in the rule-audit file.

- [ ] **Step 3: Implement small surface-form packs**

Arabic shall split only listed conjunction, future, and target suffix cues.

Hindi shall match only listed compound harm verbs, targets, negators, and reports.

- [ ] **Step 4: Run Arabic and Hindi panels**

Evaluate these panels with `analyze_with_rules(arabic_hindi_rules(language), text, ReplyTarget::Unknown)`.

Run: `cargo test -p toxtrain --test behavior_panels arabic_hindi`

Run: `cargo test --test profile_contract`

Expected: PASS.

### Task 6: Add Chinese, Japanese, and Korean compact packs and panels

**Files:**

- Create: `src/rules/packs/cjk.rs`
- Create: `src/rules/channel.rs`
- Modify: `src/rules/mod.rs`
- Modify: `src/lib.rs`
- Create: `tests/fixtures/behavior/zh.tsv`
- Create: `tests/fixtures/behavior/ja.tsv`
- Create: `tests/fixtures/behavior/ko.tsv`
- Modify: `crates/toxtrain/tests/behavior_panels.rs`
- Modify: `resources/datasets/rule-audit-v1.tsv`

**Interfaces:**

- Consumes: Compact clause views with code-point offsets.
- Produces: Three no-space packs, three 24-row panels, and the complete shared rule channel.

- [ ] **Step 1: Add CJK toxic seeds**

| Language | Threat | Harm wish | Self-harm command | Directed insult frame |
|---|---|---|---|---|
| ZH | `我要杀了你` | `希望你去死` | `自杀吧` | `你是畜生` |
| JA | `お前を殺してやる` | `死ねばいい` | `自殺しろ` | `お前はクズだ` |
| KO | `너를 죽이겠다` | `죽었으면 좋겠다` | `자살해` | `넌 쓰레기야` |

- [ ] **Step 2: Add dataset-backed variants and 16 clean controls**

Include inserted-space evasions, punctuation boundaries, mixed Latin text, negation, quotation, reporting, and counterspeech.

Record every inspected CJK draft development row in the rule-audit file.

- [ ] **Step 3: Implement code-point phrase matching**

```rust
pub fn find_compact_phrase(clause: &CompactClause<'_>, phrase: &[char]) -> Option<Range<usize>> {
    clause.codepoints.windows(phrase.len())
        .position(|window| window == phrase)
        .map(|start| clause.raw_range(start, start + phrase.len()))
}
```

- [ ] **Step 4: Keep CJK dictionary features disabled**

Do not enable Charabia Chinese, Japanese, or Korean dictionaries.

- [ ] **Step 5: Complete the shared rule channel**

```rust
pub(crate) enum ResolvedRules {
    Spanish(&'static RulePack),
    V2(&'static LanguageRules),
}

pub struct RuleChannel {
    language: Language,
    lexical: Option<Detector>,
    rules: ResolvedRules,
}

impl RuleChannel {
    pub fn from_hurtlex_bytes(
        language: Language,
        hurtlex: Option<&[u8]>,
    ) -> Result<Self, RuleChannelError>;

    pub fn analyze(&self, text: &str, reply_target: ReplyTarget) -> RuleOutcome;
}

pub fn canonical_rule_identity(language: Language) -> Vec<u8>;
```

Return one static pack for each non-Spanish language.

Load only conservative HurtLex entries. Expose no caller-selected HurtLex level in `RuleChannel`.

Add a test that proves an inclusive-only HurtLex entry cannot create a rule event.

For each new language, call `analyze_with_rules` with the supplied `ReplyTarget`.

Set the lexical score to `HURTLEX_SCORE` after any conservative match passes language and collision filters.

Set the lexical score to zero when no conservative HurtLex match remains.

A HurtLex match shall not supply a target, intent, imperative, wish, outcome, or strong-insult cue.

Set `RuleOutcome.score` to the larger semantic or lexical score. Never add both scores.

Set `should_nudge` from the composed score and `RULE_NUDGE_THRESHOLD`.

Preserve semantic and lexical evidence even when one score is lower.

Add one test where a HurtLex-only match returns score `30` and no nudge.

Add one test where a direct threat plus HurtLex returns score `95`.

Add one test where an explicit strong insult plus `ReplyTarget::Person` returns score `70`.

Route Spanish through the unchanged legacy pack and semantic policy.

Hash the Spanish legacy pack through the same ordered identity fields.

Use rule-pack version `1` for the frozen Spanish identity.

Test one frozen rule identity SHA-256 value for every language.

Make the compiler and `NudgeDetector` call `RuleChannel::analyze`.

- [ ] **Step 6: Run CJK panel and feature tests**

Evaluate these panels with `analyze_with_rules(cjk_rules(language), text, ReplyTarget::Unknown)`.

Run: `cargo test -p toxtrain --test behavior_panels cjk`

Run: `cargo test --test profile_contract`

Run: `cargo test --test rules_v2`

Expected: PASS.

### Task 7: Publish final data and compile all new models

**Files:**

- Create: `data/prepared-v1/`
- Create atomically: `resources/models/multilingual-v2/`

**Interfaces:**

- Consumes: Frozen raw sources, rule-audit exclusions, and frozen rule packs.
- Produces: Fourteen gate-passing version-two artifacts and one complete manifest.

- [ ] **Step 1: Publish the final audit-excluded data**

Run:

```bash
cargo run --release --locked -p toxtrain -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --output data/prepared-v1
```

Expected: Every listed development row has the audit-only exclusion reason.

- [ ] **Step 2: Compile all 14 models against final-path validation**

Run:

```bash
cargo run --release --locked -p toxtrain -- compile \
  --prepared-root data/prepared-v1 \
  --hurtlex-root data/raw-v1/hurtlex \
  --behavior-root tests/fixtures/behavior \
  --spanish-legacy resources/models/es-legacy-input-v1.json \
  --output resources/models/multilingual-v2
```

Expected: Every new language passes both validation gates.

- [ ] **Step 3: Stop on a frozen validation failure**

Do not inspect individual validation rows after a failed gate.

Do not change a rule pack or boundary from validation-error inspection.

Create a new prepared-data and validation version before another quality attempt.

- [ ] **Step 4: Verify artifact sizes and hashes**

Run: `find resources/models/multilingual-v2 -name '*-sparse-v2.bin' -exec stat -f '%N %z' {} \;`

Expected: Fourteen files. Each file has 131,112 bytes.

Run: `shasum -a 256 resources/models/multilingual-v2/*-sparse-v2.bin resources/models/multilingual-v2/es-chargram-v1.bin`

Expected: The printed values match `resources/models/multilingual-v2/manifest.json`.

- [ ] **Step 5: Prove atomic no-replace publication**

Stage all 15 artifacts and `manifest.json` in one sibling directory.

Flush and sync every staged file and the staging directory.

Publish `multilingual-v2` with `RenameFlags::NOREPLACE`.

Sync `resources/models` after the rename succeeds.

Reject an existing destination and preserve all existing bytes.

### Task 8: Embed the full registry and expose one runtime detector

**Files:**

- Modify: `src/registry.rs`
- Create: `src/runtime.rs`
- Modify: `src/policy.rs`
- Modify: `src/detector.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Create: `tests/runtime_registry.rs`
- Modify: `tests/toxcheck_cli.rs`

**Interfaces:**

- Consumes: All 15 artifacts, rule packs, expected hashes, and external HurtLex bytes.
- Produces: `NudgeDetector` and one explicit-language `toxcheck check` path.

- [ ] **Step 1: Write registry coverage and mismatch tests**

Place the private registry coverage test inside `src/registry.rs` under `#[cfg(test)]`.

Place the public runtime tests inside `tests/runtime_registry.rs`.

```rust
#[test]
fn registry_embeds_exactly_one_valid_model_per_language() {
    for language in Language::ALL {
        let entry = registry_entry(language);
        let model = entry.model().expect("embedded model");
        assert_eq!(model.language(), language);
        assert_eq!(model.feature_profile(), entry.feature_profile);
        assert_eq!(model.normalization_profile(), entry.normalization_profile);
        assert_eq!(model.feature_schema(), entry.feature_schema);
    }
}

#[test]
fn public_runtime_does_not_truncate_after_4096_bytes() {
    let detector = fixture_detector(Language::En).expect("detector");
    let text = format!("{} I will kill you", "neutral ".repeat(700));
    assert!(text.len() > 4_096);
    assert!(detector.check(&text, ReplyTarget::Unknown).should_nudge);
}

#[test]
fn every_language_keeps_the_public_result_invariant() {
    for language in Language::ALL {
        let hurtlex = std::fs::read(hurtlex_path(language)).expect("HurtLex fixture");
        let detector = NudgeDetector::from_hurtlex_bytes(
            language,
            Some(&hurtlex),
        ).expect("detector");
        for text in ["", "neutral message", "I will kill you"] {
            let result = detector.check(text, ReplyTarget::Unknown);
            assert!(result.score <= 100, "{}", language.code());
            assert_eq!(result.threshold, 50, "{}", language.code());
            assert_eq!(
                result.should_nudge,
                result.score >= result.threshold,
                "{}",
                language.code(),
            );
        }
    }
}
```

Resolve `hurtlex_path` as `data/raw-v1/hurtlex/{CODE}/1.2/hurtlex_{CODE}.tsv`.

- [ ] **Step 2: Run the registry test and confirm incomplete registry failure**

Run: `cargo test -p toxcheck --lib registry::tests::registry_embeds_exactly_one_valid_model_per_language`

Expected: FAIL until every artifact is registered.

- [ ] **Step 3: Add the complete cached registry**

Set every `include_bytes!` path under `resources/models/multilingual-v2`.

Test every embedded byte slice against the matching manifest digest.

```rust
pub(crate) struct RegistryEntry {
    pub language: Language,
    pub artifact: &'static [u8],
    pub artifact_sha256: [u8; 32],
    pub feature_profile: FeatureProfile,
    pub normalization_profile: NormalizationProfile,
    pub feature_schema: FeatureSchema,
    pub rule_pack_version: u16,
    pub rule_pack_sha256: [u8; 32],
    pub hurtlex_sha256: Option<[u8; 32]>,
    pub model: OnceLock<Result<SparseModel, SparseModelError>>,
    pub rules: RuleCache,
}

pub(crate) enum RuleCache {
    Spanish(OnceLock<RulePack>),
    V2(OnceLock<LanguageRules>),
}

impl RegistryEntry {
    pub(crate) fn model(&'static self) -> Result<&'static SparseModel, RuntimeInitError> {
        match self.model.get_or_init(|| SparseModel::from_bytes(self.artifact)) {
            Ok(model) => Ok(model),
            Err(source) => Err(RuntimeInitError::InvalidEmbeddedModel {
                language: self.language,
                reason: source.to_string(),
            }),
        }
    }
}
```

- [ ] **Step 4: Add the product runtime**

```rust
pub struct NudgeDetector {
    language: Language,
    model: &'static SparseModel,
    rule_channel: RuleChannel,
}

impl NudgeDetector {
    pub fn from_hurtlex_bytes(
        language: Language,
        hurtlex: Option<&[u8]>,
    ) -> Result<Self, RuntimeInitError>;

    pub fn analyze(&self, text: &str, reply_target: ReplyTarget) -> PolicyResult;

    pub fn check(&self, text: &str, reply_target: ReplyTarget) -> NudgeResult;
}
```

Map the sparse raw boundary to score `50` with the artifact scale.

Set the public score to `max(rule_channel.score, sparse_score)`.

Set `threshold` to `50` and derive `should_nudge` from the public score.

The product runtime shall load conservative HurtLex entries only.

Expose no caller-selected HurtLex level through `NudgeDetector` or `toxcheck`.

- [ ] **Step 5: Validate every behavior identity field**

Validate the artifact SHA-256, artifact profile IDs, rule-pack version, rule-pack SHA-256, and optional HurtLex SHA-256 during initialization.

Reject unknown languages and missing required HurtLex data.

Reject HurtLex bytes for a registry entry without an expected HurtLex hash.

Use no lexical matches when the registry entry has no HurtLex resource.

Resolve each acquired file as `{root}/{CODE}/1.2/hurtlex_{CODE}.tsv`.

Make `check` return `analyze(text, reply_target).nudge()`.

- [ ] **Step 6: Keep the CLI language explicit**

```bash
cargo run --release --locked --bin toxcheck -- check \
  --language TR \
  --data-dir data/raw-v1/hurtlex \
  --text "seni öldüreceğim"
```

The command shall reject `--language auto`.

- [ ] **Step 7: Run registry, CLI, and Spanish compatibility tests**

Run: `cargo test --test runtime_registry --test toxcheck_cli --test spanish_compatibility --test policy`

Expected: PASS.

### Task 9: Run validation and behavior evidence without test rows

**Files:**

- Create: `reports/multilingual-validation.json`
- Create: `reports/multilingual-behavior.json`
- Modify: `README.md`

**Interfaces:**

- Consumes: Frozen artifacts, validation splits, behavior panels, and the shipping runtime.
- Produces: Pre-test validation and behavior evidence.

- [ ] **Step 1: Evaluate the final validation path**

Run:

```bash
cargo run --release --locked -p toxtrain -- evaluate \
  --split validation \
  --prepared-root data/prepared-v1 \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --output reports/multilingual-validation.json
```

Expected: Each new language passes three-percent false warnings and 90-percent precision.

- [ ] **Step 2: Evaluate every behavior panel**

Run:

```bash
cargo run --release --locked -p toxtrain -- behavior \
  --fixture-root tests/fixtures/behavior \
  --prepared-root data/prepared-v1 \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --output reports/multilingual-behavior.json
```

Expected: All 336 new fixture decisions match.

- [ ] **Step 3: Run all pre-test Rust checks**

Run: `cargo fmt --check`

Expected: PASS.

Run: `cargo test --all-targets`

Expected: PASS.

Run: `cargo clippy --all-targets -- -D warnings`

Expected: PASS.

- [ ] **Step 4: Stop before any untouched test command**

Do not read `data/prepared-v1/*/test.tsv` in this plan.

The verification plan owns the freeze and one-time test run.
