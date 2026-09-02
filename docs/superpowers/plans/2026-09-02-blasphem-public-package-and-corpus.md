# Blasphem Public Package and Corpus Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename every first-party artifact to Blasphem, make the corpus and every generated artifact reproducible offline from a fresh clone, and publish the repository as `sospedra/blasphem` with a private npm package.

**Architecture:** The workspace keeps its current shape: a root `blasphem` library and CLI, four member crates, tracked raw corpora, and committed model artifacts. This plan renames the crates, commits the pinned upstream language tables, repairs two reproduction defects (Malay split identity, Spanish training path), adds a source-role and sealed-evaluation contract to the corpus pipeline, adds one read-only `reproduce` command plus one writing `regenerate` command, adds a private browser npm package, and delivers the first public commit with GitHub Actions.

**Tech Stack:** Rust 1.97.0 (edition 2024), Cargo workspace, `wasm-bindgen` 0.2.127, Node 24.18.0, npm 11.16.0, Playwright with its bundled Chromium, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-02-blasphem-public-package-and-corpus-design.md`

---

## Global Constraints

Every task's requirements implicitly include this section. Values are copied from the spec.

- Root Cargo package, library, and CLI name: `blasphem`.
- Language detector crate: package `blasphem-language`, library `blasphem_language`, directory `crates/blasphem-language`.
- Browser crate: package `blasphem-wasm`, library `blasphem_wasm`, directory `crates/blasphem-wasm`.
- Training crate: package `blasphem-train`, library `blasphem_train`, directory `crates/blasphem-train`.
- Evidence crate: package `blasphem-bench`, library `blasphem_bench`, directory `crates/blasphem-bench`.
- Language model builder binary name: `blasphem-language-model`.
- Language artifact file name: `blasphem-language-15-v1.bin`.
- Language artifact eight-byte magic: `BLASPHEM`.
- Npm package name: exact unscoped `blasphem`, `private` set to `true`.
- JavaScript constructor: `BlasphemDetector`. JavaScript result class: `BlasphemResult`.
- Generated browser files: `blasphem.js` and `blasphem_bg.wasm`.
- The term ELDC stays only in third-party attribution and pinned upstream records.
- Neutral domain names stay unchanged: HurtLex, language, corpus, model, detector.
- The sparse artifact magics `TOXSPRS1` and `TOXSPRS2` and the clean-control domain tag `TOXCLEAN1` stay unchanged. They are frozen on-disk formats, not package names. Changing them changes every model hash.
- Vendored upstream headers live under `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8`.
- Malay public code is `MS`. Malay frozen storage code is `ID`. Corpus split hashing uses the storage code.
- Sealed partition hashes live in `resources/datasets/evaluation-lock-v1.json`.
- Source roles: `baseline`, `training_only`, `sealed_evaluation`.
- Canonical corpus contribution schema: `native_id\tlabel\ttext`. Labels: `toxic` or `clean`.
- Rule-evidence rows live in `resources/datasets/rule-audit-v1.tsv`.
- Reproduction command: `cargo run --release --locked -p blasphem-train -- reproduce`.
- Update command: `cargo run --release --locked -p blasphem-train -- regenerate`.
- Canonical identity target: `x86_64-unknown-linux-gnu` in GitHub Actions.
- Optional native byte-comparison target: `aarch64-apple-darwin`.
- First-party code license: Apache License 2.0.
- Public repository: `sospedra/blasphem`. Local `main` pushes to `origin`.
- No npm publish command runs.

---

## Verified Baseline

Run before Task 1. These facts were measured on 2026-09-02 and the plan depends on them.

| Fact | Check | Result |
| --- | --- | --- |
| Workspace compiles | `cargo check --workspace --all-targets --locked` | exit 0 |
| Rust | `rustc --version` | `1.97.0 (2d8144b78 2026-07-07)` |
| Node / npm | `node --version` / `npm --version` | `v24.18.0` / `11.16.0` |
| wasm-bindgen CLI | `wasm-bindgen --version` | `0.2.127` |
| Git history | `git log --oneline` | one commit, one tracked file |
| Raw sources in lock | `resources/datasets/source-lock-v1.json` | 37 |
| Prepared data regenerates | `cargo run --release --locked -p toxtrain -- prepare ...` | **FAILS** |

The preparation failure is the Malay defect this plan repairs:

```text
Error: audit-only source identifier is not a development row:
ibrohim-budi@be98de98e974b65838d2b5145ee2c89e9bf53a6b/unsplit/000382
```

`split_hash` in `crates/toxtrain/src/datasets/prepare.rs:121` hashes `language.code()`, which returns `MS`. The frozen partition in `data/prepared-v1/ID` was built when the code was `ID`. Row `unsplit/000382` was a development row under `ID`; under `MS` it lands in another split, so `validate_audit_only_source_ids` rejects it. Task 4 fixes this.

---

## File Structure

Directories renamed (git mv, contents unchanged unless a task says otherwise):

| From | To |
| --- | --- |
| `crates/eldc` | `crates/blasphem-language` |
| `crates/toxcheck-wasm` | `crates/blasphem-wasm` |
| `crates/toxtrain` | `crates/blasphem-train` |
| `crates/toxbench` | `crates/blasphem-bench` |
| `crates/eldc/src/bin/import-eldc.rs` | `crates/blasphem-language/src/bin/blasphem-language-model.rs` |
| `crates/eldc/data/eldc-15-v1.bin` | `crates/blasphem-language/data/blasphem-language-15-v1.bin` |
| `data/textdetox/es-source.tsv` | `data/raw-v1/textdetox/es.tsv` |

Files created:

| Path | Responsibility |
| --- | --- |
| `tests/rename_contract.rs` | Fails while an old first-party name lives in an active source file. |
| `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8/*.h` | The four pinned upstream tables. |
| `resources/models/language-artifact-v1.json` | Language artifact identity: path, bytes, SHA-256, source commit, header digests. |
| `resources/datasets/evaluation-lock-v1.json` | Sealed validation and test partition hashes for 15 languages. |
| `crates/blasphem-train/src/source_role.rs` | The `SourceRole` enum and its ingestion rules. |
| `crates/blasphem-train/src/evaluation_lock.rs` | Sealed-hash parse, compute, and compare. |
| `crates/blasphem-train/src/community_corpus.rs` | The canonical `native_id\tlabel\ttext` adapter. |
| `crates/blasphem-train/src/reproduce.rs` | The nine read-only reproduction steps. |
| `crates/blasphem-train/src/regenerate.rs` | The writing update path. |
| `rust-toolchain.toml` | Pins the Rust toolchain and both extra targets. |
| `.nvmrc` | Pins Node. |
| `packages/blasphem/package.json` | The private npm package. |
| `packages/blasphem/scripts/build.mjs` | Builds `dist/blasphem.js` and `dist/blasphem_bg.wasm`. |
| `packages/blasphem/scripts/pack-check.mjs` | Inspects the npm archive without publishing. |
| `packages/blasphem/scripts/browser-smoke.mjs` | Drives pinned Playwright Chromium against `dist`. |
| `packages/blasphem/index.d.ts` | TypeScript declarations. |
| `packages/blasphem/NOTICE` | Third-party notices for embedded data. |
| `LICENSE` | Apache License 2.0 text. |
| `NOTICE` | Repository-level third-party attribution. |
| `CONTRIBUTING.md` | Both corpus contribution paths. |
| `.github/workflows/ci.yml` | Format, tests, Clippy, reproduce, npm pack, browser smoke. |

Files excluded by `.gitignore`: `/target/`, `/data/prepared-v1/`, `/data/prepared-draft-v1/`, `/.superpowers/`, `/packages/blasphem/dist/`, `/packages/blasphem/node_modules/`, `/packages/blasphem/*.tgz`.

---

## Phases

Tasks run in order. Each phase leaves the workspace green.

1. **Naming** — Tasks 1 to 3.
2. **Reproduction corrections** — Tasks 4 to 7.
3. **Corpus contract** — Tasks 8 to 10.
4. **Reproduce and regenerate** — Tasks 11 to 13.
5. **Npm package** — Tasks 14 to 15.
6. **Delivery** — Tasks 16 to 18.

---

### Task 1: Rename contract test and workspace crate rename

Renames the five first-party packages. The contract test is written first and must fail before the rename.

**Files:**
- Create: `tests/rename_contract.rs`
- Modify: `Cargo.toml`
- Modify: `crates/eldc/Cargo.toml` → `crates/blasphem-language/Cargo.toml`
- Modify: `crates/toxcheck-wasm/Cargo.toml` → `crates/blasphem-wasm/Cargo.toml`
- Modify: `crates/toxtrain/Cargo.toml` → `crates/blasphem-train/Cargo.toml`
- Modify: `crates/toxbench/Cargo.toml` → `crates/blasphem-bench/Cargo.toml`
- Modify: `.cargo/config.toml`
- Modify: every `.rs` file that names an old package (72 files for `toxcheck` alone)

**Interfaces:**
- Produces: crate paths `blasphem`, `blasphem_language`, `blasphem_wasm`, `blasphem_train`, `blasphem_bench`. Every later task uses these.
- Produces: binary names `blasphem`, `blasphem-train`, `blasphem-bench`, `blasphem-language-model`.
- Produces: `CARGO_BIN_EXE_blasphem`, `CARGO_BIN_EXE_blasphem-train`, `CARGO_BIN_EXE_blasphem-bench`.

- [ ] **Step 1: Write the failing contract test**

Create `tests/rename_contract.rs`:

```rust
use std::{fs, path::Path};

/// Old first-party identifiers that must not survive in active source.
const RETIRED_NAMES: [&str; 8] = [
    "toxcheck",
    "toxtrain",
    "toxbench",
    "toxcheck-wasm",
    "toxcheck_wasm",
    "eldc",
    "ELDC",
    "import-eldc",
];

/// Directories that hold generated output, history, or third-party records.
const SKIPPED_DIRECTORIES: [&str; 6] = [
    ".git",
    ".superpowers",
    "target",
    "node_modules",
    "vendor",
    "dist",
];

/// Files that keep the upstream name as attribution or as a pinned record.
const ATTRIBUTION_FILES: [&str; 4] = [
    "crates/blasphem-language/UPSTREAM.md",
    "crates/blasphem-language/FORMAT.md",
    "NOTICE",
    "packages/blasphem/NOTICE",
];

fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn is_scanned(relative: &str) -> bool {
    if ATTRIBUTION_FILES.contains(&relative) {
        return false;
    }
    if relative.starts_with("docs/") || relative.starts_with("reports/") {
        return false;
    }
    let extension = Path::new(relative).extension().and_then(|value| value.to_str());
    matches!(
        extension,
        Some("rs" | "toml" | "md" | "json" | "mjs" | "js" | "ts" | "sh" | "yml" | "html")
    )
}

fn collect(directory: &Path, root: &Path, found: &mut Vec<String>) {
    let entries = fs::read_dir(directory).expect("readable directory");
    for entry in entries {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_str().expect("UTF-8 file name");
        if path.is_dir() {
            if !SKIPPED_DIRECTORIES.contains(&name) {
                collect(&path, root, found);
            }
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("path inside the project root")
            .to_str()
            .expect("UTF-8 relative path")
            .to_owned();
        if !is_scanned(&relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for retired in RETIRED_NAMES {
            if text.contains(retired) {
                found.push(format!("{relative}: {retired}"));
            }
        }
    }
}

#[test]
fn active_source_uses_only_blasphem_names() {
    let root = project_root();
    let mut found = Vec::new();
    collect(root, root, &mut found);
    found.sort();
    assert!(
        found.is_empty(),
        "retired first-party names remain in active source:\n{}",
        found.join("\n")
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --locked --test rename_contract`
Expected: FAIL, listing more than 100 occurrences across `Cargo.toml`, `src/`, and `crates/`.

- [ ] **Step 3: Move the crate directories**

```bash
git mv crates/eldc crates/blasphem-language
git mv crates/toxcheck-wasm crates/blasphem-wasm
git mv crates/toxtrain crates/blasphem-train
git mv crates/toxbench crates/blasphem-bench
git mv crates/blasphem-language/src/bin/import-eldc.rs \
       crates/blasphem-language/src/bin/blasphem-language-model.rs
```

If `git mv` refuses because the paths are untracked, use `mv` instead. The repository has one commit and one tracked file.

- [ ] **Step 4: Rewrite the manifests**

Set `Cargo.toml` at the root to:

```toml
[package]
name = "blasphem"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
publish = false

[workspace]
members = [".", "crates/blasphem-language", "crates/blasphem-wasm", "crates/blasphem-bench", "crates/blasphem-train"]
default-members = [".", "crates/blasphem-language", "crates/blasphem-wasm", "crates/blasphem-bench", "crates/blasphem-train"]
resolver = "2"

[features]
default = ["language-detection"]
language-detection = ["dep:blasphem-language"]

[dependencies]
aho-corasick = "1.1"
anyhow = "1.0"
charabia = { version = "0.9.9", default-features = false }
clap = { version = "4.5", features = ["derive"] }
csv = "1.3"
blasphem-language = { path = "crates/blasphem-language", optional = true }
serde = { version = "1.0", features = ["derive"] }
sha2 = "0.10"
thiserror = "2.0"
unicode-security = "0.1.2"
unicode-normalization = "0.1"
unicode-segmentation = "1.12"
unicode-general-category = "1.0"
unicode-script = "0.5"

[dev-dependencies]
serde_json = "1.0"
tempfile = "3.20"

[profile.release]
lto = "thin"
codegen-units = 1
strip = true

[lints.clippy]
correctness = "deny"
suspicious = "warn"
style = "warn"
complexity = "warn"
perf = "warn"
```

In the four member manifests set `name` to `blasphem-language`, `blasphem-wasm`, `blasphem-train`, `blasphem-bench`. In each, change the dependency line `toxcheck = { path = ... }` to `blasphem = { path = ... }`. In `crates/blasphem-wasm/Cargo.toml` change `language-detection = ["toxcheck/language-detection"]` to `language-detection = ["blasphem/language-detection"]`.

Set `.cargo/config.toml` to:

```toml
[alias]
perf-gate = "test --release --locked -p blasphem-bench --test dense_runtime_regression"
```

- [ ] **Step 5: Rewrite the Rust source names**

Order matters. Longer and hyphenated tokens go first.

```bash
FILES=$(git ls-files -co --exclude-standard \
  | grep -E '\.(rs|toml|sh|mjs|html)$' \
  | grep -v '^\.superpowers/' \
  | grep -v '^crates/blasphem-language/vendor/')

for f in $FILES; do
  perl -pi -e '
    s/import-eldc/blasphem-language-model/g;
    s/eldc-15-v1\.bin/blasphem-language-15-v1.bin/g;
    s/toxcheck_wasm/blasphem_wasm/g;
    s/toxcheck-wasm/blasphem-wasm/g;
    s/toxtrain::/blasphem_train::/g;
    s/toxbench::/blasphem_bench::/g;
    s/eldc::/blasphem_language::/g;
    s/CARGO_BIN_EXE_toxtrain/CARGO_BIN_EXE_blasphem-train/g;
    s/CARGO_BIN_EXE_toxbench/CARGO_BIN_EXE_blasphem-bench/g;
    s/CARGO_BIN_EXE_toxcheck/CARGO_BIN_EXE_blasphem/g;
    s/\btoxtrain\b/blasphem-train/g;
    s/\btoxbench\b/blasphem-bench/g;
    s/\btoxcheck\b/blasphem/g;
  ' "$f"
done
```

`crates/blasphem-language/UPSTREAM.md`, `crates/blasphem-language/FORMAT.md`, and the `tools/` scripts keep the upstream ELDC name. They are attribution and pinned records. Confirm the loop left them alone:

```bash
grep -c ELDC crates/blasphem-language/UPSTREAM.md
```
Expected: a nonzero count.

- [ ] **Step 6: Fix the residue by hand**

```bash
cargo check --workspace --all-targets --locked 2>&1 | grep -E '^(error|warning: unused)' | head -40
```

Expected residue and its fix:

- `src/main.rs:14` still reads `name = "toxcheck"` if the word boundary missed it. Set it to `name = "blasphem"`.
- `crates/blasphem-bench/src/main.rs:9` reads `about = "Experimental toxcheck runtime evidence"`. Set it to `about = "Experimental blasphem runtime evidence"`.
- `crates/blasphem-bench/src/auto.rs:653` holds the string `cargo tree -p toxcheck-wasm --no-default-features -e normal`. Set it to `cargo tree -p blasphem-wasm --no-default-features -e normal`.
- `crates/blasphem-bench/tests/size_contract.rs:20` and `crates/blasphem-bench/tests/cli.rs:9` join a temporary path named `toxcheck`. Set both to `blasphem`.
- `crates/blasphem-language/src/lib.rs` keeps `ELDC` in its error text. Those messages describe the upstream format. Reword each to say "language model" instead of "ELDC model", because error text is active source, not attribution. Example: `"the ELDC model magic is invalid"` becomes `"the language model magic is invalid"`.

Repeat until `cargo check --workspace --all-targets --locked` exits 0.

- [ ] **Step 7: Refresh the lock file**

```bash
cargo update --workspace --offline 2>/dev/null || cargo metadata --format-version 1 >/dev/null
git diff --stat Cargo.lock
```

Expected: `Cargo.lock` records the five new package names.

- [ ] **Step 8: Run the contract test and the suite**

Run: `cargo test --locked --test rename_contract`
Expected: PASS

Run: `cargo test --workspace --locked`
Expected: PASS. Tests that read `crates/eldc/data/eldc-15-v1.bin` now read `crates/blasphem-language/data/blasphem-language-15-v1.bin`. If a test still fails on a path, fix the path, not the test assertion.

Run: `cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: exit 0

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "Rename first-party packages to Blasphem"
```

---

### Task 2: Language artifact magic and identity record

Changes the eight-byte magic to `BLASPHEM`, rebuilds the artifact, and records its identity.

**Files:**
- Modify: `crates/blasphem-language/src/lib.rs:6`
- Modify: `crates/blasphem-language/src/tests.rs:11,40`
- Modify: `crates/blasphem-language/src/bin/blasphem-language-model.rs`
- Modify: `crates/blasphem-language/FORMAT.md`
- Create: `resources/models/language-artifact-v1.json`
- Modify: `crates/blasphem-language/data/blasphem-language-15-v1.bin` (regenerated)

**Interfaces:**
- Consumes: the renamed crate from Task 1.
- Produces: `resources/models/language-artifact-v1.json` with fields `schema_version`, `artifact_relative_path`, `artifact_bytes`, `artifact_sha256`, `source_commit`, `source_headers` (array of `{ file_name, sha256 }`). Task 7 and Task 12 read it.

- [ ] **Step 1: Write the failing magic test**

Add to `crates/blasphem-language/src/tests.rs`:

```rust
#[test]
fn the_committed_artifact_uses_the_blasphem_magic() {
    let bytes = include_bytes!("../data/blasphem-language-15-v1.bin");
    assert_eq!(&bytes[..8], b"BLASPHEM");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-language the_committed_artifact_uses_the_blasphem_magic`
Expected: FAIL, left `[69, 76, 68, 67, 49, 53, 0, 0]` (`ELDC15\0\0`).

- [ ] **Step 3: Change the magic**

In `crates/blasphem-language/src/lib.rs:6`:

```rust
const MAGIC: &[u8; 8] = b"BLASPHEM";
```

In `crates/blasphem-language/src/tests.rs`, replace both `bytes.extend_from_slice(b"ELDC15\0\0");` with `bytes.extend_from_slice(b"BLASPHEM");`.

In `crates/blasphem-language/src/bin/blasphem-language-model.rs`, replace the literal that writes the magic with `b"BLASPHEM"`. Find it with:

```bash
grep -n 'ELDC15' crates/blasphem-language/src/bin/blasphem-language-model.rs
```

In `crates/blasphem-language/FORMAT.md`, change the magic row to `BLASPHEM` and keep the upstream attribution paragraph.

- [ ] **Step 4: Rebuild the artifact**

Task 7 commits the vendored headers. Until then, build from the local checkout that matches the pinned digests:

```bash
cargo run --release --locked -p blasphem-language --bin blasphem-language-model -- \
  /private/tmp/eldc-main/src/eldc \
  crates/blasphem-language/data/blasphem-language-15-v1.bin
shasum -a 256 crates/blasphem-language/data/blasphem-language-15-v1.bin
```

Expected: the builder prints no error and the file is 18,498,380 bytes.

- [ ] **Step 5: Record the identity**

Create `resources/models/language-artifact-v1.json`. Fill `artifact_bytes` and `artifact_sha256` from the previous step:

```json
{
  "schema_version": "language-artifact-v1",
  "artifact_relative_path": "crates/blasphem-language/data/blasphem-language-15-v1.bin",
  "artifact_bytes": 0,
  "artifact_sha256": "",
  "source_commit": "a0301db809ff2e48a418018aa5359fb0c4354eb8",
  "source_headers": [
    { "file_name": "large_db.h", "sha256": "4f9f3d9741e5f594b0a50da9bf1d26cfba2b8f049a1b75627114a6cc9c0dfe64" },
    { "file_name": "eld_unicode_bits.h", "sha256": "e620b9feb08eb32ce751a7148a51b19c5eb2774d2dff74f5dd2d1363184df23b" },
    { "file_name": "eld_tolower.h", "sha256": "97722a4d9765e609631ce527ff42b27a4e589d7e673d17e8bf1da68068da1d2b" },
    { "file_name": "eld_unicode.h", "sha256": "26b6b645823f81796dcdafdf8eedb41299d769d8c06579eab9ec4ffa3e519cf0" }
  ]
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test --workspace --locked`
Expected: PASS, including `crates/blasphem-language/tests/parity.rs`. The parity fixture compares detection results, not artifact bytes, so the magic change does not move it.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Use the BLASPHEM magic for the language artifact"
```

---

### Task 3: Browser class names and generated file names

**Files:**
- Modify: `crates/blasphem-wasm/src/lib.rs:166-243`
- Modify: `crates/blasphem-wasm/verify-browser.sh`
- Modify: `crates/blasphem-wasm/tests/browser-smoke.html`
- Modify: `crates/blasphem-wasm/tests/run-browser-smoke.mjs`
- Modify: `crates/blasphem-wasm/README.md`

**Interfaces:**
- Produces: JavaScript classes `BlasphemDetector` and `BlasphemResult`.
- Produces: generated files `blasphem.js` and `blasphem_bg.wasm` via `wasm-bindgen --out-name blasphem`.
- Consumes: the Rust structs `WasmDetector` and `WasmCheckResult`, which keep their Rust names.

- [ ] **Step 1: Rename the exported classes**

In `crates/blasphem-wasm/src/lib.rs`, add a `js_name` to both `#[wasm_bindgen]` struct attributes:

```rust
/// The browser-facing detector.
#[wasm_bindgen(js_name = BlasphemDetector)]
pub struct WasmDetector {
    core: DetectorCore,
}
```

```rust
/// The small browser result for the pre-send nudge.
#[wasm_bindgen(js_name = BlasphemResult)]
pub struct WasmCheckResult {
    inner: CoreResult,
}
```

Leave every `#[wasm_bindgen(getter)]` and `js_name` on the methods unchanged.

- [ ] **Step 2: Change the generated file name**

In `crates/blasphem-wasm/verify-browser.sh`, replace both `--out-name blasphem_wasm` occurrences with `--out-name blasphem`, and change the two input paths from `blasphem_wasm.wasm` to the same name the build produces:

```sh
FULL_INPUT="$FULL_TARGET/wasm32-unknown-unknown/release/blasphem_wasm.wasm"
```

The Cargo output keeps the library name `blasphem_wasm`. Only the wasm-bindgen output name changes. In `crates/blasphem-wasm/tests/run-browser-smoke.mjs`, change the four resolved paths:

```js
const wasmPath = resolve(fullOutput, "blasphem_bg.wasm");
const gluePath = resolve(fullOutput, "blasphem.js");
const explicitWasmPath = resolve(explicitOutput, "blasphem_bg.wasm");
const explicitGluePath = resolve(explicitOutput, "blasphem.js");
```

- [ ] **Step 3: Update the smoke page**

In `crates/blasphem-wasm/tests/browser-smoke.html`, change the module import and the class names:

```js
import init, { BlasphemDetector } from "/target/task7-wasm-full-web/blasphem.js";
```

Find every `new WasmDetector(` and change it to `new BlasphemDetector(`.

- [ ] **Step 4: Build and check the generated names**

```bash
cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/blasphem_wasm.wasm \
  --target web --out-dir target/task3-check --out-name blasphem
ls target/task3-check
grep -c 'class BlasphemDetector' target/task3-check/blasphem.js
grep -c 'class BlasphemResult' target/task3-check/blasphem.js
```

Expected: the directory holds `blasphem.js` and `blasphem_bg.wasm`. Both greps print `1`.

- [ ] **Step 5: Run the crate tests**

Run: `cargo test --locked -p blasphem-wasm`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Export BlasphemDetector and BlasphemResult from the browser build"
```

---

### Task 4: Malay storage-code split hashing

Restores the frozen Malay partition. This is the defect proven in the Verified Baseline.

**Files:**
- Modify: `crates/blasphem-train/src/datasets/prepare.rs:121-131`
- Test: `crates/blasphem-train/tests/preparation.rs`

**Interfaces:**
- Consumes: `blasphem::Language::storage_code()` at `src/language.rs:129`.
- Produces: `split_hash(Language, &str) -> u64` hashing the storage code. Task 5, Task 9, and Task 12 depend on this identity.

- [ ] **Step 1: Write the failing regression test**

Add to `crates/blasphem-train/tests/preparation.rs`:

```rust
use blasphem::Language;
use blasphem_train::datasets::prepare::split_hash;

#[test]
fn malay_split_hashing_uses_the_frozen_storage_code() {
    let text = "contoh teks untuk pembagian";
    let malay = split_hash(Language::Ms, text);
    let mut expected = 0xcbf2_9ce4_8422_2325_u64;
    for byte in b"ID\0".iter().chain(text.as_bytes()) {
        expected = (expected ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    assert_eq!(malay, expected, "Malay must hash the ID storage code");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-train malay_split_hashing_uses_the_frozen_storage_code`
Expected: FAIL. The left value hashes `MS\0`.

- [ ] **Step 3: Hash the storage code**

In `crates/blasphem-train/src/datasets/prepare.rs`, change one line in `split_hash`:

```rust
#[must_use]
pub fn split_hash(language: Language, normalized: &str) -> u64 {
    language
        .storage_code()
        .bytes()
        .chain(std::iter::once(0))
        .chain(normalized.bytes())
        .fold(FNV_OFFSET, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
        })
}
```

`clean_control_identity` in `crates/blasphem-train/src/compiler.rs:318` keeps `language.code()`. The spec accepts the regenerated Malay clean-control hash; Task 13 writes it into the manifest.

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --locked -p blasphem-train malay_split_hashing_uses_the_frozen_storage_code`
Expected: PASS

- [ ] **Step 5: Prove the frozen partition returns**

```bash
rm -rf /tmp/blasphem-malay-check
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --output /tmp/blasphem-malay-check
for split in development validation test; do
  a=$(shasum -a 256 "data/prepared-v1/ID/$split.tsv" | cut -d' ' -f1)
  b=$(shasum -a 256 "/tmp/blasphem-malay-check/ID/$split.tsv" | cut -d' ' -f1)
  [ "$a" = "$b" ] && echo "ID/$split SAME" || echo "ID/$split DIFF"
done
```

Expected: the command no longer reports `audit-only source identifier is not a development row`, and all three lines print `SAME`. Frozen row counts are 8991, 1969, and 1989 including the header line.

If any line prints `DIFF`, stop. The frozen partition is the contract; do not update `data/prepared-v1`.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Hash the storage code when assigning corpus splits"
```

---

### Task 5: Spanish raw source and the 38th lock entry

**Files:**
- Move: `data/textdetox/es-source.tsv` → `data/raw-v1/textdetox/es.tsv`
- Modify: `resources/datasets/source-lock-v1.json`
- Modify: `resources/datasets/source-catalog-v1.json`
- Test: `crates/blasphem-train/tests/source_manifest.rs`

**Interfaces:**
- Produces: lock entry `source_file_id` `textdetox-es`, `file_path` `textdetox/es.tsv`, `detector_language` `ES`.
- Produces: 38 sources in the lock. Task 8, Task 12, and Task 17 count them.

- [ ] **Step 1: Move the source**

```bash
git mv data/textdetox/es-source.tsv data/raw-v1/textdetox/es.tsv
head -1 data/raw-v1/textdetox/es.tsv
shasum -a 256 data/raw-v1/textdetox/es.tsv
```

Expected header: `source_id	language	toxic	text`. Expected digest: `8e3c8078d7406e7b695ffb943e0439240ada11d6abc9d12ac313efdb6d2f1da9`. The format matches the other nine TextDetox files, so `TextDetoxAdapter` reads it unchanged.

- [ ] **Step 2: Add the lock entry**

Insert into the `sources` array of `resources/datasets/source-lock-v1.json`, matching the shape of the existing `textdetox-en` entry:

```json
{
  "dataset": "textdetox",
  "detector_language": "ES",
  "source_file_id": "textdetox-es",
  "immutable_source_url": "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/01907546324b0330d2d8b7669648cc18823323e5/data/es-00000-of-00001.parquet",
  "archive_member": null,
  "revision": "01907546324b0330d2d8b7669648cc18823323e5",
  "file_path": "textdetox/es.tsv",
  "file_sha256": "8e3c8078d7406e7b695ffb943e0439240ada11d6abc9d12ac313efdb6d2f1da9",
  "license_id": "CC-BY-4.0",
  "license_url": "https://creativecommons.org/licenses/by/4.0/",
  "citation": "TextDetox multilingual toxicity dataset",
  "upstream_lineage": [
    "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset"
  ],
  "lineage_status": "resolved"
}
```

Leave `download_sha256` absent. The field is optional and the parquet download is not part of the data-offline path.

Add the matching `SourceRequest` entry to `resources/datasets/source-catalog-v1.json` with `requested_url` set to the same URL, `requested_revision` set to the revision, and `revision_url` set to `null`.

- [ ] **Step 3: Write the count test**

Add to `crates/blasphem-train/tests/source_manifest.rs`:

```rust
#[test]
fn the_source_lock_registers_spanish_as_the_thirty_eighth_input() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/datasets/source-lock-v1.json");
    let file = std::fs::File::open(path).expect("readable source lock");
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(file)
        .expect("valid source lock");
    assert_eq!(lock.sources.len(), 38);
    let spanish = lock
        .sources
        .iter()
        .find(|source| source.source_file_id == "textdetox-es")
        .expect("Spanish source entry");
    assert_eq!(spanish.file_path, "textdetox/es.tsv");
    assert_eq!(spanish.detector_language, blasphem::Language::Es);
}
```

- [ ] **Step 4: Run the test**

Run: `cargo test --locked -p blasphem-train the_source_lock_registers_spanish_as_the_thirty_eighth_input`
Expected: PASS

- [ ] **Step 5: Prepare and read the Spanish counts**

```bash
rm -rf /tmp/blasphem-es-check
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --output /tmp/blasphem-es-check
python3 -c "
import json
m = json.load(open('/tmp/blasphem-es-check/manifest.json'))
print('ES', m['language_counts']['ES'])
print('ID', m['language_counts']['ID'])
"
```

Expected: `ES` reports nonzero development, validation, and test counts. `ID` still reports 8990, 1968, 1988 rows excluding headers.

Record the ES counts. Task 6 and Task 9 use them.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "Register the Spanish TextDetox source in the corpus lock"
```

---

### Task 6: Spanish training path in the compiler

The compiler refuses Spanish today (`CompileError::SpanishVersionTwoInput`) and copies the frozen artifact instead. There is no legacy encoder in the workspace. This task adds one and trains Spanish from prepared data.

**Files:**
- Modify: `src/sparse.rs` (add `encode_sparse_v1`)
- Modify: `crates/blasphem-train/src/compiler.rs:42,53-56,437-440,503-505,555-565`
- Modify: `crates/blasphem-train/src/model_manifest.rs:426-462`
- Modify: `crates/blasphem-train/src/main.rs` (drop `--spanish-legacy`)
- Delete: `resources/models/es-legacy-input-v1.json`
- Test: `crates/blasphem-train/tests/compiler.rs`

**Interfaces:**
- Consumes: `blasphem::extract_feature_bins(FeatureProfile::EsLegacyWordChar35V1, NormalizationProfile::EsLegacyCharabiaV1, &str)`, already implemented at `src/features.rs:85`.
- Produces: `pub fn encode_sparse_v1(input: &SparseV1Input<'_>) -> Result<Vec<u8>, SparseModelError>` in `blasphem::sparse`.
- Produces: `compile_language` accepting `Language::Es`.
- Produces: `BatchCompileOptions` without the `spanish_legacy` field. Task 12 and Task 13 construct it.

The frozen V1 header is 32 bytes and its layout is fixed by `parse_v1` and `parse_payload`:

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 8 | magic `TOXSPRS1` |
| 8 | 2 | version `1` little endian |
| 10 | 2 | storage code, ASCII uppercase |
| 12 | 4 | bin count `65536` |
| 16 | 4 | bias, `i32` |
| 20 | 4 | decision boundary, `i32` |
| 24 | 4 | score scale, `u32` |
| 28 | 2 | false-warning limit, basis points |
| 30 | 2 | weight scale `256` |
| 32 | 131072 | 65536 weights, `i16` little endian |

- [ ] **Step 1: Write the failing encoder test**

Add to `src/sparse.rs` under its existing test module, or create the module if absent:

```rust
#[test]
fn a_version_one_artifact_round_trips_through_the_parser() {
    let weights = vec![7_i16; BIN_COUNT];
    let bytes = encode_sparse_v1(&SparseV1Input {
        bias: -13,
        decision_boundary: 10_962,
        score_scale: 27_695,
        max_false_warning_basis_points: 300,
        weights: &weights,
    })
    .expect("encodes");
    assert_eq!(bytes.len(), V1_HEADER_LENGTH + PAYLOAD_LENGTH);
    assert_eq!(&bytes[..8], b"TOXSPRS1");
    let model = SparseModel::from_bytes(&bytes).expect("parses");
    assert_eq!(model.language(), Language::Es);
    assert_eq!(model.feature_schema(), FeatureSchema::EsLegacyV1);
    assert_eq!(model.raw_boundary(), 10_962);
    assert_eq!(model.score_scale(), 27_695);
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --locked a_version_one_artifact_round_trips_through_the_parser`
Expected: FAIL with `cannot find function encode_sparse_v1`.

- [ ] **Step 3: Add the legacy encoder**

In `src/sparse.rs`, after `encode_sparse_v2`:

```rust
/// The complete input for one version-one Spanish sparse artifact.
pub struct SparseV1Input<'a> {
    pub bias: i32,
    pub decision_boundary: i32,
    pub score_scale: u32,
    pub max_false_warning_basis_points: u16,
    pub weights: &'a [i16],
}

/// Encodes one validated version-one Spanish sparse artifact.
///
/// # Errors
///
/// Returns an error when the weight table or calibration is invalid.
pub fn encode_sparse_v1(input: &SparseV1Input<'_>) -> Result<Vec<u8>, SparseModelError> {
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

    let mut output = Vec::with_capacity(V1_HEADER_LENGTH + PAYLOAD_LENGTH);
    output.extend_from_slice(V1_MAGIC);
    output.extend_from_slice(&V1_FORMAT_VERSION.to_le_bytes());
    output.extend_from_slice(Language::Es.storage_code().as_bytes());
    output.extend_from_slice(&(BIN_COUNT as u32).to_le_bytes());
    output.extend_from_slice(&input.bias.to_le_bytes());
    output.extend_from_slice(&input.decision_boundary.to_le_bytes());
    output.extend_from_slice(&input.score_scale.to_le_bytes());
    output.extend_from_slice(&input.max_false_warning_basis_points.to_le_bytes());
    output.extend_from_slice(&WEIGHT_SCALE.to_le_bytes());
    for weight in input.weights {
        output.extend_from_slice(&weight.to_le_bytes());
    }
    Ok(output)
}
```

Export it from `src/lib.rs` beside `encode_sparse_v2`.

- [ ] **Step 4: Run the encoder test to verify it passes**

Run: `cargo test --locked a_version_one_artifact_round_trips_through_the_parser`
Expected: PASS

- [ ] **Step 5: Write the failing Spanish compile test**

Add to `crates/blasphem-train/tests/compiler.rs`:

```rust
#[test]
fn spanish_compiles_deterministically_from_prepared_input() {
    let prepared = prepared_root_for_test();
    let first = compile_language_for(blasphem::Language::Es, &prepared);
    let second = compile_language_for(blasphem::Language::Es, &prepared);
    assert_eq!(first.artifact, second.artifact, "training must be deterministic");
    assert_eq!(&first.artifact[..8], b"TOXSPRS1");
    let model = blasphem::SparseModel::from_bytes(&first.artifact).expect("parses");
    assert_eq!(model.language(), blasphem::Language::Es);
    assert_eq!(
        (model.feature_profile(), model.normalization_profile(), model.feature_schema()),
        blasphem::Language::Es.profiles()
    );
}
```

Reuse the existing helpers in that file. `prepared_root_for_test` mirrors the helper at `crates/blasphem-train/tests/compiler.rs:434`, which joins `language.storage_code()`. `compile_language_for` builds a `CompileRequest` the same way the existing multi-language tests do.

- [ ] **Step 6: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-train spanish_compiles_deterministically_from_prepared_input`
Expected: FAIL with `Spanish cannot use a version-two sparse training path`.

- [ ] **Step 7: Remove the Spanish guards and branch the encoder**

In `crates/blasphem-train/src/compiler.rs`:

Delete the guard in `train_weights`:

```rust
    let language = first.detector_language;
    let (expected_profile, expected_normalization, _) = language.profiles();
```

Delete the guard at the top of `compile_language`:

```rust
pub fn compile_language(request: &CompileRequest) -> Result<CompiledLanguage, CompileError> {
    if request.rule_channel.language() != request.language {
```

Branch the encode call:

```rust
    let artifact = if request.language == Language::Es {
        encode_sparse_v1(&SparseV1Input {
            bias: trained.bias,
            decision_boundary: calibration.boundary,
            score_scale,
            max_false_warning_basis_points: FALSE_WARNING_LIMIT_BASIS_POINTS,
            weights: &trained.weights,
        })?
    } else {
        encode_sparse_v2(&SparseV2Input {
            language: request.language,
            feature_profile,
            normalization_profile,
            feature_schema,
            bias: trained.bias,
            decision_boundary: calibration.boundary,
            score_scale,
            max_false_warning_basis_points: FALSE_WARNING_LIMIT_BASIS_POINTS,
            weights: &trained.weights,
        })?
    };
```

Import `SparseV1Input` and `encode_sparse_v1` at the top of the file. Delete the `SpanishVersionTwoInput` variant from `CompileError` and every test that asserts it.

- [ ] **Step 8: Remove the legacy declaration path**

In `crates/blasphem-train/src/compiler.rs`, delete the `spanish_legacy` field from `BatchCompileOptions` and the `load_spanish_legacy` call in `compile_model_set`. Compile Spanish through the same loop as the other 14 languages.

In `crates/blasphem-train/src/model_manifest.rs`, delete `load_spanish_legacy`, `VerifiedSpanishLegacy`, `parse_spanish_legacy_input`, `validate_spanish_declaration`, `validate_spanish_metadata`, and the `ModelSetError` variants that only those functions raise.

In `crates/blasphem-train/src/main.rs`, delete the `spanish_legacy` field from `CompileArgs` and its use in `compile_models`.

```bash
rm resources/models/es-legacy-input-v1.json
```

Delete the tests in `crates/blasphem-train/tests/model_manifest.rs` that load the declaration (lines around 229, 264, 526). Keep the tests that check manifest entry shape.

- [ ] **Step 9: Compile the full model set**

```bash
rm -rf /tmp/blasphem-es-check /tmp/blasphem-models-check
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --output /tmp/blasphem-es-check
cargo run --release --locked -p blasphem-train -- compile \
  --prepared-root /tmp/blasphem-es-check \
  --hurtlex-root data/raw-v1/hurtlex \
  --behavior-root tests/fixtures/behavior \
  --output /tmp/blasphem-models-check
python3 -c "
import json
m = json.load(open('/tmp/blasphem-models-check/manifest.json'))
print('entries', len(m['entries']))
for e in m['entries']:
    if e['language'] in ('ES','MS','ID'):
        print(e['language'], e['artifact_relative_path'], e['artifact_sha256'], e['clean_control_sha256'])
"
```

Expected: 15 entries. The Spanish entry names a Spanish artifact and reports a new `artifact_sha256`, not `3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36`. Record the Spanish and Malay digests. Task 13 writes them into the committed manifest.

If Spanish fails a validation gate (`validation_gates.precision_passed` false), stop and report. The gate is the contract; do not relax it.

- [ ] **Step 10: Run the suite**

Run: `cargo test --workspace --locked`
Expected: PASS. Runtime tests still read the committed `resources/models/*`, which Task 13 replaces. If a test asserts the old Spanish digest, move that assertion to read `resources/models/multilingual-v2/manifest.json` rather than a hard-coded string.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "Train Spanish from prepared corpus data"
```

---

### Task 7: Vendored language tables and the offline language build

**Files:**
- Create: `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8/large_db.h`
- Create: `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8/eld_unicode_bits.h`
- Create: `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8/eld_tolower.h`
- Create: `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8/eld_unicode.h`
- Modify: `crates/blasphem-language/UPSTREAM.md`
- Test: `crates/blasphem-language/tests/artifact.rs`

**Interfaces:**
- Consumes: `resources/models/language-artifact-v1.json` from Task 2.
- Produces: a vendored directory the builder reads with no network access. Task 12 step 5 uses it.

- [ ] **Step 1: Copy and verify the headers**

```bash
VENDOR=crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8
mkdir -p "$VENDOR"
for f in large_db.h eld_unicode_bits.h eld_tolower.h eld_unicode.h; do
  cp "/private/tmp/eldc-main/src/eldc/$f" "$VENDOR/$f"
done
shasum -a 256 "$VENDOR"/*.h
```

Expected digests, in the order the builder pins them:

```text
4f9f3d9741e5f594b0a50da9bf1d26cfba2b8f049a1b75627114a6cc9c0dfe64  large_db.h
e620b9feb08eb32ce751a7148a51b19c5eb2774d2dff74f5dd2d1363184df23b  eld_unicode_bits.h
97722a4d9765e609631ce527ff42b27a4e589d7e673d17e8bf1da68068da1d2b  eld_tolower.h
26b6b645823f81796dcdafdf8eedb41299d769d8c06579eab9ec4ffa3e519cf0  eld_unicode.h
```

If `/private/tmp/eldc-main` is gone, clone `https://github.com/nitotm/eldc` at commit `a0301db809ff2e48a418018aa5359fb0c4354eb8` and copy from `src/eldc`. The digests above are the acceptance test.

`large_db.h` is 50,798,963 bytes. That is under GitHub's 100 MB hard limit and under its 50 MiB warning threshold.

- [ ] **Step 2: Write the failing rebuild test**

Create `crates/blasphem-language/tests/artifact.rs`:

```rust
use std::{fs, path::PathBuf, process::Command};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_committed_artifact_rebuilds_from_the_vendored_tables() {
    let root = project_root();
    let vendor = root
        .join("crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8");
    let output = tempfile::NamedTempFile::new().expect("temporary output");
    let status = Command::new(env!("CARGO_BIN_EXE_blasphem-language-model"))
        .arg(&vendor)
        .arg(output.path())
        .status()
        .expect("runs the language model builder");
    assert!(status.success(), "the builder must succeed");

    let rebuilt = fs::read(output.path()).expect("readable rebuild");
    let committed = fs::read(
        root.join("crates/blasphem-language/data/blasphem-language-15-v1.bin"),
    )
    .expect("readable committed artifact");
    assert_eq!(rebuilt, committed, "the rebuild must match the committed artifact");
}
```

Add `tempfile = "3.20"` to `[dev-dependencies]` in `crates/blasphem-language/Cargo.toml`.

- [ ] **Step 3: Run it to verify it passes**

Run: `cargo test --release --locked -p blasphem-language the_committed_artifact_rebuilds_from_the_vendored_tables`
Expected: PASS. Use `--release`; the debug build parses the 50 MB table slowly.

If the bytes differ, the committed artifact predates the `BLASPHEM` magic. Rerun Task 2 step 4 against the vendored directory and commit the result.

- [ ] **Step 4: Record the vendor path**

In `crates/blasphem-language/UPSTREAM.md`, add after the digest table:

```markdown
The repository vendors these four files under
`crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8`.
The build reads them from that directory. It downloads nothing.
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Vendor the pinned upstream language tables"
```

---

### Task 8: Source roles

**Files:**
- Create: `crates/blasphem-train/src/source_role.rs`
- Modify: `crates/blasphem-train/src/lib.rs`
- Modify: `crates/blasphem-train/src/source_manifest.rs:35-71`
- Modify: `resources/datasets/source-lock-v1.json` (38 entries)
- Modify: `resources/datasets/source-catalog-v1.json` (38 entries)
- Test: `crates/blasphem-train/tests/source_manifest.rs`

**Interfaces:**
- Produces: `pub enum SourceRole { Baseline, TrainingOnly, SealedEvaluation }` serialized as `baseline`, `training_only`, `sealed_evaluation`.
- Produces: a required `source_role` field on `SourceRequest`, `FrozenSource`, and `SourceRecord`. Task 10 reads it.

- [ ] **Step 1: Write the failing role test**

Add to `crates/blasphem-train/tests/source_manifest.rs`:

```rust
use blasphem_train::source_role::SourceRole;

#[test]
fn every_current_source_declares_the_baseline_role() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../resources/datasets/source-lock-v1.json");
    let file = std::fs::File::open(path).expect("readable source lock");
    let lock = blasphem_train::source_manifest::parse_frozen_source_lock(file)
        .expect("valid source lock");
    assert_eq!(lock.sources.len(), 38);
    for source in &lock.sources {
        assert_eq!(
            source.source_role,
            SourceRole::Baseline,
            "{} must keep its frozen partition",
            source.source_file_id
        );
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-train every_current_source_declares_the_baseline_role`
Expected: FAIL with `unresolved module or unlinked crate 'source_role'`.

- [ ] **Step 3: Add the role type**

Create `crates/blasphem-train/src/source_role.rs`:

```rust
use serde::{Deserialize, Serialize};

/// How the preparation pipeline may use one corpus source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    /// A frozen source whose partition the pipeline must preserve.
    Baseline,
    /// A community source whose rows enter only the development partition.
    TrainingOnly,
    /// A source reserved for sealed validation and test rows.
    SealedEvaluation,
}

impl SourceRole {
    /// Returns true when the role forbids new validation or test rows.
    #[must_use]
    pub const fn is_development_only(self) -> bool {
        matches!(self, Self::TrainingOnly)
    }
}
```

Add `pub mod source_role;` to `crates/blasphem-train/src/lib.rs`.

- [ ] **Step 4: Add the field to the three records**

In `crates/blasphem-train/src/source_manifest.rs`, add to `SourceRequest`, `FrozenSource`, and `SourceRecord`, placed after `detector_language`:

```rust
    pub source_role: SourceRole,
```

Import it with `use crate::source_role::SourceRole;`. The structs carry `#[serde(deny_unknown_fields)]`, so the field is required and every JSON record must declare it.

Update every construction site the compiler reports. Find them with:

```bash
cargo check -p blasphem-train --all-targets --locked 2>&1 | grep -n 'missing field' | head -30
```

`crates/blasphem-train/src/acquisition.rs` builds `SourceRecord` from `SourceRequest`; copy the role through. `crates/blasphem-train/src/main.rs` builds `FrozenSource` from `SourceRecord`; copy the role through.

- [ ] **Step 5: Write the role into the two JSON files**

```bash
python3 - <<'PY'
import json
for path in (
    "resources/datasets/source-lock-v1.json",
    "resources/datasets/source-catalog-v1.json",
):
    with open(path) as handle:
        document = json.load(handle)
    for source in document["sources"]:
        ordered = {}
        for key, value in source.items():
            ordered[key] = value
            if key == "detector_language":
                ordered["source_role"] = "baseline"
        source.clear()
        source.update(ordered)
    with open(path, "w") as handle:
        json.dump(document, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
    print(path, len(document["sources"]))
PY
```

Expected: both lines print `38`.

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test --locked -p blasphem-train every_current_source_declares_the_baseline_role`
Expected: PASS

Run: `cargo test --workspace --locked`
Expected: PASS. Test fixtures in `crates/blasphem-train/tests/cli.rs:591,597` embed JSON source records; add `"source_role":"baseline"` to both.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Declare a corpus role on every source record"
```

---

### Task 9: Sealed evaluation lock

**Files:**
- Create: `crates/blasphem-train/src/evaluation_lock.rs`
- Create: `resources/datasets/evaluation-lock-v1.json`
- Modify: `crates/blasphem-train/src/lib.rs`
- Modify: `crates/blasphem-train/src/main.rs` (`PrepareArgs`, `prepare`)
- Test: `crates/blasphem-train/tests/evaluation_lock.rs`

**Interfaces:**
- Produces: `pub fn sealed_partition_digest(path: &Path) -> io::Result<Sha256Digest>` hashing raw file bytes.
- Produces: `pub fn verify_sealed_partitions(prepared_root: &Path, lock: &EvaluationLock) -> Result<(), EvaluationLockError>`.
- Produces: `resources/datasets/evaluation-lock-v1.json` with `schema_version` `evaluation-lock-v1` and a `languages` map keyed by storage code, each holding `validation_sha256` and `test_sha256`.
- Task 12 step 3 and Task 13 call these.

- [ ] **Step 1: Write the failing lock test**

Create `crates/blasphem-train/tests/evaluation_lock.rs`:

```rust
use std::{fs, path::PathBuf};

use blasphem_train::evaluation_lock::{
    EvaluationLockError, parse_evaluation_lock, verify_sealed_partitions,
};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn committed_lock() -> blasphem_train::evaluation_lock::EvaluationLock {
    let path = project_root().join("resources/datasets/evaluation-lock-v1.json");
    let file = fs::File::open(path).expect("readable evaluation lock");
    parse_evaluation_lock(file).expect("valid evaluation lock")
}

#[test]
fn the_lock_seals_fifteen_languages() {
    let lock = committed_lock();
    assert_eq!(lock.languages.len(), 15);
    assert!(lock.languages.contains_key("ID"), "Malay seals under its storage code");
    assert!(lock.languages.contains_key("ES"));
}

#[test]
fn a_moved_test_row_fails_verification() {
    let source = project_root().join("data/prepared-v1");
    if !source.exists() {
        eprintln!("skipped: data/prepared-v1 is derived and not committed");
        return;
    }
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().join("prepared");
    copy_tree(&source, &root);

    let target = root.join("EN/test.tsv");
    let mut text = fs::read_to_string(&target).expect("readable test split");
    text.push_str("EN\tclean\tinjected@0/row/000000\tinjected row\n");
    fs::write(&target, text).expect("writable test split");

    let error = verify_sealed_partitions(&root, &committed_lock())
        .expect_err("a changed sealed file must fail");
    assert!(matches!(error, EvaluationLockError::SealedHashChanged { .. }));
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to).expect("creatable directory");
    for entry in fs::read_dir(from).expect("readable directory") {
        let entry = entry.expect("readable entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copyable file");
        }
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-train --test evaluation_lock`
Expected: FAIL with `unresolved module or unlinked crate 'evaluation_lock'`.

- [ ] **Step 3: Add the lock module**

Create `crates/blasphem-train/src/evaluation_lock.rs`:

```rust
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::evidence::Sha256Digest;

pub const EVALUATION_LOCK_SCHEMA_VERSION: &str = "evaluation-lock-v1";

/// The sealed validation and test digests for one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedLanguage {
    pub validation_sha256: Sha256Digest,
    pub test_sha256: Sha256Digest,
}

/// The sealed evaluation partitions, keyed by storage code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationLock {
    pub schema_version: String,
    pub languages: BTreeMap<String, SealedLanguage>,
}

#[derive(Debug, Error)]
pub enum EvaluationLockError {
    #[error("cannot parse the evaluation lock: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid evaluation lock schema version: expected {expected}, got {actual}")]
    InvalidSchemaVersion {
        expected: &'static str,
        actual: String,
    },
    #[error("cannot read the sealed partition {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the sealed partition {relative_path} changed: expected {expected}, got {actual}")]
    SealedHashChanged {
        relative_path: String,
        expected: String,
        actual: String,
    },
}

/// Parses one evaluation lock document.
///
/// # Errors
///
/// Returns an error when the JSON or the schema version is invalid.
pub fn parse_evaluation_lock(reader: impl Read) -> Result<EvaluationLock, EvaluationLockError> {
    let lock: EvaluationLock = serde_json::from_reader(reader)?;
    if lock.schema_version != EVALUATION_LOCK_SCHEMA_VERSION {
        return Err(EvaluationLockError::InvalidSchemaVersion {
            expected: EVALUATION_LOCK_SCHEMA_VERSION,
            actual: lock.schema_version,
        });
    }
    Ok(lock)
}

/// Returns the SHA-256 digest of one prepared split file.
///
/// # Errors
///
/// Returns an error when the file is unreadable.
pub fn sealed_partition_digest(path: &Path) -> Result<Sha256Digest, EvaluationLockError> {
    let bytes = fs::read(path).map_err(|source| EvaluationLockError::Io {
        path: path.to_owned(),
        source,
    })?;
    let digest = Sha256::digest(&bytes);
    Ok(Sha256Digest::from_bytes(digest.as_slice())
        .expect("SHA-256 output is a valid digest"))
}

/// Computes the sealed digests for every language directory under one prepared root.
///
/// # Errors
///
/// Returns an error when a sealed file is unreadable.
pub fn compute_sealed_partitions(
    prepared_root: &Path,
    storage_codes: &[&str],
) -> Result<BTreeMap<String, SealedLanguage>, EvaluationLockError> {
    let mut languages = BTreeMap::new();
    for code in storage_codes {
        languages.insert(
            (*code).to_owned(),
            SealedLanguage {
                validation_sha256: sealed_partition_digest(
                    &prepared_root.join(code).join("validation.tsv"),
                )?,
                test_sha256: sealed_partition_digest(&prepared_root.join(code).join("test.tsv"))?,
            },
        );
    }
    Ok(languages)
}

/// Rejects any change to a sealed validation or test partition.
///
/// # Errors
///
/// Returns an error on the first missing or changed sealed file.
pub fn verify_sealed_partitions(
    prepared_root: &Path,
    lock: &EvaluationLock,
) -> Result<(), EvaluationLockError> {
    for (code, sealed) in &lock.languages {
        for (name, expected) in [
            ("validation.tsv", &sealed.validation_sha256),
            ("test.tsv", &sealed.test_sha256),
        ] {
            let path = prepared_root.join(code).join(name);
            let actual = sealed_partition_digest(&path)?;
            if &actual != expected {
                return Err(EvaluationLockError::SealedHashChanged {
                    relative_path: format!("{code}/{name}"),
                    expected: expected.to_string(),
                    actual: actual.to_string(),
                });
            }
        }
    }
    Ok(())
}
```

Add `pub mod evaluation_lock;` to `crates/blasphem-train/src/lib.rs`. If `Sha256Digest` lacks `from_bytes` or `Display`, use the constructor and formatter already present in `crates/blasphem-train/src/evidence.rs`; read that file first and match its API.

- [ ] **Step 4: Seed the lock from the accepted partitions**

```bash
rm -rf /tmp/blasphem-seed
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --output /tmp/blasphem-seed

python3 - <<'PY'
import hashlib, json, os
root = "/tmp/blasphem-seed"
codes = sorted(d for d in os.listdir(root) if os.path.isdir(os.path.join(root, d)))
def digest(path):
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()
languages = {
    code: {
        "validation_sha256": digest(os.path.join(root, code, "validation.tsv")),
        "test_sha256": digest(os.path.join(root, code, "test.tsv")),
    }
    for code in codes
}
document = {"schema_version": "evaluation-lock-v1", "languages": languages}
with open("resources/datasets/evaluation-lock-v1.json", "w") as handle:
    json.dump(document, handle, indent=2)
    handle.write("\n")
print("sealed languages:", len(languages), sorted(languages))
PY
```

Expected: `sealed languages: 15` and the list contains `ID` and `ES`. `ID` is Malay under its storage code.

- [ ] **Step 5: Enforce the lock during preparation**

In `crates/blasphem-train/src/main.rs`, add to `PrepareArgs`:

```rust
    #[arg(long)]
    evaluation_lock: Option<PathBuf>,
```

After the prepared output is published, verify it when the flag is present:

```rust
    if let Some(lock_path) = arguments.evaluation_lock.as_ref() {
        let file = File::open(lock_path)
            .with_context(|| format!("cannot open {}", lock_path.display()))?;
        let lock = parse_evaluation_lock(file).context("cannot parse the evaluation lock")?;
        verify_sealed_partitions(&arguments.output, &lock)
            .context("the prepared output changes a sealed evaluation partition")?;
    }
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --locked -p blasphem-train --test evaluation_lock`
Expected: PASS

Run the enforcement end to end:

```bash
rm -rf /tmp/blasphem-sealed-ok
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --evaluation-lock resources/datasets/evaluation-lock-v1.json \
  --output /tmp/blasphem-sealed-ok
echo "exit=$?"
```

Expected: `exit=0`.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Seal the validation and test partitions behind a lock file"
```

---

### Task 10: Training-only corpus contributions

**Files:**
- Create: `crates/blasphem-train/src/community_corpus.rs`
- Modify: `crates/blasphem-train/src/lib.rs`
- Modify: `crates/blasphem-train/src/datasets/prepare.rs` (`PreparationPolicy`, `split_for_source`)
- Modify: `crates/blasphem-train/src/datasets/types.rs` (`DatasetId`)
- Create: `crates/blasphem-train/tests/fixtures/community/valid.tsv`
- Create: `crates/blasphem-train/tests/fixtures/community/conflicting.tsv`
- Test: `crates/blasphem-train/tests/community_corpus.rs`

**Interfaces:**
- Consumes: `SourceRole` from Task 8.
- Produces: `pub struct CommunityCorpusAdapter` implementing `DatasetAdapter`, reading `native_id\tlabel\ttext`.
- Produces: `PreparationPolicy::source_roles: BTreeMap<String, SourceRole>` keyed by `source_file_id`.
- Produces: `ExclusionReason::SealedBaselineDuplicate`.

- [ ] **Step 1: Write the fixtures**

Create `crates/blasphem-train/tests/fixtures/community/valid.tsv`:

```text
native_id	label	text
row-000001	toxic	eres un completo idiota
row-000002	clean	gracias por tu ayuda hoy
row-000003	clean	nos vemos en la reunion
```

Create `crates/blasphem-train/tests/fixtures/community/conflicting.tsv`:

```text
native_id	label	text
row-000001	clean	eres un completo idiota
```

- [ ] **Step 2: Write the failing adapter test**

Create `crates/blasphem-train/tests/community_corpus.rs`:

```rust
use std::{fs::File, path::PathBuf};

use blasphem::{EvalLabel, Language};
use blasphem_train::community_corpus::CommunityCorpusAdapter;
use blasphem_train::datasets::{DatasetAdapter, RowDisposition, SourceInput, SourceSplit};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/community")
        .join(name)
}

#[test]
fn the_canonical_schema_imports_three_rows() {
    let mut reader = File::open(fixture("valid.tsv")).expect("readable fixture");
    let adapter = CommunityCorpusAdapter::new(Language::Es, "community-es-demo");
    let mut inputs = vec![SourceInput {
        source_file_id: "community-es-demo",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];
    let rows = adapter.import(&mut inputs).expect("imports the fixture");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].source_id, "community-es-demo/row-000001");
    assert_eq!(rows[0].disposition, RowDisposition::Candidate(EvalLabel::Toxic));
    assert_eq!(rows[1].disposition, RowDisposition::Candidate(EvalLabel::Clean));
}

#[test]
fn an_invalid_label_names_the_source_and_the_row() {
    let mut reader = std::io::Cursor::new("native_id\tlabel\ttext\nrow-1\tmaybe\thola\n");
    let adapter = CommunityCorpusAdapter::new(Language::Es, "community-es-demo");
    let mut inputs = vec![SourceInput {
        source_file_id: "community-es-demo",
        source_split: SourceSplit::Unsplit,
        reader: &mut reader,
    }];
    let error = adapter.import(&mut inputs).expect_err("rejects the label");
    let message = error.to_string();
    assert!(message.contains("community-es-demo"), "{message}");
    assert!(message.contains("row-1"), "{message}");
}
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-train --test community_corpus`
Expected: FAIL with `unresolved module or unlinked crate 'community_corpus'`.

- [ ] **Step 4: Write the adapter**

Create `crates/blasphem-train/src/community_corpus.rs`. Read `crates/blasphem-train/src/datasets/textdetox.rs` first and copy its `DatasetAdapter` shape, error type, and CSV reader configuration. The adapter:

- Declares `DatasetId::Community`.
- Requires the exact header `native_id`, `label`, `text`.
- Accepts only `toxic` and `clean` in `label`.
- Builds `source_id` as `{source_file_id}/{native_id}`.
- Marks an empty `text` as `RowDisposition::Excluded(ExclusionReason::EmptyText)`.
- Names the `source_file_id` and the `native_id` in every error.

Add `Community` to `DatasetId` in `crates/blasphem-train/src/datasets/types.rs` with `#[serde(rename = "community")]` and the matching `Display` arm.

Add `pub mod community_corpus;` to `crates/blasphem-train/src/lib.rs`.

- [ ] **Step 5: Route training-only rows to development**

In `crates/blasphem-train/src/datasets/prepare.rs`, add to `PreparationPolicy`:

```rust
    pub source_roles: BTreeMap<String, SourceRole>,
```

In `split_for_source`, force development for a training-only source before the hash runs:

```rust
fn split_for_source(
    row: &ImportedRow,
    policy: &PreparationPolicy,
    normalized: &str,
) -> Result<DatasetSplit, PreparationError> {
    if policy
        .source_roles
        .get(&row.source_file_id)
        .is_some_and(|role| role.is_development_only())
    {
        return Ok(DatasetSplit::Development);
    }
    // the existing body follows unchanged
```

Add `SealedBaselineDuplicate` to `ExclusionReason` with `#[serde(rename = "sealed_baseline_duplicate")]`.

In the duplicate-group resolution inside `prepare_language`, when a group holds rows from more than one source and at least one row belongs to a `Baseline` source, keep the baseline row as the representative and mark the training-only rows `Excluded(ExclusionReason::SealedBaselineDuplicate)`. When a training-only row carries a label that differs from the baseline representative, return the existing conflict error instead of excluding. Read the current grouping code before editing; the representative choice already exists and only its ordering rule changes.

Update every `PreparationPolicy` construction site the compiler reports.

- [ ] **Step 6: Write the failing ingestion test**

Add to `crates/blasphem-train/tests/preparation.rs`:

```rust
#[test]
fn a_training_only_source_never_enters_validation_or_test() {
    let mut roles = std::collections::BTreeMap::new();
    roles.insert("community-es-demo".to_owned(), SourceRole::TrainingOnly);
    let policy = PreparationPolicy {
        language: Language::Es,
        split_policy: SplitPolicy::Hash70_15_15,
        split_version: "fnv1a-uppercase-v1",
        normalization_version: "runtime-normalize-v2",
        audit_only_source_ids: Default::default(),
        source_roles: roles,
    };
    let rows = community_rows(40);
    let prepared = prepare_language(rows, &policy).expect("prepares the community rows");
    assert_eq!(prepared.validation.len(), 0);
    assert_eq!(prepared.test.len(), 0);
    assert_eq!(prepared.development.len(), 40);
}
```

Write `community_rows(count)` as a local helper that returns `count` `ImportedRow` values with distinct texts and `source_file_id` `community-es-demo`, following the row builders already in that file.

- [ ] **Step 7: Run all three tests to verify they pass**

Run: `cargo test --locked -p blasphem-train --test community_corpus --test preparation`
Expected: PASS

Run: `cargo test --workspace --locked`
Expected: PASS

- [ ] **Step 8: Prove the baseline partitions did not move**

```bash
rm -rf /tmp/blasphem-role-check
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --audit-exclusions resources/datasets/rule-audit-v1.tsv \
  --evaluation-lock resources/datasets/evaluation-lock-v1.json \
  --output /tmp/blasphem-role-check
echo "exit=$?"
```

Expected: `exit=0`.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "Accept training-only community corpus contributions"
```

---

### Task 11: Toolchain pins

**Files:**
- Create: `rust-toolchain.toml`
- Create: `.nvmrc`
- Create: `packages/blasphem/TOOLCHAIN.md`

**Interfaces:**
- Produces: pinned versions Task 12, Task 14, Task 15, and Task 17 assert.

- [ ] **Step 1: Pin Rust**

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.97.0"
components = ["clippy", "rustfmt"]
targets = ["wasm32-unknown-unknown", "x86_64-unknown-linux-gnu"]
profile = "minimal"
```

- [ ] **Step 2: Pin Node**

Create `.nvmrc`:

```text
24.18.0
```

- [ ] **Step 3: Verify the pins resolve**

```bash
rustc --version
cargo --version
node --version
npm --version
wasm-bindgen --version
```

Expected: `rustc 1.97.0`, `cargo 1.97.0`, `v24.18.0`, `11.16.0`, `wasm-bindgen 0.2.127`.

The `wasm-bindgen` CLI version must equal the crate version pinned at `crates/blasphem-wasm/Cargo.toml` (`wasm-bindgen = "=0.2.127"`). A mismatch makes the generated glue reject the module at load time.

- [ ] **Step 4: Record the pins**

Create `packages/blasphem/TOOLCHAIN.md`:

```markdown
# Pinned tools

The browser package builds only with these versions.

| Tool | Version | Source |
| --- | --- | --- |
| Rust | 1.97.0 | `rust-toolchain.toml` |
| Node | 24.18.0 | `.nvmrc` |
| npm | 11.16.0 | `packages/blasphem/package.json` `engines` |
| `wasm-bindgen-cli` | 0.2.127 | `cargo install wasm-bindgen-cli --version 0.2.127 --locked` |
| Playwright | recorded by `packages/blasphem/package.json` | `npm install --save-exact --save-dev @playwright/test` |
| Chromium | recorded by `packages/blasphem/chromium-revision.txt` | `npx playwright install chromium` |

Task 15 writes the Playwright and Chromium rows.
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Pin the Rust and Node toolchains"
```

---

### Task 12: The read-only reproduce command

**Files:**
- Create: `crates/blasphem-train/src/reproduce.rs`
- Modify: `crates/blasphem-train/src/lib.rs`
- Modify: `crates/blasphem-train/src/main.rs` (`Command::Reproduce`)
- Test: `crates/blasphem-train/tests/reproduce.rs`

**Interfaces:**
- Consumes: `verify_sealed_partitions` (Task 9), `compile_model_set` without `spanish_legacy` (Task 6), `resources/models/language-artifact-v1.json` (Task 2), the vendored headers (Task 7).
- Produces: `pub fn reproduce(options: &ReproduceOptions) -> Result<ReproduceReport, ReproduceError>`.
- Produces: `ReproduceOptions { project_root: PathBuf, work_root: PathBuf, skip_browser: bool }`.
- Produces: `ReproduceReport { steps: Vec<StepOutcome> }` where `StepOutcome { name: String, passed: bool, detail: String }`.

The command performs the nine spec steps in order and stops at the first failure.

- [ ] **Step 1: Write the failing command test**

Create `crates/blasphem-train/tests/reproduce.rs`:

```rust
use std::process::Command;

#[test]
fn reproduce_rejects_one_changed_raw_byte() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().join("clone");
    copy_project(&root);

    let target = root.join("data/raw-v1/textdetox/en.tsv");
    let mut bytes = std::fs::read(&target).expect("readable raw source");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x20;
    std::fs::write(&target, bytes).expect("writable raw source");

    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .current_dir(&root)
        .args(["reproduce", "--skip-browser"])
        .output()
        .expect("runs reproduce");
    assert!(!output.status.success(), "a changed raw byte must fail");
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("textdetox/en.tsv"), "{message}");
}
```

Write `copy_project(&root)` as a local helper that copies `Cargo.toml`, `Cargo.lock`, `src`, `crates`, `data/raw-v1`, `data/hurtlex`, `resources`, `tests`, and `samples` from `env!("CARGO_MANIFEST_DIR")/../..`. Skip `target` and `.git`.

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test --locked -p blasphem-train --test reproduce`
Expected: FAIL with `unrecognized subcommand 'reproduce'`.

- [ ] **Step 3: Write the reproduce module**

Create `crates/blasphem-train/src/reproduce.rs`. Structure it as nine named functions that a driver calls in order:

```rust
/// The nine ordered reproduction steps.
pub const STEP_NAMES: [&str; 9] = [
    "verify-raw-inputs",
    "generate-prepared-data",
    "verify-sealed-partitions",
    "compile-model-artifacts",
    "rebuild-language-artifact",
    "compare-model-manifest",
    "build-native-binary",
    "build-wasm-modules",
    "run-checks",
];
```

Step behavior:

1. `verify-raw-inputs` — read `resources/datasets/source-lock-v1.json`, hash each `data/raw-v1/{file_path}`, compare with `file_sha256`. Report the `file_path` on mismatch.
2. `generate-prepared-data` — run the preparation pipeline into `work_root/prepared`. Never touch `data/prepared-v1`.
3. `verify-sealed-partitions` — call `verify_sealed_partitions(work_root/prepared, lock)`.
4. `compile-model-artifacts` — call `compile_model_set` into `work_root/models` with `hurtlex_root` `data/raw-v1/hurtlex` and `behavior_root` `tests/fixtures/behavior`.
5. `rebuild-language-artifact` — run the `blasphem-language-model` binary over the vendored directory into `work_root/language.bin`, then compare its bytes and digest with `resources/models/language-artifact-v1.json`.
6. `compare-model-manifest` — for each of the 15 entries in `resources/models/multilingual-v2/manifest.json`, compare `artifact_sha256` with the digest of the matching file in `work_root/models`.
7. `build-native-binary` — `cargo build --release --locked --bin blasphem`.
8. `build-wasm-modules` — build `blasphem-wasm` for `wasm32-unknown-unknown` twice, default features and `--no-default-features`, then run `wasm-bindgen --target web --out-name blasphem` for each.
9. `run-checks` — `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check`, then the npm checks and the browser smoke unless `skip_browser` is set.

Every step returns `Err` with a message naming the file or artifact that failed. The driver stops at the first `Err`.

Add `pub mod reproduce;` to `crates/blasphem-train/src/lib.rs`.

- [ ] **Step 4: Wire the subcommand**

In `crates/blasphem-train/src/main.rs`:

```rust
#[derive(Debug, Args)]
struct ReproduceArgs {
    /// The directory that holds generated data. Defaults to a temporary directory.
    #[arg(long)]
    work_root: Option<PathBuf>,
    /// Skips the npm and browser checks.
    #[arg(long)]
    skip_browser: bool,
}
```

Add `Reproduce(ReproduceArgs)` to `enum Command` and dispatch it. On success print one line:

```rust
    println!("status=reproduced steps={}", report.steps.len());
```

On failure return the error. `anyhow` prints it to stderr and the process exits nonzero.

- [ ] **Step 5: Run the command against the real repository**

```bash
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
echo "exit=$?"
```

Expected: `exit=0` and `status=reproduced steps=9`.

If step 6 reports a Spanish or Malay mismatch, that is expected before Task 13. Run Task 13 first, then rerun this step.

- [ ] **Step 6: Run the test to verify it passes**

Run: `cargo test --release --locked -p blasphem-train --test reproduce`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add the data-offline reproduce command"
```

---

### Task 13: The regenerate command and the refreshed artifacts

**Files:**
- Create: `crates/blasphem-train/src/regenerate.rs`
- Modify: `crates/blasphem-train/src/lib.rs`
- Modify: `crates/blasphem-train/src/main.rs` (`Command::Regenerate`)
- Modify: `resources/models/multilingual-v2/*.bin`
- Modify: `resources/models/multilingual-v2/manifest.json`
- Modify: `resources/models/es-chargram-v1.bin`, `resources/models/es-chargram-v1.json`
- Modify: `reports/*.json`, `docs/*.md`

**Interfaces:**
- Consumes: the reproduce steps from Task 12.
- Produces: `pub fn regenerate(options: &RegenerateOptions) -> Result<RegenerateReport, RegenerateError>` writing reviewed artifacts in place.

- [ ] **Step 1: Write the regenerate module**

Create `crates/blasphem-train/src/regenerate.rs`. It runs steps 1 to 5 of `reproduce`, then writes rather than compares:

- Copies each compiled artifact from the work root into `resources/models/multilingual-v2/`.
- Writes `resources/models/multilingual-v2/manifest.json` with `write_canonical_json`.
- Copies the Spanish artifact to `resources/models/es-chargram-v1.bin` and rewrites `resources/models/es-chargram-v1.json` from the manifest entry.
- Copies the rebuilt language artifact to `crates/blasphem-language/data/blasphem-language-15-v1.bin` and rewrites `resources/models/language-artifact-v1.json`.
- Rewrites the evidence reports under `reports/` by calling the existing `evaluate`, `behavior`, and `cli-smoke` paths.

Reuse `crate::atomic_publish` for every write so a failed run leaves no partial file.

Add `pub mod regenerate;` to `crates/blasphem-train/src/lib.rs` and `Regenerate(RegenerateArgs)` to `enum Command`.

- [ ] **Step 2: Run it**

```bash
cargo run --release --locked -p blasphem-train -- regenerate
echo "exit=$?"
git status --short resources/ reports/ crates/blasphem-language/data/
```

Expected: `exit=0`. The Spanish artifact, the Malay manifest entry, and the reports show as modified.

- [ ] **Step 3: Read the diff before accepting it**

```bash
python3 -c "
import json
m = json.load(open('resources/models/multilingual-v2/manifest.json'))
for e in m['entries']:
    print(e['language'], e['artifact_sha256'][:16], e['clean_control_sha256'][:16] if e['clean_control_sha256'] else None, e['validation_gates'])
"
```

Expected: 15 rows. Every row reports `false_warning_passed`, `precision_passed`, and `has_true_positive` true. If any gate is false, stop and report which language failed. Do not commit a failing gate.

- [ ] **Step 4: Confirm the runtime picks up the new bytes**

`src/sparse.rs:16` and `src/registry.rs:206-346` embed the artifacts with `include_bytes!`. Rebuild and run the CLI:

```bash
cargo build --release --locked --bin blasphem
./target/release/blasphem check --language ES --text "Te voy a matar"
./target/release/blasphem check --language ES --text "No te voy a matar"
./target/release/blasphem check --language MS --text "Kalau bertemu lagi, akan kupatahkan rahangmu"
```

Expected: the two threatening lines report a nudge. The negated line does not.

- [ ] **Step 5: Run the full reproduction**

```bash
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
echo "exit=$?"
```

Expected: `exit=0`. Step 6 now matches because the manifest holds the regenerated digests.

- [ ] **Step 6: Run the suite**

Run: `cargo test --workspace --locked`
Expected: PASS

Run: `cargo clippy --workspace --all-targets --locked -- -D warnings`
Expected: exit 0

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Regenerate the model artifacts and evidence reports"
```

---

### Task 14: The private npm package

**Files:**
- Create: `packages/blasphem/package.json`
- Create: `packages/blasphem/scripts/build.mjs`
- Create: `packages/blasphem/scripts/pack-check.mjs`
- Create: `packages/blasphem/index.d.ts`
- Create: `packages/blasphem/README.md`
- Create: `packages/blasphem/NOTICE`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: `blasphem-wasm` and the `--out-name blasphem` build from Task 3.
- Produces: `dist/blasphem.js`, `dist/blasphem_bg.wasm`, `dist/index.d.ts`.
- Produces: the npm scripts `build`, `pack:check`, and `test:browser` (Task 15).

- [ ] **Step 1: Write the manifest**

Create `packages/blasphem/package.json`:

```json
{
  "name": "blasphem",
  "version": "0.1.0",
  "private": true,
  "description": "Experimental multilingual pre-send toxicity nudge for browsers",
  "license": "Apache-2.0",
  "type": "module",
  "sideEffects": false,
  "engines": {
    "node": "24.18.0",
    "npm": "11.16.0"
  },
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "browser": "./dist/blasphem.js",
      "default": "./dist/blasphem.js"
    },
    "./blasphem_bg.wasm": "./dist/blasphem_bg.wasm"
  },
  "types": "./dist/index.d.ts",
  "files": [
    "dist/blasphem.js",
    "dist/blasphem_bg.wasm",
    "dist/index.d.ts",
    "README.md",
    "NOTICE",
    "LICENSE"
  ],
  "scripts": {
    "build": "node scripts/build.mjs",
    "pack:check": "node scripts/pack-check.mjs",
    "test:browser": "node scripts/browser-smoke.mjs"
  }
}
```

- [ ] **Step 2: Write the build script**

Create `packages/blasphem/scripts/build.mjs`:

```js
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const projectRoot = resolve(packageRoot, "../..");
const distribution = resolve(packageRoot, "dist");
const buildTarget = resolve(projectRoot, "target/npm-wasm");

const REQUIRED_WASM_BINDGEN = "0.2.127";

function run(command, args, options = {}) {
  return execFileSync(command, args, { encoding: "utf8", ...options });
}

const wasmBindgenVersion = run("wasm-bindgen", ["--version"]).trim();
if (wasmBindgenVersion !== `wasm-bindgen ${REQUIRED_WASM_BINDGEN}`) {
  throw new Error(
    `wasm-bindgen must be ${REQUIRED_WASM_BINDGEN}, found "${wasmBindgenVersion}"`,
  );
}

rmSync(distribution, { recursive: true, force: true });
mkdirSync(distribution, { recursive: true });

run(
  "cargo",
  [
    "build",
    "--release",
    "--locked",
    "--target",
    "wasm32-unknown-unknown",
    "-p",
    "blasphem-wasm",
    "--manifest-path",
    resolve(projectRoot, "Cargo.toml"),
  ],
  { env: { ...process.env, CARGO_TARGET_DIR: buildTarget }, stdio: "inherit" },
);

run(
  "wasm-bindgen",
  [
    resolve(buildTarget, "wasm32-unknown-unknown/release/blasphem_wasm.wasm"),
    "--target",
    "web",
    "--out-dir",
    distribution,
    "--out-name",
    "blasphem",
  ],
  { stdio: "inherit" },
);

copyFileSync(resolve(packageRoot, "index.d.ts"), resolve(distribution, "index.d.ts"));
console.log("status=built");
```

- [ ] **Step 3: Write the declarations**

Create `packages/blasphem/index.d.ts`:

```ts
/** Loads the WebAssembly module. Call this once before creating a detector. */
export default function init(module?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module): Promise<unknown>;

/** The small browser result for the pre-send nudge. */
export class BlasphemResult {
  readonly ok: boolean;
  readonly score: number;
  readonly threshold: number;
  readonly shouldNudge: boolean;
  readonly evaluated: boolean;
  readonly resolvedLanguage: string;
  readonly languageReliable: boolean;
  readonly languageScore: number | undefined;
  free(): void;
}

/** The browser-facing detector. */
export class BlasphemDetector {
  /** Builds one detector for an explicit language code or `AUTO`. */
  constructor(language: string);
  readonly language: string;
  check(text: string): BlasphemResult;
  free(): void;
}
```

Confirm the property list against the generated glue after Step 5. Every getter in `crates/blasphem-wasm/src/lib.rs:204-243` must appear.

- [ ] **Step 4: Write the pack check**

Create `packages/blasphem/scripts/pack-check.mjs`:

```js
import { execFileSync } from "node:child_process";
import { readFileSync, rmSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));

if (manifest.name !== "blasphem") {
  throw new Error(`the package name must be "blasphem", found "${manifest.name}"`);
}
if (manifest.private !== true) {
  throw new Error("the package must stay private");
}

const report = JSON.parse(
  execFileSync("npm", ["pack", "--dry-run", "--json"], {
    cwd: packageRoot,
    encoding: "utf8",
  }),
);
const [archive] = report;
const names = archive.files.map((file) => file.path).sort();

const REQUIRED = [
  "LICENSE",
  "NOTICE",
  "README.md",
  "dist/blasphem.js",
  "dist/blasphem_bg.wasm",
  "dist/index.d.ts",
  "package.json",
];
for (const required of REQUIRED) {
  if (!names.includes(required)) {
    throw new Error(`the archive is missing ${required}`);
  }
}

const FORBIDDEN = ["data/", "crates/", "reports/", "target/", "src/", "resources/"];
for (const name of names) {
  for (const forbidden of FORBIDDEN) {
    if (name.startsWith(forbidden)) {
      throw new Error(`the archive must not carry ${name}`);
    }
  }
}

rmSync(resolve(packageRoot, archive.filename), { force: true });
console.log(`status=packed files=${names.length} bytes=${archive.unpackedSize}`);
```

- [ ] **Step 5: Build and check**

```bash
cp LICENSE packages/blasphem/LICENSE
cd packages/blasphem && npm run build && npm run pack:check; cd -
```

Copy `LICENSE` after Task 16 creates it; until then use a placeholder file and replace it in Task 16.

Expected: `status=built` then `status=packed files=7 bytes=<number>`.

```bash
grep -c 'class BlasphemDetector' packages/blasphem/dist/blasphem.js
grep -c 'class BlasphemResult' packages/blasphem/dist/blasphem.js
```

Expected: both print `1`.

- [ ] **Step 6: Exclude the build output**

Append to `.gitignore`:

```text
/packages/blasphem/dist/
/packages/blasphem/node_modules/
/packages/blasphem/*.tgz
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "Add the private blasphem browser package"
```

---

### Task 15: Browser smoke on pinned Chromium

The current runner at `crates/blasphem-wasm/tests/run-browser-smoke.mjs` loads Playwright from `/Applications/ChatGPT.app` and Chrome from `/Applications`. Neither survives a fresh clone. This task moves the smoke into the package with a pinned Playwright and its bundled Chromium.

**Files:**
- Create: `packages/blasphem/scripts/browser-smoke.mjs`
- Create: `packages/blasphem/tests/smoke.html`
- Create: `packages/blasphem/chromium-revision.txt`
- Modify: `packages/blasphem/package.json` (devDependencies)
- Modify: `packages/blasphem/TOOLCHAIN.md`
- Delete: `crates/blasphem-wasm/verify-browser.sh`, `crates/blasphem-wasm/tests/run-browser-smoke.mjs`, `crates/blasphem-wasm/tests/browser-smoke.html`

**Interfaces:**
- Consumes: `packages/blasphem/dist` from Task 14.
- Produces: `reports/browser-smoke.json` with the same canonical-JSON shape the deleted runner wrote.

- [ ] **Step 1: Pin Playwright**

```bash
cd packages/blasphem
npm install --save-exact --save-dev @playwright/test
npx playwright install chromium
npx playwright --version
node -e "console.log(require('playwright-core/lib/server/registry/index.js').registry.findExecutable('chromium').browserVersion)" 2>/dev/null || true
cd -
```

Record the printed Playwright version in `packages/blasphem/TOOLCHAIN.md`. Write the Chromium revision that `npx playwright install chromium` reports into `packages/blasphem/chromium-revision.txt`, one line, digits only.

Confirm `package.json` now holds an exact devDependency, no caret:

```bash
python3 -c "
import json
d = json.load(open('packages/blasphem/package.json'))
v = d['devDependencies']['@playwright/test']
assert v[0].isdigit(), f'pin must be exact, found {v}'
print('playwright', v)
"
```

- [ ] **Step 2: Write the smoke page**

Create `packages/blasphem/tests/smoke.html`. Copy the case table from the deleted `crates/blasphem-wasm/tests/browser-smoke.html` so the coverage does not shrink, and change the import:

```html
<script type="module">
  import init, { BlasphemDetector } from "/dist/blasphem.js";
  // the existing case table and window.__blasphemReport assignment follow
</script>
```

The page must cover an explicit language route, an `AUTO` route that resolves, and an `AUTO` route that stays unknown. The unknown route must report `ok === true`.

- [ ] **Step 3: Write the runner**

Create `packages/blasphem/scripts/browser-smoke.mjs`. Copy the static file server, the canonical JSON writer, and the compressed-size records from the deleted runner. Change three things:

```js
import { chromium } from "@playwright/test";

const browser = await chromium.launch({ headless: true });
```

Serve `packageRoot` rather than the project root, and navigate to `/tests/smoke.html`. Assert the pinned Chromium revision before launching:

```js
const expectedRevision = readFileSync(resolve(packageRoot, "chromium-revision.txt"), "utf8").trim();
const actualRevision = chromium.executablePath().match(/chromium[-_](\d+)/)?.[1];
if (actualRevision !== expectedRevision) {
  throw new Error(`Chromium must be revision ${expectedRevision}, found ${actualRevision}`);
}
```

Write the report to `reports/browser-smoke.json` under the project root.

- [ ] **Step 4: Run the smoke**

```bash
cd packages/blasphem && npm run build && npm run test:browser; cd -
```

Expected: one line starting `status=passed`, and `reports/browser-smoke.json` exists with `"status":"passed"`.

- [ ] **Step 5: Delete the old runner**

```bash
git rm crates/blasphem-wasm/verify-browser.sh \
       crates/blasphem-wasm/tests/run-browser-smoke.mjs \
       crates/blasphem-wasm/tests/browser-smoke.html
grep -rn 'verify-browser' README.md docs/ || true
```

Update every reference the grep finds to the new command.

- [ ] **Step 6: Wire it into reproduce**

In `crates/blasphem-train/src/reproduce.rs` step 9, when `skip_browser` is false, run in `packages/blasphem`:

1. `npm ci`
2. `npm run build`
3. `npm run pack:check`
4. `npm run test:browser`

Each nonzero exit fails the step and names the command.

- [ ] **Step 7: Run the full reproduction**

```bash
cargo run --release --locked -p blasphem-train -- reproduce
echo "exit=$?"
```

Expected: `exit=0` and `status=reproduced steps=9`.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "Run the browser smoke on pinned Playwright Chromium"
```

---

### Task 16: Licensing, notices, and contributor documentation

**Files:**
- Create: `LICENSE`
- Create: `NOTICE`
- Create: `CONTRIBUTING.md`
- Modify: `packages/blasphem/NOTICE`
- Modify: `packages/blasphem/LICENSE`
- Modify: `README.md`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: the license fields in `resources/datasets/source-lock-v1.json` (Task 5, Task 8).

- [ ] **Step 1: Add the license**

```bash
curl -fsSL https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE
shasum -a 256 LICENSE
head -3 LICENSE
cp LICENSE packages/blasphem/LICENSE
```

Expected: the first lines read `Apache License`, `Version 2.0, January 2004`.

If the machine has no network, copy the text from any local Apache-2.0 crate checkout under `~/.cargo/registry`.

- [ ] **Step 2: Generate the notices**

```bash
python3 - <<'PY'
import json
lock = json.load(open("resources/datasets/source-lock-v1.json"))
rows = {}
for source in lock["sources"]:
    key = (source["dataset"], source["license_id"], source["license_url"], source["citation"])
    rows.setdefault(key, []).append(source["source_file_id"])

lines = [
    "# Third-party data notices",
    "",
    "Blasphem first-party code uses the Apache License 2.0. See LICENSE.",
    "",
    "The corpora and lexica below keep their own recorded licenses.",
    "",
]
for (dataset, license_id, license_url, citation), ids in sorted(rows.items()):
    lines.append(f"## {dataset}")
    lines.append("")
    lines.append(f"- License: {license_id}")
    lines.append(f"- License URL: {license_url}")
    lines.append(f"- Citation: {citation}")
    lines.append(f"- Sources: {len(ids)}")
    if license_id == "NOASSERTION":
        lines.append("- Status: the upstream license is unresolved. This record claims no permission.")
    if license_id == "CC-BY-NC-4.0":
        lines.append("- Status: noncommercial terms. Downstream users must check their own use.")
    lines.append("")
open("NOTICE", "w").write("\n".join(lines))
print("datasets:", len(rows))
PY
cat NOTICE
```

Expected: `datasets: 8` and a section for `germeval-2018` marked unresolved and one for `k-mhas` marked noncommercial.

- [ ] **Step 3: Write the package notice**

Create `packages/blasphem/NOTICE`. The browser build embeds the 15 HurtLex files and the 15 model artifacts, not the corpora. Cover exactly what ships:

```markdown
# Third-party notices for the blasphem browser package

The Blasphem code in this package uses the Apache License 2.0. See LICENSE.

## HurtLex 1.2

The package embeds the 15 HurtLex lexicon files.

- License: CC-BY-SA-4.0
- License URL: https://creativecommons.org/licenses/by-sa/4.0/
- Citation: HurtLex multilingual lexicon of offensive words, version 1.2

## Language detection tables

The language artifact derives from nitotm/eldc at commit
`a0301db809ff2e48a418018aa5359fb0c4354eb8`, Apache License 2.0, author Nito.

## Model artifacts

The 15 toxicity artifacts are weight tables trained on the corpora listed in the
repository NOTICE file. They contain no corpus text.
```

- [ ] **Step 4: Write the contribution guide**

Create `CONTRIBUTING.md` covering both paths the spec names.

The simple path: add a TSV under `data/raw-v1/community/{language}/{source_file_id}.tsv` using the canonical schema, add a source record with `source_role` `training_only`, run `prepare` with the evaluation lock, and open a pull request.

The custom path: add a typed adapter under `crates/blasphem-train/src/datasets/`, add fixtures under `crates/blasphem-train/tests/fixtures/`, and add adapter tests.

State the rules the pipeline enforces, one per line:

- The canonical schema is `native_id`, `label`, `text`, tab separated, with that header.
- The label is `toxic` or `clean`.
- A new source declares `source_role` `training_only`.
- Training-only rows enter only the development partition.
- A sealed baseline row wins a duplicate. The pipeline excludes the new copy.
- A duplicate with a conflicting label fails preparation.
- A row used to create a rule goes into `resources/datasets/rule-audit-v1.tsv` and never into later quality evidence.
- Pull request checks read only committed inputs. They fetch no contributor URL.

- [ ] **Step 5: Update the README**

Replace every `toxcheck`, `toxtrain`, and `toxbench` command with its Blasphem name. Add a reproduction section near the top:

````markdown
## Reproduce every artifact

```bash
cargo run --release --locked -p blasphem-train -- reproduce
```

The command reads only committed inputs. It downloads no corpus, lexicon, or
model source. It writes generated data to a temporary directory and returns a
nonzero status after any mismatch.
````

Add a licensing section pointing at `LICENSE` and `NOTICE`.

- [ ] **Step 6: Write the ignore rules**

Set `.gitignore` to:

```text
/target/
/data/prepared-v1/
/data/prepared-draft-v1/
/.superpowers/
/packages/blasphem/dist/
/packages/blasphem/node_modules/
/packages/blasphem/*.tgz
```

Both prepared directories hold derived data, and each carries a `provenance.tsv` above GitHub's 100 MB file limit.

- [ ] **Step 7: Verify no oversized or private file would be committed**

```bash
git add -A
git diff --cached --name-only | wc -l
git diff --cached --name-only | while read -r f; do
  [ -f "$f" ] && s=$(wc -c < "$f") && [ "$s" -gt 52428800 ] && echo "OVER 50 MiB: $f ($s)"
done
git diff --cached --name-only | grep -E '^\.superpowers/|^target/|^data/prepared' && echo "LEAK" || echo "no excluded path staged"
grep -rIl "$HOME" --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules . | head
```

Expected: no file over 50 MiB, `no excluded path staged`, and no file containing the home directory path. `large_db.h` is 50,798,963 bytes, which is below the 52,428,800-byte threshold.

If the home-path grep finds a file, fix that file before continuing. `crates/blasphem-language/UPSTREAM.md` and any doc that names `/private/tmp/eldc-main` must point at the vendored directory instead.

- [ ] **Step 8: Commit**

```bash
git commit -m "Add the Apache license, third-party notices, and contribution guide"
```

---

### Task 17: GitHub Actions

**Files:**
- Create: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `rust-toolchain.toml`, `.nvmrc`, the `reproduce` command, the npm scripts.

- [ ] **Step 1: Write the workflow**

Create `.github/workflows/ci.yml`:

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  canonical:
    name: canonical (x86_64-unknown-linux-gnu)
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4

      - name: Show the pinned toolchain
        run: |
          rustc --version
          cargo --version

      - uses: actions/setup-node@v4
        with:
          node-version-file: .nvmrc

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: cargo-${{ runner.os }}-${{ hashFiles('Cargo.lock', 'rust-toolchain.toml') }}

      - name: Install wasm-bindgen-cli
        run: cargo install wasm-bindgen-cli --version 0.2.127 --locked

      - name: Format
        run: cargo fmt --all --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets --locked -- -D warnings

      - name: Tests
        run: cargo test --workspace --locked

      - name: Install npm dependencies
        working-directory: packages/blasphem
        run: npm ci

      - name: Install pinned Chromium
        working-directory: packages/blasphem
        run: npx playwright install --with-deps chromium

      - name: Reproduce
        run: cargo run --release --locked -p blasphem-train -- reproduce

      - name: Upload the evidence reports
        uses: actions/upload-artifact@v4
        with:
          name: reports
          path: reports/

  macos:
    name: optional native bytes (aarch64-apple-darwin)
    runs-on: macos-15
    continue-on-error: true
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version-file: .nvmrc

      - name: Install wasm-bindgen-cli
        run: cargo install wasm-bindgen-cli --version 0.2.127 --locked

      - name: Reproduce without the browser
        run: cargo run --release --locked -p blasphem-train -- reproduce --skip-browser

      - name: Record the native binary digest
        run: shasum -a 256 target/release/blasphem
```

The canonical job proves model, language, native, and WASM identity. The macOS job proves model, language, and WASM identity plus a functional native build; it does not claim native binary identity across hosts.

- [ ] **Step 2: Validate the workflow locally**

```bash
python3 -c "
import sys
try:
    import yaml
except ImportError:
    sys.exit('install pyyaml or skip: pip install pyyaml')
d = yaml.safe_load(open('.github/workflows/ci.yml'))
print('jobs:', list(d['jobs']))
print('canonical steps:', len(d['jobs']['canonical']['steps']))
"
```

Expected: `jobs: ['canonical', 'macos']`.

- [ ] **Step 3: Run the same commands the canonical job runs**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo run --release --locked -p blasphem-train -- reproduce
```

Expected: every command exits 0.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "Add the GitHub Actions reproduction workflow"
```

---

### Task 18: Public repository delivery

Ask the user before this task runs. It creates a public repository and pushes the corpus.

**Files:** none created. This task publishes.

- [ ] **Step 1: Squash the history into one clean commit**

The repository holds one prior commit and per-task commits from this plan. The spec requires an initial public history with no secret, local path, task snapshot, or build directory. Confirm the working tree is clean and rewrite:

```bash
git status --short
git log --oneline
```

If the user wants one commit, run:

```bash
git checkout --orphan public-main
git add -A
git commit -m "Publish Blasphem"
git branch -M public-main main
```

Do not run this without the user's word. Ask first.

- [ ] **Step 2: Re-run the leak scan on the final tree**

```bash
git ls-files | wc -l
git ls-files | grep -E '^\.superpowers/|^target/|^data/prepared' && echo "LEAK" || echo "clean"
git ls-files -z | xargs -0 -n1 -I{} sh -c '[ -f "{}" ] && s=$(wc -c < "{}") && [ "$s" -gt 52428800 ] && echo "OVER: {} $s"' | head
git grep -lI "$HOME" -- . | head
git grep -lI "private/tmp" -- . | head
```

Expected: `clean`, no oversized file, and no match for the home path or `/private/tmp`.

- [ ] **Step 3: Confirm the repository builds from a fresh clone**

```bash
rm -rf /tmp/blasphem-fresh
git clone --no-hardlinks . /tmp/blasphem-fresh
cd /tmp/blasphem-fresh
cargo build --release --locked --bin blasphem
./target/release/blasphem check --language EN --text "You are an idiot"
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
cd -
```

Expected: the build succeeds, the check reports a nudge, and reproduce exits 0. The fresh clone downloads pinned Cargo dependencies. It downloads no corpus, lexicon, or model source.

- [ ] **Step 4: Create the public repository**

```bash
gh repo create sospedra/blasphem --public --source . --remote origin --push
gh repo view sospedra/blasphem --json name,visibility,defaultBranchRef
```

Expected: `"visibility":"PUBLIC"` and the default branch `main`.

- [ ] **Step 5: Confirm the push and the workflow**

```bash
git log --oneline origin/main -1
gh run list --repo sospedra/blasphem --limit 3
```

Expected: `origin/main` holds the verified commit and one workflow run is queued or running.

- [ ] **Step 6: Confirm nothing published to npm**

```bash
python3 -c "
import json
d = json.load(open('packages/blasphem/package.json'))
assert d['private'] is True
print('private:', d['private'])
"
history 2>/dev/null | grep -c 'npm publish' || echo 0
```

Expected: `private: True`. No `npm publish` command appears anywhere in this plan.

---

## Self-Review

**Spec coverage.**

| Spec section | Tasks |
| --- | --- |
| Naming | 1, 2, 3 |
| Repository structure | 1, 5, 7, 14, 16 |
| Reproduction command | 12, 13, 11 |
| Current reproduction corrections | 4, 5, 6, 7 |
| Corpus contribution contract | 8, 9, 10, 16 |
| Npm package | 14, 15 |
| Data flow | 6, 13, 14 |
| Error behavior | 9, 10, 12, 14 |
| Tests | 1, 4, 5, 6, 7, 8, 9, 10, 12, 14, 15 |
| Public repository delivery | 16, 17, 18 |
| Acceptance criteria | 12, 17, 18 |

**Two spec lines this plan interprets rather than implements literally.**

1. "Two clean canonical builds shall produce identical model, language, native, and WASM bytes." Task 17 runs the canonical job once per push. Byte identity across two runs is proven by the `reproduce` command's step 6 comparison against the committed manifest, which every run repeats. A second full job doubles CI time for the same signal. If the user wants an explicit double build, add a second `reproduce` invocation to the canonical job and diff `target/release/blasphem` across the two.

2. "Sealed baseline rows shall win when duplicate text appears in a new training source." Task 10 implements this inside the existing duplicate-group resolution. The current code already picks a representative; Task 10 changes only the ordering rule. An executor must read that code before editing, because the plan cannot quote a function it has not seen in full.

**Known ordering hazard.** Task 12 step 5 fails on Spanish and Malay digests until Task 13 regenerates the manifest. The step says so. Do not "fix" it by editing the manifest by hand.

**Type consistency.** `SourceRole` (Task 8) is used by `PreparationPolicy.source_roles` (Task 10) and by `FrozenSource.source_role` (Task 8). `sealed_partition_digest` and `verify_sealed_partitions` (Task 9) are called by `prepare` (Task 9 step 5) and `reproduce` step 3 (Task 12). `encode_sparse_v1` and `SparseV1Input` (Task 6) are called only by `compile_language`. `BatchCompileOptions` loses `spanish_legacy` in Task 6 and is constructed in Task 12 and Task 13 without it.
