# Multilingual verification implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Verify all 15 language paths, freeze one behavior version, open test rows once, and publish quality, size, speed, and memory evidence.

**Architecture:** Use one final-path evaluator for validation, behavior cases, and untouched tests.
Hash every pre-test input into one freeze record.
Publish a claim before any test file opens.
Publish the test result through atomic no-replace publication.

**Tech stack:** Rust 2024, fixed-sample `Instant` timing, macOS `getrusage`, SHA-256, RFC 8785 JSON, and Markdown rendering.

**Spec:** `docs/superpowers/specs/2026-09-01-multilingual-sparse-nudge-detector-design.md`

## Global constraints

Complete the runtime plan and data plan first.

Complete Tasks 1 through 8 of the model plan next.

Complete Task 1 here before Task 9 of the model plan.

Complete Task 9 of the model plan before Task 2 here.

Do not open any new-language test split before the freeze task.

Apply gates per language. Pooled metrics shall remain informational.

Each new test split shall contain at least 300 clean and 300 toxic rows.

The new-language test false-warning rate shall not exceed three percent.

The new-language test precision shall not fall below 90 percent.

The untouched test shall have no recall floor.

Keep the existing Spanish evidence unchanged and outside the new gates.

Use `BTreeMap<String, T>` for every language-keyed evidence map.

Use uppercase language codes as map keys.

Resolve HurtLex files as `{root}/{CODE}/1.2/hurtlex_{CODE}.tsv`.

Verify each file against the matching model-manifest digest before detector initialization.

Serialize every evidence file with RFC 8785 JSON Canonicalization Scheme rules.

Hash the exact canonical bytes. Do not add a trailing newline.

Use one validated 64-character lowercase SHA-256 type in every evidence record.

The project directory is not a Git repository. Each task ends with a verification checkpoint instead of a commit.

---

## File structure

- Create `crates/toxtrain/src/verification.rs` for metrics, gates, and per-language evaluation.
- Extend `crates/toxtrain/src/evidence.rs` with canonical JSON serialization.
- Create `crates/toxtrain/src/benchmark.rs` for fixed-sample timing and memory.
- Create `crates/toxtrain/src/freeze.rs` for behavior-version hashes and sealed-test control.
- Create `crates/toxtrain/src/report.rs` for final Markdown rendering.
- Create benchmark fixtures under `tests/fixtures/benchmark`.
- Create freeze and report fixtures under `crates/toxtrain/tests/fixtures`.
- Reuse `crates/toxtrain/src/behavior_panel.rs` for all behavior parsing.
- Keep behavior integration tests in `crates/toxtrain/tests/behavior_panels.rs`.
- Create structured evidence under `reports`.
- Create `docs/multilingual-proof-report.md` as the user-facing result.
- Modify `crates/toxtrain/Cargo.toml` for `libc` and canonical JSON serialization.
- Reuse `crates/toxtrain/src/atomic_publish.rs` for freeze, claim, and test output publication.

### Task 1: Add per-language final-path verification

**Files:**

- Create: `crates/toxtrain/src/verification.rs`
- Modify: `crates/toxtrain/src/evidence.rs`
- Create: `crates/toxtrain/tests/final_path_evaluation.rs`
- Create: `reports/spanish-legacy-evidence.json`
- Modify: `crates/toxtrain/src/lib.rs`
- Modify: `crates/toxtrain/src/main.rs`
- Modify: `crates/toxtrain/src/calibration.rs`
- Modify: `crates/toxtrain/src/behavior_panel.rs`
- Modify: `crates/toxtrain/src/datasets/types.rs`
- Modify: `crates/toxtrain/tests/behavior_panels.rs`
- Modify: `crates/toxtrain/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/evaluation.rs`
- Modify: `src/language.rs`

**Interfaces:**

- Consumes: One initialized `NudgeDetector` and one prepared split.
- Produces: One `LanguageEvaluation` with counts, metrics, projected precision, and gate states.

- [ ] **Step 1: Write tolerance-based metric tests**

```rust
fn assert_close(actual: Option<f64>, expected: f64) {
    let actual = actual.expect("defined metric");
    let difference = (actual - expected).abs();
    assert!(difference <= 1.0e-12, "actual={actual} expected={expected}");
}

#[test]
fn metrics_and_projected_precision_use_the_documented_formulas() {
    let matrix = ConfusionMatrix {
        true_positive: 90,
        false_positive: 3,
        true_negative: 97,
        false_negative: 10,
    };
    let metrics = VerificationMetrics::from_matrix(matrix);
    assert_close(metrics.false_warning_rate, 3.0 / 100.0);
    assert_close(metrics.precision, 90.0 / 93.0);
    assert_close(metrics.recall, 90.0 / 100.0);
    assert_close(
        metrics.projected_precision_1_percent,
        (0.01 * 0.90) / (0.01 * 0.90 + 0.99 * 0.03),
    );
}
```

Do not use `assert_eq!` for a calculated floating-point metric.

- [ ] **Step 2: Write exact sample-size tests**

```rust
fn fixture_rows(language: Language, clean: usize, toxic: usize) -> Vec<PreparedRow> {
    let clean_rows = (0..clean).map(|index| PreparedRow {
        detector_language: language,
        label: EvalLabel::Clean,
        source_id: format!("clean-{index}"),
        text: format!("clean fixture {index}"),
    });
    let toxic_rows = (0..toxic).map(|index| PreparedRow {
        detector_language: language,
        label: EvalLabel::Toxic,
        source_id: format!("toxic-{index}"),
        text: format!("toxic fixture {index}"),
    });
    clean_rows.chain(toxic_rows).collect()
}

#[test]
fn evaluation_rejects_small_new_language_splits() {
    let rows = fixture_rows(Language::En, 299, 300);
    let error = validate_class_counts(Language::En, DatasetSplit::Validation, &rows)
        .expect_err("small clean class");
    assert!(matches!(error, VerificationError::InsufficientClassRows { .. }));
}
```

- [ ] **Step 3: Run evaluation tests and confirm the missing module failure**

Run: `cargo test -p toxtrain --test final_path_evaluation`

Expected: FAIL.

- [ ] **Step 4: Add shared canonical evidence primitives**

Add `serde_jcs = "0.2"` to `crates/toxtrain/Cargo.toml`.

Reuse `evidence::Sha256Digest` from the data plan. Do not declare another digest type.

Keep its existing 64-character lowercase hexadecimal validation.

Define `canonical_json_bytes<T: Serialize>` with `serde_jcs::to_vec`.

This function shall reject a serialization error. Evidence constructors shall reject non-finite floats first.

Define one typed canonical input reader. It must compare the input bytes with the reserialized canonical bytes.

Use this reader for each verification evidence input. Reject any byte difference.

Parse upstream manifests with their typed readers. Hash the exact upstream manifest bytes.

Add tests for invalid digests, duplicate members, whitespace, a trailing newline, and nested key order.

Add a test for reversed `BTreeMap` insertion order. Both canonical byte vectors must match.

- [ ] **Step 5: Add typed metric and evaluation records**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationMetrics {
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub specificity: Option<f64>,
    pub f1: Option<f64>,
    pub false_warning_rate: Option<f64>,
    pub projected_precision_1_percent: Option<f64>,
    pub projected_precision_5_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageEvaluation {
    pub language: Language,
    pub split: DatasetSplit,
    pub matrix: ConfusionMatrix,
    pub metrics: VerificationMetrics,
    pub gates: Option<GateResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEvidence {
    pub schema_version: u16,
    pub split: DatasetSplit,
    pub model_manifest_sha256: Sha256Digest,
    pub prepared_manifest_sha256: Sha256Digest,
    pub languages: BTreeMap<String, LanguageEvaluation>,
    pub pooled_matrix: ConfusionMatrix,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedTestEvidence {
    pub schema_version: u16,
    pub behavior_version: Sha256Digest,
    pub sealed_test_id: Sha256Digest,
    pub evaluation: EvaluationEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorCaseResult {
    pub case_id: String,
    pub text: String,
    pub event_type: EventType,
    pub pair_id: String,
    pub control_kind: ControlKind,
    pub evidence_kind: EvidenceKind,
    pub evidence_ref: String,
    pub expected_nudge: bool,
    pub actual_nudge: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageBehaviorResult {
    pub language: Language,
    pub passed: bool,
    pub cases: Vec<BehaviorCaseResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorEvidence {
    pub schema_version: u16,
    pub model_manifest_sha256: Sha256Digest,
    pub prepared_manifest_sha256: Sha256Digest,
    pub languages: BTreeMap<String, LanguageBehaviorResult>,
}
```

Calculate precision as `TP / (TP + FP)`. Use `None` when the denominator is zero.

Calculate recall as `TP / (TP + FN)`. Use `None` when the denominator is zero.

Calculate specificity as `TN / (TN + FP)`. Use `None` when the denominator is zero.

Calculate F1 as `2 * precision * recall / (precision + recall)`. Use `None` when an input is undefined.

Use `None` for F1 when the precision and recall sum is zero.

Calculate projected precision with the specification formula. Use `None` when its denominator is zero.

Serialize each `None` metric as JSON `null`.

Add a no-prediction test. It must prove that precision, F1, and both projected precision values are `None`.

Reuse the serialized `calibration::GateResult` type.

Add `Serialize` and `Deserialize` to the shared `ConfusionMatrix` type.

Add `Clone`, `Serialize`, and `Deserialize` to the behavior-panel enums.

Serialize `Language` as its uppercase two-letter code. Deserialize it through `Language::from_str`.

Serialize `DatasetSplit` as `development`, `validation`, or `test`.

Use `toxtrain::datasets::PreparedRow` for every evaluation input. Do not use the root `EvalRow` type.

Use `None` for Spanish gates. Reject `None` for every new language.

Reject a non-finite metric before JSON serialization.

- [ ] **Step 6: Evaluate the public Boolean path**

```rust
for row in rows {
    let predicted = detector.check(&row.text, ReplyTarget::Unknown).should_nudge;
    matrix.observe(row.label, predicted);
}
```

Do not call sparse scoring or rule scoring directly from the evaluator.

- [ ] **Step 7: Apply per-language sample and quality gates**

Require at least 300 rows from each class for every new language.

Use `u128` integer comparisons for the three-percent and 90-percent gates.

Do not gate Spanish through these new thresholds.

- [ ] **Step 8: Add the evaluation command**

```rust
#[derive(Clone, Copy, ValueEnum)]
enum EvaluationSplit {
    Validation,
    Test,
}

enum Command {
    Evaluate {
        split: EvaluationSplit,
        prepared_root: PathBuf,
        model_manifest: PathBuf,
        spanish_legacy: Option<PathBuf>,
        hurtlex_root: PathBuf,
        freeze: Option<PathBuf>,
        sealed_dir: Option<PathBuf>,
        output: Option<PathBuf>,
    },
    Behavior {
        fixture_root: PathBuf,
        prepared_root: PathBuf,
        model_manifest: PathBuf,
        hurtlex_root: PathBuf,
        output: PathBuf,
    },
}
```

Map `EvaluationSplit` to the shared `DatasetSplit` type after CLI parsing.

Require `spanish_legacy`, `freeze`, and `sealed_dir` for `split=test`.

Reject these three fields for validation. Require `output` for validation.

Reject `output` for test.

Use `evidence::canonical_json_bytes` for every evaluation output.

Write validation as `EvaluationEvidence`. Write test results as `SealedTestEvidence`.

Write behavior results as `BehaviorEvidence` through the final public Boolean path.

Require 14 language entries, 24 unique cases per language, and 336 passing cases.

Require every dataset evidence reference to map to one final `AuditOnly` provenance row.

- [ ] **Step 9: Record the frozen Spanish evidence**

Read and verify `resources/models/es-legacy-input-v1.json` first.

Copy the existing matrices into `reports/spanish-legacy-evidence.json`.

Use behavior matrix `TP=39`, `TN=46`, `FP=0`, and `FN=3`.

Use validation matrix `TP=159`, `TN=382`, `FP=18`, and `FN=203`.

Use test matrix `TP=177`, `TN=386`, `FP=14`, and `FN=242`.

Record proof-report SHA-256 `7634a66dfd43e22aac8d729ce5d06cbd1384aafe5f4ddb24c77a087039337d42`.

Record the existing Spanish artifact and panel hashes. Do not evaluate Spanish test rows again.

Record all 88 Spanish case results. Use the three documented toxic misses and the unchanged expected labels.

Use `behavior_panel::load_panel` for every new-language panel. Do not add another panel parser.

Extend `crates/toxtrain/tests/behavior_panels.rs` with final-path result checks and canonical evidence checks.

- [ ] **Step 10: Run final-path evaluation tests**

Run: `cargo test -p toxtrain --test final_path_evaluation`

Expected: PASS.

### Task 2: Add fixed benchmark fixtures and timing

**Files:**

- Create: `crates/toxtrain/src/benchmark.rs`
- Create: `crates/toxtrain/tests/benchmark_fixtures.rs`
- Create: `tests/fixtures/benchmark/messages.jsonl`
- Modify: `crates/toxtrain/Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/toxtrain/src/main.rs`

**Interfaces:**

- Consumes: 90 hashed fixtures and one initialized detector for each language.
- Produces: Per-fixture p50, p95, p99, maximum, throughput, and peak RSS.

- [ ] **Step 1: Write fixture dimension and length tests**

```rust
#[test]
fn benchmark_fixture_matrix_is_complete_and_exact() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/benchmark/messages.jsonl",
    ));
    let fixtures = load_benchmark_fixtures(path).expect("fixtures");
    assert_eq!(fixtures.len(), 90);
    assert_unique_dimensions(&fixtures).expect("unique dimensions");
    for fixture in fixtures {
        match fixture.length {
            FixtureLength::UnicodeScalars280 => assert_eq!(fixture.text.chars().count(), 280),
            FixtureLength::Utf8Bytes4096 => assert_eq!(fixture.text.as_bytes().len(), 4096),
        }
        assert_eq!(sha256_hex(fixture.text.as_bytes()), fixture.sha256);
    }
}
```

- [ ] **Step 2: Run fixture tests and confirm the missing fixture failure**

Run: `cargo test -p toxtrain --test benchmark_fixtures`

Expected: FAIL until all 90 fixtures exist.

- [ ] **Step 3: Add one clean, toxic, and dense fixture per language and length**

Use one natural seed per language and kind.

Pad short fixtures with neutral Unicode text until they contain exactly 280 scalar values.

Pad long fixtures with valid neutral UTF-8 text until they contain exactly 4,096 bytes.

Store the SHA-256 value of the decoded text bytes in each JSON line.

Serialize the fixture metadata with exact enum names. Reject unknown JSON fields.

- [ ] **Step 4: Add the fixed-sample timer**

```rust
fn measure(
    detector: &NudgeDetector,
    fixture: &BenchmarkFixture,
) -> TimingSummary {
    for _ in 0..100 {
        std::hint::black_box(detector.check(std::hint::black_box(&fixture.text), ReplyTarget::Unknown));
    }
    let samples = match fixture.length {
        FixtureLength::UnicodeScalars280 => 5_000,
        FixtureLength::Utf8Bytes4096 => 1_000,
    };
    collect_sorted_nanoseconds(detector, fixture, samples)
}
```

Initialize each language detector before any warm-up call. Reuse it for all six fixtures from that language.

Define this exact per-fixture result.

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingSummary {
    pub samples: u64,
    pub input_bytes: u64,
    pub p50_nanoseconds: u64,
    pub p95_nanoseconds: u64,
    pub p99_nanoseconds: u64,
    pub maximum_nanoseconds: u64,
    pub checks_per_second: f64,
    pub bytes_per_second: f64,
    pub peak_rss_bytes: u64,
}
```

Calculate checks per second as `samples * 1_000_000_000 / total_elapsed_nanoseconds`.

Calculate bytes per second as `samples * input_bytes * 1_000_000_000 / total_elapsed_nanoseconds`.

Retain the unrounded total elapsed nanoseconds from the same measured samples.

Reject a zero elapsed total or a non-finite calculated rate.

Add a formula test with 100 samples, 280 bytes, and one elapsed second.

Require 100 checks per second and 28,000 bytes per second within `1.0e-12`.

- [ ] **Step 5: Use nearest-rank percentiles**

```rust
fn nearest_rank(sorted: &[u64], percentile: usize) -> u64 {
    let rank = sorted.len().saturating_mul(percentile).div_ceil(100);
    sorted[rank.saturating_sub(1)]
}
```

- [ ] **Step 6: Apply every latency gate to every fixture**

Require `p95 < 1_000_000` nanoseconds for 280-scalar fixtures.

Require `p95 < 10_000_000` nanoseconds for 4,096-byte fixtures.

- [ ] **Step 7: Read concrete macOS peak RSS bytes**

Add this target dependency.

```toml
[target.'cfg(target_os = "macos")'.dependencies]
libc = "0.2"
```

Use one narrow `unsafe` call and one narrow initialization read.

```rust
#[cfg(target_os = "macos")]
fn peak_rss_bytes() -> Result<u64, BenchmarkError> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();

    // SAFETY: The pointer is valid and writable for one libc::rusage value.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return Err(BenchmarkError::PeakRss(std::io::Error::last_os_error()));
    }

    // SAFETY: A successful getrusage call initialized the complete value.
    let usage = unsafe { usage.assume_init() };
    u64::try_from(usage.ru_maxrss).map_err(|_| BenchmarkError::NegativePeakRss)
}

#[cfg(not(target_os = "macos"))]
fn peak_rss_bytes() -> Result<u64, BenchmarkError> {
    Err(BenchmarkError::UnsupportedPeakRssTarget)
}
```

macOS reports `ru_maxrss` in bytes. Do not multiply this value by 1,024.

Read peak RSS after every fixture finishes. Store the process-wide maximum as `peak_rss_bytes`.

Add one macOS test that requires a positive value. Other targets must return the named unsupported-target error.

- [ ] **Step 8: Add benchmark CLI output**

```bash
cargo run --release --locked -p toxtrain -- benchmark \
  --fixtures tests/fixtures/benchmark/messages.jsonl \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --output reports/multilingual-performance.json \
  --computer "MacBook Pro"
```

The runner shall record the computer name, Rust version, and target triple.

Use a `BTreeMap<String, TimingSummary>` keyed by fixture identifier.

- [ ] **Step 9: Run benchmark fixture tests**

Run: `cargo test -p toxtrain --test benchmark_fixtures`

Expected: PASS.

### Task 3: Add binary, artifact, and freeze records

**Files:**

- Create: `crates/toxtrain/src/size.rs`
- Create: `crates/toxtrain/src/freeze.rs`
- Create: `crates/toxtrain/tests/size.rs`
- Create: `crates/toxtrain/tests/freeze.rs`
- Create: `crates/toxtrain/tests/fixtures/freeze/complete-inputs.json`
- Create: `reports/multilingual-size.json`
- Create: `reports/freezes/`
- Modify: `crates/toxtrain/src/atomic_publish.rs`
- Modify: `crates/toxtrain/src/main.rs`

**Interfaces:**

- Consumes: The shipping binary, model set, HurtLex root, prepared manifest, and pre-test evidence.
- Produces: One immutable behavior version and sealed-test identifier.

- [ ] **Step 1: Write size and missing-input freeze tests**

```rust
#[test]
fn size_gate_uses_the_exact_shipping_limits() {
    assert!(check_binary_size(7_340_032).is_ok());
    assert!(check_binary_size(7_340_033).is_err());
    assert!(check_artifact_size(262_143).is_ok());
    assert!(check_artifact_size(262_144).is_err());
}

#[test]
fn freeze_rejects_one_missing_language_result() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/freeze/complete-inputs.json",
    ));
    let mut inputs = read_freeze_inputs(path).expect("complete freeze inputs");
    inputs.behavior.model_entries.remove("IT");
    let error = create_freeze(inputs).expect_err("missing Italian model entry");
    assert!(matches!(
        error,
        FreezeError::MissingLanguage {
            section: "model_entries",
            language: Language::It,
        }
    ));
}
```

Add a size test that rejects one missing artifact and one manifest digest mismatch.

Add a size test that requires all 15 HurtLex file records.

- [ ] **Step 2: Run freeze tests and confirm the missing module failure**

Run: `cargo test -p toxtrain --test size --test freeze`

Expected: FAIL.

- [ ] **Step 3: Build and measure the exact shipping target**

Run:

```bash
cargo build --release --locked --target aarch64-apple-darwin --bin toxcheck
stat -f '%z' target/aarch64-apple-darwin/release/toxcheck
```

Expected: The binary has at most 7,340,032 bytes.

- [ ] **Step 4: Record every artifact and external resource size**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSizeRecord {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SizeEvidence {
    pub schema_version: u16,
    pub target_triple: String,
    pub binary: FileSizeRecord,
    pub artifacts: BTreeMap<String, FileSizeRecord>,
    pub hurtlex: BTreeMap<String, FileSizeRecord>,
}

enum Command {
    Size {
        binary: PathBuf,
        model_manifest: PathBuf,
        hurtlex_root: PathBuf,
        target_triple: String,
        output: PathBuf,
    },
    Freeze {
        release_id: String,
        model_manifest: PathBuf,
        prepared_root: PathBuf,
        spanish_legacy: PathBuf,
        validation: PathBuf,
        behavior: PathBuf,
        performance: PathBuf,
        size: PathBuf,
        output_dir: PathBuf,
    },
}
```

The size JSON shall contain the binary, all 15 artifacts, and each external HurtLex file.

It shall contain one SHA-256 value for every recorded file.

Require both maps to contain every uppercase language code exactly once.

Run:

```bash
cargo run --release --locked -p toxtrain -- size \
  --binary target/aarch64-apple-darwin/release/toxcheck \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --target-triple aarch64-apple-darwin \
  --output reports/multilingual-size.json
```

Expected: The command writes canonical `SizeEvidence` and passes every size gate.

- [ ] **Step 5: Test canonical JSON coverage for freeze inputs**

Use `evidence::canonical_json_bytes` for the freeze input types.

Keep all language maps as `BTreeMap<String, T>`. Store no `HashMap` in an evidence type.

Reject an invalid digest, unknown field, missing field, extra language, and non-canonical fixture bytes.

- [ ] **Step 6: Define the complete freeze types**

```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenModelIdentity {
    pub artifact_sha256: Sha256Digest,
    pub feature_profile: FeatureProfile,
    pub normalization_profile: NormalizationProfile,
    pub feature_schema: FeatureSchema,
    pub rule_pack_version: u16,
    pub rule_pack_sha256: Sha256Digest,
    pub hurtlex_sha256: Option<Sha256Digest>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalBehaviorInputs {
    pub schema_version: u16,
    pub runtime_binary_sha256: Sha256Digest,
    pub model_manifest_sha256: Sha256Digest,
    pub prepared_manifest_sha256: Sha256Digest,
    pub spanish_legacy_sha256: Sha256Digest,
    pub validation_sha256: Sha256Digest,
    pub behavior_sha256: Sha256Digest,
    pub performance_sha256: Sha256Digest,
    pub size_sha256: Sha256Digest,
    pub model_entries: BTreeMap<String, FrozenModelIdentity>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSealedTestInputs {
    pub schema_version: u16,
    pub test_files: BTreeMap<String, PreparedFileIdentity>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreezeRecord {
    pub schema_version: u16,
    pub release_id: String,
    pub behavior_version: Sha256Digest,
    pub sealed_test_id: Sha256Digest,
    pub created_at_unix_seconds: u64,
    pub behavior: CanonicalBehaviorInputs,
    pub sealed_tests: CanonicalSealedTestInputs,
}
```

Require every `model_entries` map to contain all 15 uppercase codes.

Require `test_files` to contain the 14 new-language codes. Reject `ES` in this map.

Require at least 300 clean and 300 toxic rows in every sealed test identity.

Read `data/prepared-v1/manifest.json` once. Use its identities for `{CODE}/test.tsv`.

Reject any other test relative path. Do not open a `test.tsv` file.

Reuse `toxtrain::datasets::PreparedFileIdentity`. Require `rows == clean_rows + toxic_rows`.

Read `runtime_binary_sha256` from size evidence. Verify the size record against the shipping binary before freezing.

Read `rule_pack_sha256` from each model manifest entry. Reject a missing or invalid rule-pack digest.

- [ ] **Step 7: Generate both identifiers from canonical bytes**

Serialize `CanonicalBehaviorInputs` with `canonical_json_bytes`.

Use its complete SHA-256 value as `behavior_version`.

Serialize `CanonicalSealedTestInputs.test_files` independently.

Use the complete SHA-256 value of those canonical bytes as `sealed_test_id`.

Exclude `release_id` and `created_at_unix_seconds` from both identifier inputs.

Exclude `CanonicalSealedTestInputs.schema_version` from the sealed test identifier.

Add a test that inserts map values in reverse order. The canonical bytes and both identifiers must remain equal.

Add a test that changes only behavior inputs. The sealed test ID must remain equal.

Add a test that changes only the sealed-input schema version. The sealed test ID must remain equal.

Add a test that changes one test-file digest. The sealed test ID must change.

- [ ] **Step 8: Publish the freeze record through atomic no-replace publication**

Add `--release-id` and `--output-dir` to the freeze command.

Accept only lowercase ASCII letters, digits, and hyphens in a release identifier.

Set `FreezeRecord.release_id` from the validated command value.

Derive the final path as `{output_dir}/{release_id}.json`.

For this release, use `reports/freezes/multilingual-v2.json`.

Create one unique staging file in the destination directory. Use `create_new(true)` for that file.

Write all canonical bytes. Flush the writer. Call `sync_all` on the staged file.

Call `atomic_publish_noreplace` for the final rename. Never use a check-then-rename sequence.

Call `sync_all` on the destination directory after the rename succeeds.

If publication fails, remove only the staging file. Preserve any existing destination bytes.

Add tests for an invalid release identifier and the derived release path.

Add tests for an existing destination, a concurrent destination, a failed write, and staging cleanup.

- [ ] **Step 9: Run freeze tests**

Run: `cargo test -p toxtrain --test size --test freeze`

Expected: PASS.

### Task 4: Freeze and open the untouched tests once

**Files:**

- Create: `reports/sealed-tests/`
- Create: `crates/toxtrain/tests/sealed_test.rs`
- Modify: `crates/toxtrain/src/main.rs`

**Interfaces:**

- Consumes: One valid freeze record, 14 sealed test files, and the Spanish legacy record.
- Produces: One permanent claim and one atomic no-replace test result.

- [ ] **Step 1: Write exact sealed-test state tests**

| Test state | Required result |
|---|---|
| Missing or invalid freeze | No claim and no test-file open |
| Existing result output | No claim and no test-file open |
| Existing sealed-test claim | No test-file open |
| First valid attempt | The claim exists before the first test-file open |
| Two concurrent attempts | Exactly one claim succeeds and one reader starts |
| Test hash mismatch | The claim remains and no result appears |
| Concurrent result destination | Existing result bytes survive and the claim remains |

Use synthetic temporary splits and an injected test-file reader. Unit tests must not read `data/prepared-v1`.

- [ ] **Step 2: Run sealed-test tests and confirm the missing control failure**

Run: `cargo test -p toxtrain --test sealed_test`

Expected: FAIL.

- [ ] **Step 3: Generate the freeze before reading any test file**

Run:

```bash
cargo run --release --locked -p toxtrain -- freeze \
  --release-id multilingual-v2 \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --prepared-root data/prepared-v1 \
  --spanish-legacy reports/spanish-legacy-evidence.json \
  --validation reports/multilingual-validation.json \
  --behavior reports/multilingual-behavior.json \
  --performance reports/multilingual-performance.json \
  --size reports/multilingual-size.json \
  --output-dir reports/freezes
```

Expected: PASS with `reports/freezes/multilingual-v2.json` and both identifiers.

Reject an existing release path. A future final claim needs a new release identifier and a new test-file set.

- [ ] **Step 4: Claim the sealed test before opening a test file**

Validate the freeze schema, model-manifest hash, Spanish hash, and one prepared-manifest hash first.

Validate all 14 frozen test-file identities against that root prepared manifest.

Reject an existing result path before claim publication. Do not open a test file during these checks.

Derive the claim path as `reports/sealed-tests/{sealed_test_id}.claim.json`.

Derive the result path as `reports/sealed-tests/{sealed_test_id}.result.json`.

The same test-file identities shall always produce the same sealed test identifier.

A later final claim shall use a new test-file set and a new release identifier.

Use this exact claim type.

```rust
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedTestClaim {
    pub schema_version: u16,
    pub behavior_version: Sha256Digest,
    pub sealed_test_id: Sha256Digest,
    pub process_id: u32,
    pub started_at_unix_seconds: u64,
}
```

Serialize the claim with RFC 8785. Publish it through the Task 3 atomic no-replace writer.

If claim publication fails, return before any test-file open.

Never remove a published claim. A process failure consumes that sealed test ID.

- [ ] **Step 5: Open and evaluate each untouched test file once**

Open each of the 14 test files one time. Read each complete file into one byte buffer.

Hash and parse that same buffer. Compare its digest with the frozen test-file identity.

Evaluate every row through `NudgeDetector::check`. Apply both gates per new language.

Copy the Spanish test matrix from `reports/spanish-legacy-evidence.json`. Set its gates to `None`.

Write all 15 language records into the nested `evaluation` field of `SealedTestEvidence`.

Run:

```bash
cargo run --release --locked -p toxtrain -- evaluate \
  --split test \
  --freeze reports/freezes/multilingual-v2.json \
  --sealed-dir reports/sealed-tests \
  --prepared-root data/prepared-v1 \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --spanish-legacy reports/spanish-legacy-evidence.json
```

Expected: The command writes one canonical result through atomic no-replace publication.

Create one unique sibling staging file with `create_new(true)`. Do not check the destination before publication.

Write, flush, and sync the staging file. Call `atomic_publish_noreplace` for the final rename.

Sync the output directory after successful publication.

If result publication fails, preserve the claim. Remove only the unpublished staging file.

- [ ] **Step 6: Do not retune after a failed test gate**

If a test gate fails, record the failed result.

Publish the failed result. Require a new sealed test identifier before a later final quality claim.

- [ ] **Step 7: Run sealed-test unit tests after the one-time evaluation**

Run: `cargo test -p toxtrain --test sealed_test`

Expected: PASS without reading the real test output as a fixture.

### Task 5: Generate and verify the final report

**Files:**

- Create: `crates/toxtrain/src/report.rs`
- Create: `crates/toxtrain/tests/report.rs`
- Create: `crates/toxtrain/tests/fixtures/report/complete/`
- Create: `docs/multilingual-proof-report.md`
- Modify: `README.md`

**Interfaces:**

- Consumes: Model, prepared, Spanish, freeze, validation, behavior, performance, size, and sealed test JSON.
- Produces: One complete Markdown report and documented test commands.

- [ ] **Step 1: Add complete canonical report fixtures**

Create these files under `crates/toxtrain/tests/fixtures/report/complete`.

| File | Required coverage |
|---|---|
| `model-manifest.json` | All 15 languages |
| `prepared-manifest.json` | Every source-label family and frozen test identity |
| `spanish-legacy-evidence.json` | Spanish validation, test, and 88 behavior cases |
| `validation.json` | All 14 new languages |
| `behavior.json` | All 14 new-language panels and every case result |
| `performance.json` | All 90 benchmark dimensions |
| `size.json` | One binary, 15 artifacts, and the declared HurtLex files |
| `freeze.json` | All pre-test hashes and 14 sealed test identities |
| `test.json` | All 15 language results and both frozen identifiers |

Write each fixture as canonical JSON. Use internally matching SHA-256 values.

- [ ] **Step 2: Write exact report input tests**

Use the complete fixture directory as the valid case. Add one table case for each missing coverage item.

Reject an extra language, duplicate fixture identifier, unknown field, wrong digest, and mismatched sealed identifier.

Assert that a pooled pass cannot change a failed language status.

Assert that the rendered report contains `Versions`, `Quality`, `Behavior`, `Runtime`, and `Limits` sections.

Assert that every language has one separate external HurtLex byte value.

Assert that every imported source-label family appears in a policy-scope table.

- [ ] **Step 3: Run report tests and confirm the missing renderer failure**

Run: `cargo test -p toxtrain --test report`

Expected: FAIL.

- [ ] **Step 4: Validate every report input before rendering**

Recompute each raw input digest. Compare every pre-test digest with the freeze record.

Require the prepared-manifest digest to equal `freeze.behavior.prepared_manifest_sha256`.

Require the behavior evidence prepared-manifest digest to equal the same value.

Require both evaluation prepared-manifest digests to equal the same value.

Require the test behavior version and sealed test ID to equal the freeze values.

Resolve the test result as `{sealed_dir}/{freeze.sealed_test_id}.result.json`.

Require the model manifest path to be `resources/models/multilingual-v2/manifest.json` in the final command.

Require the prepared-data path to be `data/prepared-v1` in every documented command.

Combine the 14 new-language records with `spanish-legacy-evidence.json`. Never apply the new gates to Spanish.

- [ ] **Step 5: Render exact per-language version and quality tables**

List source, artifact, feature, normalization, rule-pack version, rule-pack hash, and HurtLex hash.

List each external HurtLex file byte size separately from the shipping binary.

List the runtime binary hash, behavior version, and sealed test ID.

List validation and untouched-test matrices and metrics for each language.

Mark every failed validation or test gate.

- [ ] **Step 6: Render behavior and runtime evidence**

Include the complete behavior result matrix.

Include binary size, artifact sizes, p50, p95, p99, maximum, throughput, and peak RSS.

Show checks per second and bytes per second as separate columns.

Add a policy-scope table for every source dataset and source-label family.

Mark labels outside the four rule events as `broader_than_nudge_policy`.

Explain that these broader labels can reduce measured precision against the product policy.

- [ ] **Step 7: Render fixed limitations**

State that scores are ordinal and not cross-language probabilities.

State that source labels differ and source prevalence is not production prevalence.

State that Chinese and French TextDetox lineage remains unresolved for this experimental spike.

- [ ] **Step 8: Generate the report from every declared input**

Run:

```bash
cargo run --release --locked -p toxtrain -- report \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --prepared-manifest data/prepared-v1/manifest.json \
  --spanish-legacy reports/spanish-legacy-evidence.json \
  --validation reports/multilingual-validation.json \
  --behavior reports/multilingual-behavior.json \
  --performance reports/multilingual-performance.json \
  --size reports/multilingual-size.json \
  --freeze reports/freezes/multilingual-v2.json \
  --sealed-dir reports/sealed-tests \
  --output docs/multilingual-proof-report.md
```

Expected: PASS with all 15 language rows.

Write the Markdown through a sibling staging file. Flush and sync before the final rename.

- [ ] **Step 9: Run final verification**

Run: `cargo fmt --check`

Expected: PASS.

Run: `cargo test --all-targets`

Expected: PASS.

Run: `cargo clippy --all-targets -- -D warnings`

Expected: PASS.

Run: `cargo build --release --locked --target aarch64-apple-darwin --bin toxcheck`

Expected: PASS with a binary at or below 7,340,032 bytes.
