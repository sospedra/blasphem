# Blasphem Corpus Single Source Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Make one committed, hand-editable TSV file per language the single source of truth for the corpus, and delete the raw corpus copy.

**Architecture:** The fold variant. Every metadata file that landed in `8cb14d8` through `6131ce6` stays: the source catalog, the source lock, the evaluation lock, `source_role`, and the community adapter. A new `corpus` module reads and writes the merged per-language file and produces the same `PreparedLanguageInput` the compiler already consumes, so the compiler changes one path and nothing downstream moves. The reproduce pipeline drops from nine steps to eight, because the corpus is no longer generated.

**Tech Stack:** Rust 1.97.0 (edition 2024), Cargo workspace, `csv` with a tab delimiter, `sha2`, `serde`, `clap`.

**Spec:** `docs/superpowers/specs/2026-09-03-blasphem-corpus-single-source-design.md`

**Status:** Complete. Tasks 1 to 8 landed in commits `1c6a6d9` through the Task 8 commit.

Four deviations from the written plan, each forced by the code:

1. `first_party_row_id` uses the digest helper in `evidence.rs`. The plan's `hex::encode` needs a `hex` dependency the workspace does not have.
2. `escape_text` also escapes a carriage return. 178 rows carry one, and an editor breaks a line on it.
3. `verify_corpus` exempts a row whose normalized text is empty. 37 punctuation-only rows normalize to nothing and collide with each other by construction. None is cross-split leakage.
4. `load_corpus_language` selects sources by `detector_language`, not by the `source` values in the file. HurtLex has no corpus rows, and the compiler requires exactly one HurtLex source per language.

Three additions the plan did not list, each required by Task 8's deletion:

1. `resources/datasets/behavior-provenance-v1.tsv` holds the 55 audit-only rows the behavior panels cite. Those rows are excluded from the corpus, so `data/prepared-draft-v1` was their only copy.
2. `evaluate` and `behavior` read `--corpus-root`. Their `--prepared-root` input no longer exists.
3. The three evidence reports were regenerated. `reports/multilingual-validation.json` gained Spanish; it had covered 14 languages since 2026-09-02.

## Global Constraints

- Corpus directory: `corpus/` at the repository root. One file per language, named by frozen storage code, so Malay is `corpus/ID.tsv`.
- Column order, tab separated, one header line: `row_id`, `source`, `split`, `label`, `origin_label`, `text`.
- `label` and `origin_label` values: `toxic` or `clean`. `origin_label` may be empty.
- `split` values: `development`, `validation`, `test`.
- Imported `row_id`: `<source_file_id>:<native_id>`. First-party `row_id`: `blasphem:<first ten hex characters of the SHA-256 of the normalized text>`.
- `text` is one line. A tab is written `\t`. A newline is written `\n`.
- Every file stays sorted by `row_id` after the header.
- `data/raw-v1/hurtlex/**` never moves. `crates/blasphem-wasm/src/lib.rs:249-263` embeds those 15 files with `include_bytes!`.
- **No new test files.** The user requires that new test files are opt-in and none were approved. Every test in this plan is a new case appended to an existing file under `crates/blasphem-train/tests/`.
- Tests assert observable behavior, never source strings and never implementation shape.
- Commit subjects only. No bodies. No trailers.
- Baseline to preserve: `cargo test --locked --workspace --no-fail-fast` passes 551 tests at `6131ce6`.
- **Do not unify `clean_control_identity` in `compiler.rs` onto `storage_code()`.** It keeps `code()` deliberately. Ruling R17 in `.superpowers/sdd/2026-09-02-blasphem-public-package-and-corpus/progress.md`. Changing it moves a Malay model hash.
- **Any dispatch over `DatasetId` must be total.** Ruling R55. An adapter can exist, compile, and pass its tests while never being reached, because dispatch lives in a separate `import_all_rows` match. That silently dropped every community row until `ba389ca`. Never add a catch-all arm to such a match; let the compiler catch the next omission.

---

## Precondition

Eight files are modified and uncommitted in the working tree, owned by session `blasphem-f7`: `Cargo.toml`, the four crate manifests, `src/detector.rs`, `src/text.rs`, `tests/detector.rs`. They are one change, a Rust review fix set: workspace dependency and lint tables, a removed third tokenizer pass in `check()`, `LexiconMatch.entry` behind an `Arc`, and a run-length encoded `CandidateView`. `cargo test --workspace --locked` passes on the dirty tree.

Do not edit those eight files. Do not commit them. Do not revert them. Their owner commits only when the user asks, and the user has not asked.

Work in this plan may proceed alongside them. `normalize_text` keeps its signature, so Task 3 is unaffected. If a task in this plan needs to change one of the eight, stop and raise it.

---

### Task 1: License year on every source record

**Files:**
- Modify: `crates/blasphem-train/src/source_manifest.rs`
- Modify: `resources/datasets/source-catalog-v1.json`
- Modify: `resources/datasets/source-lock-v1.json`
- Modify: `data/raw-v1/source-observation-v1.json`
- Test: `crates/blasphem-train/tests/source_manifest.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `SourceRequest.license_year: u16`, `FrozenSource.license_year: u16`, `SourceRecord.license_year: u16`.

- [x] **Step 1: Append the failing test to `crates/blasphem-train/tests/source_manifest.rs`**

```rust
#[test]
fn every_frozen_source_states_the_upstream_license_year() {
    let bytes = std::fs::read("../../resources/datasets/source-lock-v1.json").unwrap();
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(bytes.as_slice()).unwrap();

    assert_eq!(lock.sources.len(), 38);
    for source in &lock.sources {
        assert!(
            (1990..=2026).contains(&source.license_year),
            "{} has an implausible license year {}",
            source.source_file_id,
            source.license_year
        );
    }
}
```

- [x] **Step 2: Run it and confirm it fails**

Run: `cargo test -p blasphem-train --test source_manifest every_frozen_source_states -- --exact`
Expected: FAIL, compile error, no field `license_year`.

- [x] **Step 3: Add the field to the three structs**

In `crates/blasphem-train/src/source_manifest.rs`, add to `SourceRequest`, `FrozenSource`, and `SourceRecord`:

```rust
    pub license_year: u16,
```

Place it directly after `license_url` in each struct, so the serialized field order matches the reading order.

- [x] **Step 4: Fill the value in the three JSON files**

For each of the 38 records, add `"license_year": <year>` after `"license_url"`. The year is the year the upstream record states, never the year of resolution and never the acquisition date. When the record states no year, use the year of the pinned revision commit.

Read the year from each upstream record before writing it. Do not guess. The eight datasets are `hurtlex`, `textdetox`, `ibrohim-budi`, `told-br`, `offenseval-tr`, `vihos`, `k-mhas`, `germeval-2018`.

- [x] **Step 5: Run the test and the whole crate**

Run: `cargo test -p blasphem-train --locked`
Expected: PASS, including the 13 existing `source_manifest` tests.

- [x] **Step 6: Render the year in the notice**

In the NOTICE generator, add one line per section:

```
- License year: {license_year}
```

- [x] **Step 7: Regenerate and inspect**

Run the notice generation step, then read `NOTICE` and confirm each of the eight sections carries a year.

- [x] **Step 8: Commit**

```bash
git add crates/blasphem-train/src/source_manifest.rs crates/blasphem-train/tests/source_manifest.rs resources/datasets data/raw-v1/source-observation-v1.json NOTICE
git commit -m "Record the upstream license year on every source"
```

---

### Task 2: Close the frozen lock regeneration hole

**Files:**
- Modify: `resources/datasets/source-lock-v1.json`
- Test: `crates/blasphem-train/tests/source_manifest.rs`

**Interfaces:**
- Consumes: `FrozenSource.license_year` from Task 1.
- Produces: a lock that `freeze-sources` can regenerate without aborting.

`freeze-sources` aborts because `textdetox-es` carries no Parquet download digest, so the committed lock cannot be rebuilt from its inputs.

- [x] **Step 1: Reproduce the abort**

Run: `cargo run --release --locked -p blasphem-train -- freeze-sources --help`
Then run the freeze with the committed observation input and record the exact error text and the source identifier it names.

- [x] **Step 2: Append the failing test to `crates/blasphem-train/tests/source_manifest.rs`**

```rust
#[test]
fn every_textdetox_lock_entry_carries_a_download_digest() {
    let bytes = std::fs::read("../../resources/datasets/source-lock-v1.json").unwrap();
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(bytes.as_slice()).unwrap();

    let missing: Vec<&str> = lock
        .sources
        .iter()
        .filter(|source| source.dataset == blasphem_train::datasets::DatasetId::TextDetox)
        .filter(|source| source.download_sha256.is_none())
        .map(|source| source.source_file_id.as_str())
        .collect();

    assert_eq!(missing, Vec::<&str>::new());
}
```

- [x] **Step 3: Run it and confirm it fails**

Run: `cargo test -p blasphem-train --test source_manifest every_textdetox_lock_entry -- --exact`
Expected: FAIL, showing `["textdetox-es"]`.

- [x] **Step 4: Record the digest**

Download the pinned `textdetox-es` Parquet file at its recorded revision, compute its SHA-256, and write it into the `download_sha256` field of that lock entry. Record the same value in `data/raw-v1/source-observation-v1.json`.

- [x] **Step 5: Verify the freeze now regenerates the lock**

Run `freeze-sources` into a scratch path, then diff the output against `resources/datasets/source-lock-v1.json`.
Expected: no difference.

- [x] **Step 6: Commit**

```bash
git add resources/datasets/source-lock-v1.json data/raw-v1/source-observation-v1.json crates/blasphem-train/tests/source_manifest.rs
git commit -m "Record the Spanish TextDetox download digest so the lock regenerates"
```

---

### Task 3: The corpus module

**Files:**
- Create: `crates/blasphem-train/src/corpus.rs`
- Modify: `crates/blasphem-train/src/lib.rs`
- Test: `crates/blasphem-train/tests/prepared_input.rs`

**Interfaces:**
- Consumes: `blasphem::normalize_text`, `blasphem::{EvalLabel, Language}`, `crate::datasets::{DatasetSplit, PreparedRow, PreparedCounts}`, `crate::prepared_input::PreparedLanguageInput`, `crate::evidence::Sha256Digest`.
- Produces:
  - `pub struct CorpusRow { pub row_id: String, pub source: String, pub split: DatasetSplit, pub label: EvalLabel, pub origin_label: Option<EvalLabel>, pub text: String }`
  - `pub const CORPUS_HEADER: [&str; 6]`
  - `pub fn parse_corpus(reader: impl Read) -> Result<Vec<CorpusRow>, CorpusError>`
  - `pub fn write_corpus(writer: impl Write, rows: &[CorpusRow]) -> Result<(), CorpusError>`
  - `pub fn split_digest(rows: &[CorpusRow], split: DatasetSplit) -> Sha256Digest`
  - `pub fn first_party_row_id(text: &str) -> String`
  - `pub fn corpus_path(root: &Path, language: Language) -> PathBuf`
  - `pub fn load_corpus_language(root: &Path, language: Language, lock: &FrozenSourceLock) -> Result<PreparedLanguageInput, CorpusError>`
  - `pub enum CorpusError`

- [x] **Step 1: Append the failing round-trip test to `crates/blasphem-train/tests/prepared_input.rs`**

```rust
#[test]
fn a_corpus_file_round_trips_through_parse_and_write() {
    use blasphem::EvalLabel;
    use blasphem_train::corpus::{CorpusRow, parse_corpus, write_corpus};
    use blasphem_train::datasets::DatasetSplit;

    let rows = vec![
        CorpusRow {
            row_id: "blasphem:0a1b2c3d4e".to_string(),
            source: "blasphem".to_string(),
            split: DatasetSplit::Development,
            label: EvalLabel::Clean,
            origin_label: None,
            text: "una linea\tcon tabulador".to_string(),
        },
        CorpusRow {
            row_id: "kmhas-train:41822".to_string(),
            source: "kmhas-train".to_string(),
            split: DatasetSplit::Test,
            label: EvalLabel::Toxic,
            origin_label: Some(EvalLabel::Clean),
            text: "plain".to_string(),
        },
    ];

    let mut buffer = Vec::new();
    write_corpus(&mut buffer, &rows).unwrap();
    let parsed = parse_corpus(buffer.as_slice()).unwrap();

    assert_eq!(parsed, rows);
    assert_eq!(buffer.iter().filter(|byte| **byte == b'\n').count(), 3);
}
```

The last assertion proves the escape rule: three newlines means one header and two rows, so the embedded tab did not create a third row and no text newline leaked.

- [x] **Step 2: Run it and confirm it fails**

Run: `cargo test -p blasphem-train --test prepared_input a_corpus_file_round_trips -- --exact`
Expected: FAIL, unresolved module `corpus`.

- [x] **Step 3: Write the module**

Create `crates/blasphem-train/src/corpus.rs`:

```rust
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use blasphem::{EvalLabel, Language, normalize_text};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{datasets::DatasetSplit, evidence::Sha256Digest};

pub const CORPUS_HEADER: [&str; 6] = [
    "row_id",
    "source",
    "split",
    "label",
    "origin_label",
    "text",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRow {
    pub row_id: String,
    pub source: String,
    pub split: DatasetSplit,
    pub label: EvalLabel,
    pub origin_label: Option<EvalLabel>,
    pub text: String,
}

#[derive(Debug, Error)]
pub enum CorpusError {
    #[error("cannot read the corpus file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the corpus header is wrong: expected {expected:?}, got {actual:?}")]
    Header { expected: Vec<String>, actual: Vec<String> },
    #[error("row {row_id} has {actual} columns, expected 6")]
    ColumnCount { row_id: String, actual: usize },
    #[error("row {row_id} has an unknown {field} value {value}")]
    UnknownValue { row_id: String, field: &'static str, value: String },
    #[error("row {row_id} contains a raw tab or newline in its text")]
    UnescapedText { row_id: String },
}

#[must_use]
pub fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
}

pub fn unescape_text(row_id: &str, value: &str) -> Result<String, CorpusError> {
    if value.contains('\t') || value.contains('\n') {
        return Err(CorpusError::UnescapedText { row_id: row_id.to_string() });
    }
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('t') => output.push('\t'),
            Some('n') => output.push('\n'),
            Some('\\') => output.push('\\'),
            other => {
                output.push('\\');
                if let Some(value) = other {
                    output.push(value);
                }
            }
        }
    }
    Ok(output)
}

#[must_use]
pub fn first_party_row_id(text: &str) -> String {
    let digest = Sha256::digest(normalize_text(text).as_bytes());
    format!("blasphem:{}", &hex::encode(digest)[..10])
}

#[must_use]
pub fn corpus_path(root: &Path, language: Language) -> PathBuf {
    root.join(format!("{}.tsv", language.storage_code()))
}
```

Write `parse_corpus`, `write_corpus`, and `split_digest` in the same file. `parse_corpus` splits each line on `\t`, rejects a line that does not yield exactly six fields, and maps the empty string in column five to `None`. `write_corpus` writes the header, then each row, applying `escape_text` to the text column only. `split_digest` feeds the rows of one split, in file order, into one `Sha256` as `row_id\tlabel\ttext\n` per row and returns the digest.

Add `pub mod corpus;` to `crates/blasphem-train/src/lib.rs` in alphabetical position, after `community_corpus`.

- [x] **Step 4: Run the test**

Run: `cargo test -p blasphem-train --test prepared_input a_corpus_file_round_trips -- --exact`
Expected: PASS.

- [x] **Step 5: Append the loader test**

```rust
#[test]
fn loading_a_corpus_language_splits_development_from_validation() {
    use blasphem::{EvalLabel, Language};
    use blasphem_train::corpus::{CorpusRow, load_corpus_language, write_corpus};
    use blasphem_train::datasets::DatasetSplit;

    let directory = tempfile::tempdir().unwrap();
    let rows = vec![
        CorpusRow {
            row_id: "textdetox-en:1".to_string(),
            source: "textdetox-en".to_string(),
            split: DatasetSplit::Development,
            label: EvalLabel::Clean,
            origin_label: None,
            text: "one".to_string(),
        },
        CorpusRow {
            row_id: "textdetox-en:2".to_string(),
            source: "textdetox-en".to_string(),
            split: DatasetSplit::Validation,
            label: EvalLabel::Toxic,
            origin_label: None,
            text: "two".to_string(),
        },
    ];
    let file = std::fs::File::create(directory.path().join("EN.tsv")).unwrap();
    write_corpus(file, &rows).unwrap();

    let bytes = std::fs::read("../../resources/datasets/source-lock-v1.json").unwrap();
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(bytes.as_slice()).unwrap();
    let loaded = load_corpus_language(directory.path(), Language::En, &lock).unwrap();

    assert_eq!(loaded.development.len(), 1);
    assert_eq!(loaded.validation.len(), 1);
    assert_eq!(loaded.development[0].text, "one");
}
```

- [x] **Step 6: Implement `load_corpus_language` and run**

It reads the file at `corpus_path`, parses it, converts each row to a `PreparedRow`, partitions by split, counts into `PreparedCounts`, and collects the `SourceRecord` values whose `source_file_id` appears in the file.

Run: `cargo test -p blasphem-train --test prepared_input --locked`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add crates/blasphem-train/src/corpus.rs crates/blasphem-train/src/lib.rs crates/blasphem-train/tests/prepared_input.rs
git commit -m "Add the merged corpus reader and writer"
```

---

### Task 4: Migrate the corpus and reseed the evaluation lock

**Files:**
- Create: `corpus/AR.tsv` through `corpus/ZH.tsv`, 15 files
- Modify: `crates/blasphem-train/src/evaluation_lock.rs`
- Modify: `resources/datasets/evaluation-lock-v1.json`
- Test: `crates/blasphem-train/tests/evaluation_lock.rs`

**Interfaces:**
- Consumes: `corpus::{CorpusRow, write_corpus, split_digest}` from Task 3.
- Produces: 15 committed corpus files, and an evaluation lock whose digests cover row subsets rather than whole files.

- [x] **Step 1: Generate the merged files**

Run `prepare` against the committed source lock and `data/raw-v1`, writing to a scratch directory. Convert its three split files per language into one merged file per language, sorted by `row_id`, with `origin_label` empty on every row, because the migration changes no label.

- [x] **Step 2: Prove the migration preserves the sealed rows**

For each language, compute `split_digest` over the validation rows and over the test rows of the merged file, and compare against the row content behind the current `validation_sha256` and `test_sha256`. Any language whose row set differs blocks the migration.

- [x] **Step 3: Append the failing test to `crates/blasphem-train/tests/evaluation_lock.rs`**

```rust
#[test]
fn the_evaluation_lock_seals_the_committed_corpus_rows() {
    use blasphem::Language;
    use blasphem_train::corpus::{parse_corpus, split_digest};
    use blasphem_train::datasets::DatasetSplit;

    let bytes = std::fs::read("../../resources/datasets/evaluation-lock-v1.json").unwrap();
    let lock = blasphem_train::evaluation_lock::parse_evaluation_lock(bytes.as_slice()).unwrap();

    assert_eq!(lock.languages.len(), 15);
    for (code, sealed) in &lock.languages {
        let path = format!("../../corpus/{code}.tsv");
        let file = std::fs::File::open(&path).unwrap_or_else(|_| panic!("missing {path}"));
        let rows = parse_corpus(file).unwrap();

        assert_eq!(split_digest(&rows, DatasetSplit::Validation), sealed.validation_sha256);
        assert_eq!(split_digest(&rows, DatasetSplit::Test), sealed.test_sha256);
    }
    let _ = Language::En;
}
```

- [x] **Step 4: Run it and confirm it fails**

Run: `cargo test -p blasphem-train --test evaluation_lock the_evaluation_lock_seals -- --exact`
Expected: FAIL, missing `../../corpus/AR.tsv`.

- [x] **Step 5: Write the 15 files and reseed the lock**

Copy the merged files into `corpus/`. Recompute both digests per language with `split_digest` and write them into `resources/datasets/evaluation-lock-v1.json`.

- [x] **Step 6: Run the test**

Run: `cargo test -p blasphem-train --test evaluation_lock --locked`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add corpus resources/datasets/evaluation-lock-v1.json crates/blasphem-train/tests/evaluation_lock.rs crates/blasphem-train/src/evaluation_lock.rs
git commit -m "Commit the merged corpus and seal it by row"
```

---

### Task 5: The corpus verify command

**Files:**
- Modify: `crates/blasphem-train/src/corpus.rs`
- Modify: `crates/blasphem-train/src/main.rs`
- Test: `crates/blasphem-train/tests/cli.rs`

**Interfaces:**
- Consumes: everything from Tasks 3 and 4.
- Produces: `pub fn verify_corpus(root: &Path, lock: &FrozenSourceLock, evaluation: &EvaluationLock) -> Result<CorpusReport, CorpusError>` and a `CorpusVerify(CorpusVerifyArgs)` variant on the `Command` enum.

- [x] **Step 1: Append the failing test to `crates/blasphem-train/tests/cli.rs`**

```rust
#[test]
fn corpus_verify_passes_on_the_committed_corpus() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["corpus-verify", "--corpus-root", "../../corpus"])
        .args(["--source-lock", "../../resources/datasets/source-lock-v1.json"])
        .args(["--evaluation-lock", "../../resources/datasets/evaluation-lock-v1.json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stdout).contains("languages=15"));
}

#[test]
fn corpus_verify_rejects_an_edited_sealed_row() {
    let directory = tempfile::tempdir().unwrap();
    for entry in std::fs::read_dir("../../corpus").unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), directory.path().join(entry.file_name())).unwrap();
    }
    let path = directory.path().join("EN.tsv");
    let text = std::fs::read_to_string(&path).unwrap();
    let edited = text.replacen("\ttest\ttoxic\t", "\ttest\tclean\t", 1);
    assert_ne!(edited, text, "the fixture must contain a sealed toxic row");
    std::fs::write(&path, edited).unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["corpus-verify", "--corpus-root"])
        .arg(directory.path())
        .args(["--source-lock", "../../resources/datasets/source-lock-v1.json"])
        .args(["--evaluation-lock", "../../resources/datasets/evaluation-lock-v1.json"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("EN"));
}
```

- [x] **Step 2: Run both and confirm they fail**

Run: `cargo test -p blasphem-train --test cli corpus_verify`
Expected: FAIL, unrecognized subcommand.

- [x] **Step 3: Implement `verify_corpus`**

It checks, per language file: the header equals `CORPUS_HEADER`; every row has six columns; `row_id` is unique across the file; `normalize_text` of the text is unique across the file; `label` and `split` parse; the text carries no raw tab or newline; `source` names a `source_file_id` in the lock or equals `blasphem`; the rows are sorted by `row_id`; and both sealed digests match the evaluation lock.

It returns a `CorpusReport` carrying the language count and the row count.

- [x] **Step 4: Wire the subcommand**

Add to the `Command` enum in `crates/blasphem-train/src/main.rs`:

```rust
    CorpusVerify(CorpusVerifyArgs),
```

```rust
#[derive(Debug, clap::Args)]
struct CorpusVerifyArgs {
    #[arg(long)]
    corpus_root: PathBuf,
    #[arg(long)]
    source_lock: PathBuf,
    #[arg(long)]
    evaluation_lock: PathBuf,
}
```

The handler prints `languages=<count> rows=<count>` on success and exits non-zero with the failing language on error.

- [x] **Step 5: Run the tests**

Run: `cargo test -p blasphem-train --test cli --locked`
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add crates/blasphem-train/src/corpus.rs crates/blasphem-train/src/main.rs crates/blasphem-train/tests/cli.rs
git commit -m "Add the corpus verify command"
```

---

### Task 6: Point the compiler at the corpus

**Files:**
- Modify: `crates/blasphem-train/src/compiler.rs:39`, `:86`
- Modify: `crates/blasphem-train/src/main.rs`
- Test: `crates/blasphem-train/tests/compiler.rs`

**Interfaces:**
- Consumes: `corpus::load_corpus_language` from Task 3.
- Produces: `CompileOptions.corpus_root: PathBuf`, replacing `prepared_root`.

`load_corpus_language` returns the same `PreparedLanguageInput` that `load_prepared_language` returns, so `compile_prepared_language` changes one call and nothing downstream moves.

- [x] **Step 1: Append the failing test to `crates/blasphem-train/tests/compiler.rs`**

```rust
#[test]
fn compiling_from_the_committed_corpus_produces_a_model_for_every_language() {
    let options = blasphem_train::compiler::CompileOptions {
        corpus_root: std::path::PathBuf::from("../../corpus"),
        ..default_compile_options()
    };

    let models = blasphem_train::compiler::compile(&options).unwrap();

    assert_eq!(models.len(), 15);
}
```

Reuse the existing helper in that file for the remaining fields. If no helper exists, build the options inline from the same values the neighbouring tests use.

- [x] **Step 2: Run it and confirm it fails**

Run: `cargo test -p blasphem-train --test compiler compiling_from_the_committed_corpus -- --exact`
Expected: FAIL, no field `corpus_root`.

- [x] **Step 3: Rename the field and switch the loader**

In `crates/blasphem-train/src/compiler.rs`, rename `prepared_root` to `corpus_root` and replace the `load_prepared_language(&options.prepared_root, language)` call with `load_corpus_language(&options.corpus_root, language, lock)`.

Update the `Compile` handler in `main.rs` to pass `--corpus-root`.

Leave `clean_control_identity` alone. It calls `code()` on purpose, per ruling R17, and switching it to `storage_code()` moves the Malay clean-control hash.

- [x] **Step 4: Run the compiler tests**

Run: `cargo test -p blasphem-train --test compiler --locked`
Expected: PASS.

- [x] **Step 5: Prove the artifacts did not change**

Run the compile against `corpus/` and compare the output model hashes against `resources/models/multilingual-v2/manifest.json`.
Expected: identical hashes. A difference means the migration changed a training row.

- [x] **Step 6: Commit**

```bash
git add crates/blasphem-train/src/compiler.rs crates/blasphem-train/src/main.rs crates/blasphem-train/tests/compiler.rs
git commit -m "Compile the models from the committed corpus"
```

---

### Task 7: Reduce reproduce to eight steps

**Files:**
- Modify: `crates/blasphem-train/src/reproduce.rs:21-46`, `:111-124`, `:161-191`
- Modify: `crates/blasphem-train/src/regenerate.rs`
- Test: `crates/blasphem-train/tests/reproduce.rs`

**Interfaces:**
- Consumes: `corpus::verify_corpus` from Task 5.
- Produces: `STEP_NAMES: [&str; 8]`, `STEPS: [Step; 8]`, `GENERATION_STEPS: usize = 4`.

- [x] **Step 1: Append the failing test to `crates/blasphem-train/tests/reproduce.rs`**

```rust
#[test]
fn reproduction_verifies_the_corpus_instead_of_generating_it() {
    assert_eq!(blasphem_train::reproduce::STEP_NAMES.len(), 8);
    assert_eq!(blasphem_train::reproduce::STEP_NAMES[0], "verify-corpus");
    assert!(!blasphem_train::reproduce::STEP_NAMES.contains(&"generate-prepared-data"));
    assert_eq!(blasphem_train::reproduce::GENERATION_STEPS, 4);
}
```

- [x] **Step 2: Run it and confirm it fails**

Run: `cargo test -p blasphem-train --test reproduce reproduction_verifies_the_corpus -- --exact`
Expected: FAIL, left 9 right 8.

- [x] **Step 3: Replace the first two steps with one**

Delete `verify_raw_inputs` and `generate_prepared_data`. Add `verify_corpus_step`, which calls `corpus::verify_corpus` and then checks the 15 HurtLex files under `data/raw-v1/hurtlex` against the source lock.

Set `const RAW_ROOT: &str = "data/raw-v1"` aside; keep `HURTLEX_ROOT` unchanged. Add `const CORPUS_ROOT: &str = "corpus"`.

Point `verify_prepared_partitions` and `compile_model_artifacts` at `CORPUS_ROOT`.

Update `regenerate.rs` the same way, leaving its `HURTLEX_ROOT` untouched.

- [x] **Step 4: Run the reproduce tests**

Run: `cargo test -p blasphem-train --test reproduce --locked`
Expected: PASS.

- [x] **Step 5: Run the full reproduction**

Run: `cargo run --release --locked -p blasphem-train -- reproduce --skip-browser`
Expected: exit 0, `status=reproduced steps=8`.

- [x] **Step 6: Commit**

```bash
git add crates/blasphem-train/src/reproduce.rs crates/blasphem-train/src/regenerate.rs crates/blasphem-train/tests/reproduce.rs
git commit -m "Verify the corpus instead of regenerating it"
```

---

### Task 8: Delete the raw corpus and document the contract

**Files:**
- Delete: `data/raw-v1/datasets/**`, `data/raw-v1/textdetox/**`, `data/source-observation-v1/**`
- Modify: `.gitignore`
- Create: `corpus/README.md`
- Modify: `CONTRIBUTING.md`, `README.md`, `.github/workflows/`
- Test: `crates/blasphem-train/tests/reproduce.rs`

**Interfaces:**
- Consumes: every earlier task.
- Produces: a repository with one copy of every corpus row.

- [x] **Step 1: Confirm nothing else reads the deleted paths**

Run: `grep -rn "raw-v1/datasets\|raw-v1/textdetox\|prepared-v1\|source-observation-v1" --include="*.rs" --include="*.yml" --include="*.toml" . | grep -v "^./target/"`
Expected: only the call sites this task changes. `data/raw-v1/hurtlex` hits are correct and stay.

- [x] **Step 2: Delete**

```bash
git rm -r data/raw-v1/datasets data/raw-v1/textdetox data/source-observation-v1
```

Remove the `/data/prepared-v1/` and `/data/prepared-draft-v1/` lines from `.gitignore` and delete both directories from disk.

- [x] **Step 3: Write `corpus/README.md`**

State what the corpus is, the six columns, the escape rule, the sort rule, the sealed partitions, and the license of each file. `corpus/KO.tsv` is the strictest, through K-MHaS. Name the license of every other file from the source lock.

- [x] **Step 4: Update `CONTRIBUTING.md`**

Its current three-column community TSV path describes a file handed to `community_corpus.rs`. Add the direct path: add a row to `corpus/<LANG>.tsv` with `source` set to `blasphem` and `split` set to `development`, or correct a label by editing `label` and writing the previous value into `origin_label`. State that a validation or test row must not be edited, and that `corpus verify` is the gate.

- [x] **Step 5: Add the gate to CI**

Add a `corpus verify` invocation to the workflow, before the reproduction step. It fetches nothing.

- [x] **Step 6: Run everything**

```bash
cargo test --locked --workspace --no-fail-fast
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
```
Expected: 551 or more passing, 0 failing, clippy 0, fmt 0, reproduce exit 0.

- [x] **Step 7: Confirm the repository shrank**

Run: `du -sh data`
Expected: about 32 MB smaller than the baseline, with `data/raw-v1/hurtlex` intact.

- [x] **Step 8: Commit**

```bash
git add -u
git add corpus/README.md
git commit -m "Delete the raw corpus copy and document the corpus contract"
```

---

## Self-Review

**Spec coverage.** Goals: Tasks 4 and 8. Row schema: Task 3. Sort order: Tasks 3 and 5. Sealed partitions: Task 4. License year: Task 1. Reproducibility hole: Task 2. Commands: Task 5. Reproduce: Task 7. Deletions: Task 8. Attribution: Tasks 1 and 8. Contribution: Task 8. Turborepo: no task, because the spec states nothing depends on turbo landing. Migration: Task 4.

**Type consistency.** `CorpusRow`, `parse_corpus`, `write_corpus`, `split_digest`, `load_corpus_language`, and `verify_corpus` keep the same names in Tasks 3, 4, 5, 6, and 7. `corpus_root` replaces `prepared_root` in Task 6 and is used under that name in Task 7.

**Known gap.** Task 1 Step 4 and Task 2 Step 4 both require reading an upstream record over the network. Neither value may be guessed. If the network is unavailable, both tasks block rather than proceed.
