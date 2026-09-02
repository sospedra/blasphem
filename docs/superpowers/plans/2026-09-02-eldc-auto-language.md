# ELDC Automatic Language Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the selected ELDC detector to pure Rust and add explicit or automatic language routing.

**Architecture:** A new `eldc` crate embeds one generated 15-profile database. The `toxcheck` library maps ELDC results to concrete toxicity detectors. Native and WASM adapters share the same routing types.

**Tech Stack:** Rust 2024, fixed binary data, Cargo, wasm-bindgen, upstream ELDC C oracle

**Spec:** `docs/superpowers/specs/2026-09-02-eldc-auto-language-design.md`

## Global constraints

The shipping runtime contains no C, FFI, subprocess, network request, or neural-network runtime.

The ELDC source pin is `a0301db809ff2e48a418018aa5359fb0c4354eb8`.

The canonical language list is `EN,ZH,ES,AR,MS,PT,FR,HI,RU,JA,DE,TR,VI,KO,IT`.

The parser accepts `ID` as an explicit alias for `MS`.

The model contains only these 15 language score profiles.

Unknown automatic input fails open.

The existing explicit toxicity decisions remain unchanged.

The root and WASM crates can compile without the optional ELDC feature.

The workspace has no Git repository. Do not create commits or branches.

---

### Task 1: ELDC model importer

**Files:**

- Create: `crates/eldc/Cargo.toml`
- Create: `crates/eldc/src/bin/import_eldc.rs`
- Create: `crates/eldc/UPSTREAM.md`
- Modify: `Cargo.toml`
- Test: `crates/eldc/tests/import_format.rs`

**Interfaces:**

- Consumes: The pinned `large_db.h`, `eld_unicode_bits.h`, and `eld_tolower.h` files.
- Produces: `eldc-15-v1.bin` with the table, filtered blob, and Unicode tables.

- [ ] **Step 1: Write the binary-format failure tests**

Test the magic, version, section lengths, source digest, and truncated input cases.

- [ ] **Step 2: Run the importer tests**

Run: `cargo test -p eldc --test import_format`

Expected: The tests fail because the parser and model types do not exist.

- [ ] **Step 3: Write the importer**

Parse hexadecimal table fields and decimal blob values without a C compiler.

Map upstream indexes `1,9,11,12,17,20,25,26,29,36,42,44,54,57,59` into 15 compact indexes.

Retain occupied table slots when their filtered score count is zero.

- [ ] **Step 4: Generate the model artifact**

Run: `cargo run -p eldc --bin import_eldc -- /private/tmp/eldc-audit-20260902/src/eldc crates/eldc/data/eldc-15-v1.bin`

Expected: The command writes a validated artifact and prints its SHA-256 digest.

- [ ] **Step 5: Run the importer tests again**

Run: `cargo test -p eldc --test import_format`

Expected: All importer tests pass.

### Task 2: Pure Rust ELDC core

**Files:**

- Create: `crates/eldc/src/lib.rs`
- Create: `crates/eldc/src/model.rs`
- Create: `crates/eldc/src/features.rs`
- Create: `crates/eldc/src/detector.rs`
- Test: `crates/eldc/tests/core.rs`

**Interfaces:**

- Consumes: `Model::embedded()` and the version-one artifact.
- Produces: `Detector::detect(&str) -> Detection`.

- [ ] **Step 1: Write feature-boundary tests**

Test ASCII words, internal apostrophes, CJK characters, duplicate features, NUL bytes, and the 1,000-byte limit.

- [ ] **Step 2: Run the core tests**

Run: `cargo test -p eldc --test core`

Expected: The tests fail because detection is not implemented.

- [ ] **Step 3: Port feature extraction**

Use byte slices and `u64::from_le_bytes` for packed features.

Use wrapping multiplication in the ELDC hash.

Stop after 500 unique features.

- [ ] **Step 4: Port scoring and reliability**

Accumulate truncated weights in `[f32; 15]`.

Normalize with the upstream `f32` formula.

Sort with upstream selected-language order.

- [ ] **Step 5: Run the core tests again**

Run: `cargo test -p eldc --test core`

Expected: All core tests pass.

### Task 3: C parity oracle

**Files:**

- Create: `crates/eldc/tests/fixtures/c-parity-v1.jsonl`
- Create: `crates/eldc/tests/parity.rs`
- Create: `crates/eldc/tools/build_c_oracle.sh`

**Interfaces:**

- Consumes: The pinned upstream C source and the same selected-language mask.
- Produces: Frozen C outputs and a Rust parity gate.

- [ ] **Step 1: Build the temporary C oracle**

Run the pinned CLI with `-l ar,de,en,es,fr,hi,it,ja,ko,ms,pt,ru,tr,vi,zh --scores 15 --reliable`.

- [ ] **Step 2: Freeze representative oracle rows**

Include upstream examples, every supported language, short text, CJK text, mixed text, and boundary inputs.

- [ ] **Step 3: Write the parity test**

Compare the selected language, reliability, feature count, score order, and scores within `0.000001`.

- [ ] **Step 4: Run the parity test**

Run: `cargo test -p eldc --test parity`

Expected: All parity rows pass without compiling C during the test.

### Task 4: Product language and routing API

**Files:**

- Modify: `src/language.rs`
- Create: `src/language_detection.rs`
- Modify: `src/lib.rs`
- Modify: `src/registry.rs`
- Modify: `src/rules/packs/word.rs`
- Modify: `src/rules/channel.rs`
- Modify: `Cargo.toml`
- Test: `tests/language_detection.rs`
- Test: `tests/profile_contract.rs`

**Interfaces:**

- Consumes: `eldc::Detector::detect` and existing `Language` values.
- Produces: `LanguageSelection`, `LanguageResolution`, and `LanguageDetector`.

- [ ] **Step 1: Write language contract tests**

Require canonical `MS`, the `ID` alias, `AUTO`, explicit bypass, reliable routing, and unknown routing.

- [ ] **Step 2: Run the language tests**

Run: `cargo test --test language_detection --test profile_contract`

Expected: The tests fail because the new public types do not exist.

- [ ] **Step 3: Add canonical Malay support**

Replace the internal `Id` variant with `Ms` at discriminant four.

Keep Indonesian model and HurtLex resource paths as legacy storage paths.

Accept both `MS` and `ID` in `FromStr`.

- [ ] **Step 4: Add automatic resolution**

Map the 15 ELDC codes to the matching toxicity language.

Map an unreliable or absent result to `LanguageResolution::Unknown`.

- [ ] **Step 5: Run the language tests again**

Run: `cargo test --test language_detection --test profile_contract`

Expected: All language tests pass.

### Task 5: Native CLI routing

**Files:**

- Modify: `src/main.rs`
- Test: `tests/toxcheck_cli.rs`
- Test: `tests/multilingual_cli_contract.rs`

**Interfaces:**

- Consumes: `LanguageSelection` and `LanguageDetector`.
- Produces: Explicit and automatic `toxcheck check` behavior.

- [ ] **Step 1: Write CLI failure tests**

Test explicit `MS`, alias `ID`, automatic supported text, automatic unknown text, and invalid codes.

- [ ] **Step 2: Run the CLI tests**

Run: `cargo test --test toxcheck_cli --test multilingual_cli_contract`

Expected: New automatic tests fail.

- [ ] **Step 3: Add CLI routing**

Resolve `AUTO` before the CLI loads a HurtLex file.

Use the legacy `ID` resource directory for canonical `MS`.

Print a stable routing line after the unchanged primary result line.

- [ ] **Step 4: Run the CLI tests again**

Run: `cargo test --test toxcheck_cli --test multilingual_cli_contract`

Expected: All CLI tests pass.

### Task 6: Browser routing

**Files:**

- Modify: `crates/toxcheck-wasm/src/lib.rs`
- Modify: `crates/toxcheck-wasm/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `crates/toxcheck-wasm/tests/core.rs`
- Modify: `crates/toxcheck-wasm/tests/browser-smoke.html`
- Modify: `crates/toxcheck-wasm/tests/run-browser-smoke.mjs`
- Modify: `crates/toxcheck-wasm/README.md`

**Interfaces:**

- Consumes: The shared language detector and embedded toxicity resources.
- Produces: Automatic WASM construction and route diagnostics.

- [ ] **Step 1: Write browser-core failure tests**

Test explicit `MS`, alias `ID`, automatic routing for 15 languages, and unknown text.

- [ ] **Step 2: Run the WASM core tests**

Run: `cargo test -p toxcheck-wasm`

Expected: New automatic tests fail.

- [ ] **Step 3: Add WASM automatic routing**

Keep the full experimental bundle capable of resolving all 15 toxicity detectors.

Return route diagnostics with every result.

Make ELDC an optional feature for explicit-only root and WASM builds.

- [ ] **Step 4: Run the WASM core tests again**

Run: `cargo test -p toxcheck-wasm`

Expected: All WASM core tests pass.

- [ ] **Step 5: Run Chromium verification**

Run: `./crates/toxcheck-wasm/verify-browser.sh`

Expected: Chromium reports native-equivalent explicit and automatic results.

- [ ] **Step 6: Build explicit-only artifacts**

Run: `cargo build -p toxcheck --lib --no-default-features --release`

Run: `cargo build -p toxcheck-wasm --no-default-features --release --target wasm32-unknown-unknown`

Expected: Both builds omit the ELDC dependency and complete successfully.

### Task 7: Regression, size, and speed evidence

**Files:**

- Modify: `crates/toxbench/src/benchmark.rs`
- Modify: `crates/toxbench/src/size.rs`
- Modify: `README.md`
- Create: `reports/eldc-auto-validation.json`
- Create: `docs/eldc-auto-report.md`

**Interfaces:**

- Consumes: The finished native and browser runtime.
- Produces: Reproducible acceptance evidence.

- [ ] **Step 1: Run all explicit regression tests**

Run: `cargo test --workspace --locked`

Expected: Every existing explicit decision passes with `ID` accepted as an alias.

- [ ] **Step 2: Run static checks**

Run: `cargo fmt --all -- --check`

Run: `cargo clippy --locked --all-targets -- -D warnings`

Expected: Both commands exit successfully.

- [ ] **Step 3: Measure automatic detection**

Measure cold initialization and warm p50 and p95 latency for short and 4 KiB messages.

- [ ] **Step 4: Measure artifacts**

Record native bytes and WASM raw, gzip, and Brotli bytes.

Record the explicit-only artifact sizes without ELDC.

- [ ] **Step 5: Write the acceptance report**

Record parity counts, routing accuracy, unknown rates, explicit regressions, size, and latency.

- [ ] **Step 6: Run the final verification commands**

Run: `cargo test --workspace --release --locked`

Run: `cargo build --release --locked --bin toxcheck`

Run: `./crates/toxcheck-wasm/verify-browser.sh`

Expected: Every command exits successfully.

## Productization continuation

This continuation starts after the experimental evidence is accepted.

It is not part of Tasks 1 through 7.

1. Define a versioned toxicity pack format for one language.

2. Move sparse, HurtLex, language rules, exclusions, and reactivation data out of shared WASM.

3. Add a manifest with canonical codes, digests, and compressed sizes.

4. Refactor static detector references into owned or shared pack storage.

5. Add an asynchronous loader with a supported language set and optional AUTO mode.

6. Cache one parsed ELDC instance across automatic detectors.

7. Measure one-language, selected-N, and all-language downloads and heap use.
