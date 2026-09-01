# Blasphem JavaScript Contract and Packages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `blasphem` (wasm for browser and Node, napi-first on Node), `@blasphem/react-native` (Nitro), and `@blasphem/packs` (per-locale data) behind one `createJudge` contract, with no language data inside the core binary.

**Architecture:** The Rust core gains a bytes-in path: a `.pack` container per language (sparse artifact plus lexicon plus rule-pack version) and a `.detect` slice per language (that language's ELD entries with run offsets, so slices merge without the global bitmap). Embedded data moves behind a default-on `embedded` cargo feature that the wasm and napi crates turn off. A private TypeScript core (`packages/core`) owns the contract, locale table, manifest parsing, and load policy, and each published package inlines it at build by copying its sources. `blasphem-train pack` writes the packs, slices, and `manifest.json` into `packages/packs/dist`.

**Tech Stack:** Rust 1.97 (edition 2024), wasm-bindgen 0.2.127, napi 3 + napi-derive 3 + `@napi-rs/cli`, TypeScript 5.9 with `tsc`, pnpm 11.13 workspaces, Turborepo 2.10, Node 24.18, Playwright 1.62, Nitro Modules for React Native.

**Spec:** `docs/superpowers/specs/2026-09-03-blasphem-js-contract-and-packages-design.md`

**Status:** executed on 2026-09-03 in one session, inline. Every task's verification command ran; the outputs are in the spec's "Implementation notes". Task 11's iOS and Android application builds stay unverified.

## Global Constraints

- `Judge.judge()` is synchronous on every transport. `createJudge` is asynchronous.
- `locales` is required. `createJudge({})` and `createJudge({ locales: [] })` throw `BLASPHEM_LOCALES_EMPTY` before any byte loads.
- `judge()` never throws on an open judge. Unroutable text returns `{ safe: true, score: 0, locale: null, grawlix: null }`.
- Errors are plain `Error` objects with a `code` string from the spec table. Engines report `CODE: detail` strings and the core splits on the first `: `.
- Code reachable through the `browser` export condition never uses `new URL(x, import.meta.url)`. The wasm glue is generated with `--omit-default-module-path`.
- `blasphem` never pins pack digests. Digests come from `manifest.json` in `@blasphem/packs`. Rust verifies them; JavaScript never hashes.
- Every published package stays `private: true`.
- `@blasphem/react-native` does not depend on `blasphem`. It lists `blasphem` as an optional peer for the `browser` condition.
- New test files are opt-in in this repository. Extend existing test files and inline `#[cfg(test)]` modules. No new `tests/*.rs`, `*.test.*`, or spec files.
- No git commits. The user commits.
- Sizes in reports are MB with two decimals, 1 MB = 1,048,576 bytes.
- Bash tool runs zsh: quote globs, do not rely on word splitting.

## Measured inputs (2026-09-03)

- v2 language model: table 2,097,152 slots, 1,180,885 occupied, 350,377 live, 425,201 packed scores, 388,635 occupied runs, longest run 98 slots. Run offsets fit one byte.
- Scores per language: ar 16,598, de 37,268, en 28,494, es 90,491, fr 35,672, hi 2,092, it 31,277, ja 778, ko 326, ms 65,925, pt 36,189, ru 19,166, tr 39,747, vi 9,568, zh 11,610.
- Unicode tables inside the model: letter bits 8,192 B, CJK bits 8,192 B, lowercase 1,920 × u16. Total 20,224 B at model offset 136.

## File structure

Rust:
- Modify `Cargo.toml` (root): features `embedded`, explicit `[[bin]]` with `required-features`; add `crates/blasphem-napi` and `crates/blasphem-ffi` members.
- Modify `crates/blasphem-language/Cargo.toml`: feature `embedded-model` (default).
- Modify `crates/blasphem-language/src/lib.rs`: gate `EMBEDDED_MODEL` and `Detector::new`, make `Detector` fields `pub(crate)`, expose `h64`, `finish_detection`, `extract_features_with_tables`, `Language::from_code`, `Language::index`, add `pub mod slice`.
- Create `crates/blasphem-language/src/slice.rs`: `.detect` format, `SliceDetector`, `write_slices`, `write_tables`, `TABLES` include.
- Create `crates/blasphem-language/data/eld-tables-v1.bin` (20,224 B, generated, committed).
- Modify `crates/blasphem-language/src/tests.rs`, `crates/blasphem-language/tests/artifact.rs`, `crates/blasphem-language/tests/parity.rs`: slice tests.
- Create `src/pack.rs`: `.pack` format, `PackError`, `encode_pack`, `decode_pack`.
- Modify `src/embedded.rs`, `src/registry.rs`, `src/sparse.rs`, `src/runtime.rs`, `src/judge.rs`, `src/language_detection.rs`, `src/lib.rs`: `embedded` gates, `NudgeDetector::from_pack`, `Judge::from_packs`, `LanguageDetector::from_slices`, `PackSource`.
- Modify `tests/runtime_registry.rs`: pack round-trip tests.
- Create `crates/blasphem-train/src/pack.rs`, `crates/blasphem-train/src/locales_table.rs`; modify `crates/blasphem-train/src/main.rs`, `lib.rs`, `Cargo.toml`; extend `crates/blasphem-train/tests/cli.rs`.
- Rewrite `crates/blasphem-wasm/src/lib.rs`, `crates/blasphem-wasm/tests/core.rs`, `crates/blasphem-wasm/README.md`.
- Create `crates/blasphem-napi/{Cargo.toml,build.rs,src/lib.rs}`.
- Create `crates/blasphem-ffi/{Cargo.toml,src/lib.rs,include/blasphem.h}`.

TypeScript and packages:
- Create `packages/core/{package.json,tsconfig.json,src/contract.ts,src/errors.ts,src/locales.generated.ts,src/locales.ts,src/manifest.ts,src/loader.ts,src/transport.ts,src/index.ts}`.
- Create `packages/packs/{package.json,README.md,NOTICE,scripts/build.mjs}`; `dist/` generated and gitignored.
- Create `packages/node/{package.json,scripts/build.mjs,scripts/npm-dirs.mjs,npm/<target>/package.json × 7}`.
- Rewrite `packages/blasphem/src/{browser.ts,node.ts,native.ts}`; delete `src/index.ts,judge.ts,load.ts`; rewrite `scripts/build.mjs`, `scripts/pack-check.mjs`, `scripts/node-smoke.mjs`, `scripts/browser-smoke.mjs`, `tests/cases.mjs`, `tests/smoke.html`, `README.md`, `TOOLCHAIN.md`, `package.json`, `tsconfig.json`.
- Create `packages/react-native/{package.json,nitro.json,tsconfig.json,src/index.ts,src/BlasphemEngine.nitro.ts,cpp/HybridBlasphemEngine.{hpp,cpp},BlasphemReactNative.podspec,android/build.gradle,android/CMakeLists.txt,README.md,scripts/build.mjs}`.
- Modify `apps/web/integrations/blasphem-assets.ts`, `apps/web/astro.config.ts`, `apps/web/src/scripts/playground.ts`, `apps/web/src/lib/languages.ts`, `apps/web/src/lib/reports.ts`, `apps/web/src/components/Rite.astro` (import copy).
- Modify `turbo.json`, `pnpm-workspace.yaml`, `.gitignore`, `package.json` (root), `pnpm-lock.yaml` (via `pnpm install`).

---

### Task 1: Feature-gate the embedded data

**Files:**
- Modify: `Cargo.toml`, `crates/blasphem-language/Cargo.toml`, `crates/blasphem-language/src/lib.rs:18,208-212,818-826`, `src/lib.rs:4,23`, `src/embedded.rs`, `src/registry.rs:32-44,80-88,162-177,203-354`, `src/runtime.rs:14-18,49-69`, `src/sparse.rs:16-17,411-424`, `src/judge.rs:89-101`, `src/language_detection.rs:70-82`, `src/main.rs` (unchanged, gated by `required-features`).

**Interfaces:**
- Produces: cargo feature `blasphem/embedded` (default on) and `blasphem-language/embedded-model` (default on). With `--no-default-features --features language-detection` the crate has zero `include_bytes!` of language data.
- Produces: `NudgeDetector { language, model: Cow<'static, SparseModel>, rule_channel }` so Task 3 can own a model.
- Produces: `RegistryEntry::expected_rule_pack_version(language) -> Result<u16, RuntimeInitError>` (always compiled) for Task 3.

- [ ] **Step 1: Root features and explicit bin**

```toml
[[bin]]
name = "blasphem"
path = "src/main.rs"
required-features = ["embedded"]

[features]
default = ["language-detection", "embedded"]
language-detection = ["dep:blasphem-language"]
embedded = ["blasphem-language?/embedded-model"]

[dependencies]
blasphem-language = { path = "crates/blasphem-language", optional = true, default-features = false }
```

- [ ] **Step 2: Language crate feature**

```toml
[features]
default = ["embedded-model"]
embedded-model = []
```

Gate `static EMBEDDED_MODEL` and `Detector::new` with `#[cfg(feature = "embedded-model")]`. Gate the `extract_features` test helper and `mod tests` with `#[cfg(all(test, feature = "embedded-model"))]`.

- [ ] **Step 3: Gate root embedded paths**

`src/lib.rs`: `#[cfg(feature = "embedded")] mod embedded;` and the matching `pub use`. `src/sparse.rs`: gate `SPANISH_ARTIFACT`, `SPANISH_MODEL`, `embedded_model`, and the tests that read them. `src/registry.rs`: split `RegistryEntry` so `artifact`, `artifact_sha256`, `hurtlex_sha256`, `model` and `parse_model` exist only under `embedded`; keep `language`, profiles, `rule_pack_version`, `rule_pack_sha256`, `rules`, `rule_channel`, `validate_rule_identity`. Replace the `include_bytes!` table with two tables: a static `RULE_IDENTITIES: [(u16, [u8;32]); 15]` used always, and an `embedded`-only `ARTIFACTS: [(&'static [u8], [u8;32], Option<[u8;32]>); 15]`. `src/runtime.rs`: `model: Cow<'static, SparseModel>`; gate `from_hurtlex_bytes`, `validate_hurtlex`, `MissingHurtlex`, `UnexpectedHurtlex`, `HurtlexDigestMismatch` under `embedded`. `src/judge.rs`: gate `Judge::new` and `requested_locales` under `embedded`. `src/language_detection.rs`: gate `LanguageDetector::new` under `embedded`.

- [ ] **Step 4: Verify both configurations**

Run: `cargo build --locked -p blasphem --no-default-features --features language-detection 2>&1 | tail -3`
Expected: `Finished` with no errors.

Run: `cargo test --locked -p blasphem -p blasphem-language 2>&1 | grep -E "^test result|FAILED|error" | head`
Expected: every `test result:` line reports `0 failed`.

Run: `grep -rn "include_bytes" src crates/blasphem-language/src | grep -v "cfg" | wc -l` after adding `#[cfg(feature = "embedded")]` above each. Manual check that every `include_bytes!` of `resources/`, `data/`, or `blasphem-language-15-v2.bin` sits under an `embedded` or `embedded-model` gate.

---

### Task 2: Detect slices in `blasphem-language`

**Files:**
- Create: `crates/blasphem-language/src/slice.rs`, `crates/blasphem-language/data/eld-tables-v1.bin`
- Modify: `crates/blasphem-language/src/lib.rs`, `crates/blasphem-language/src/tests.rs`, `crates/blasphem-language/tests/artifact.rs`, `crates/blasphem-language/tests/parity.rs`

**Interfaces:**
- Produces:

```rust
pub mod slice {
    pub const SLICE_MAGIC: &[u8; 8] = b"BLSPHDET";
    pub const SLICE_FORMAT_VERSION: u32 = 1;
    pub const SLICE_HEADER_LEN: usize = 68;
    pub const TABLES: &[u8] = include_bytes!("../data/eld-tables-v1.bin"); // 20,224 bytes

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub enum SliceError { InvalidMagic, UnsupportedVersion(u32), UnknownLanguage([u8; 2]),
        InvalidTableLength(u32), InvalidSourceCommit, Truncated, TrailingData,
        InvalidEntry { index: usize }, Unsorted { index: usize }, DuplicateLanguage(Language), Empty }

    pub struct SliceDetector { /* private */ }
    impl SliceDetector {
        pub fn from_slices(slices: &[&[u8]]) -> Result<Self, SliceError>;
        pub fn languages(&self) -> Vec<Language>;
        pub fn detect(&self, text: &str) -> Detection;   // same type as Detector::detect
    }

    pub fn write_slices(model: &[u8]) -> Result<Vec<(Language, Vec<u8>)>, ModelError>; // all 15, ALL order
    pub fn write_tables(model: &[u8]) -> Result<Vec<u8>, ModelError>;                  // 20,224 bytes
}
impl Language { pub fn from_code(code: &str) -> Option<Self>; pub const fn index(self) -> usize; }
```

- Slice byte layout, little endian, no padding:

| Bytes | Value |
| --- | --- |
| 0..8 | `BLSPHDET` |
| 8..12 | u32 format version 1 |
| 12..14 | two ASCII bytes, lowercase code |
| 14..16 | u16 zero |
| 16..20 | u32 table length |
| 20..24 | u32 entry count n |
| 24..28 | f32 average score |
| 28..68 | 40 ASCII bytes upstream commit |
| 68.. | n × 12: u32 slot, u32 fingerprint, u32 packed = `(weight_bits & 0xffff_ff00) \| run_offset` |

Entries sorted by (fingerprint, slot), strictly increasing. `run_offset` = circular distance from the start of the occupied run to the slot.

- Lookup: `h = h64(feature)`, `F = ((h >> 32) as u32).max(1)`, `H = (h as u32 as usize) & mask`. Binary search the entries with fingerprint `F`. A candidate is reachable when `((slot - H) & mask) <= run_offset`. Take the reachable candidate with the smallest circular distance, then add every entry at that same slot and fingerprint. Then `finish_detection(&averages, raw, features.len())`.

- [ ] **Step 1: Write the failing tests in `src/tests.rs`**

Add a test that builds a 64-slot model with `with_bitmaps` and two live slots in one run, writes slices with `write_slices`, and asserts: each slice header, entry count per language, and that `SliceDetector::from_slices` over all slices returns `detect()` equal to `Detector::from_bytes(model).detect()` for three probe strings. Add a test that a slice with entries out of order fails with `SliceError::Unsorted { index: 1 }`. Add a test that `from_slices(&[])` fails with `SliceError::Empty`.

Run: `cargo test --locked -p blasphem-language slice 2>&1 | tail -5`
Expected: compile error, `slice` module missing.

- [ ] **Step 2: Implement `slice.rs`**

Writer: parse the model with `parse_model` (make `Detector` fields `pub(crate)`), compute `run_start` for every slot by scanning `occupied` circularly (start of a run is an occupied slot whose predecessor is unoccupied; if slot 0 and slot `table_len - 1` are both occupied, the run wraps), then for each live slot and each packed score push `(fingerprint, slot, weight_bits, run_offset)` to that language's vector, sort by (fingerprint, slot), serialize.

Reader: validate header, code known, commit equals `SOURCE_COMMIT`, `table_len` power of two ≥ 64, `n * 12` remaining bytes exactly, entries strictly increasing, `fingerprint != 0`, `slot < table_len`. Merge all slices into one `Vec<SliceEntry { fingerprint: u32, slot: u32, run_offset: u8, language: u8, weight: f32 }>` sorted by (fingerprint, slot, language). Reject a language given twice.

- [ ] **Step 3: Tables file and parity**

Add to `tests/artifact.rs`: `write_tables(committed model) == fs::read("data/eld-tables-v1.bin")`. Generate the file once with a Rust snippet through `cargo test` guard failing first, or with `dd if=data/blasphem-language-15-v2.bin of=data/eld-tables-v1.bin bs=1 skip=136 count=20224`. Add to `tests/parity.rs`: for every fixture row, `SliceDetector::from_slices(all 15 slices).detect(text)` equals `Detector::new().detect(text)`.

Run: `cargo test --locked -p blasphem-language 2>&1 | grep -E "^test result|FAILED"`
Expected: all `0 failed`.

---

### Task 3: Pack container and bytes-in judge in the root crate

**Files:**
- Create: `src/pack.rs`
- Modify: `src/lib.rs`, `src/runtime.rs`, `src/judge.rs`, `src/language_detection.rs`, `src/registry.rs`, `tests/runtime_registry.rs`

**Interfaces:**
- Produces:

```rust
pub const PACK_MAGIC: &[u8; 8] = b"BLSPHPCK";
pub const PACK_FORMAT_VERSION: u32 = 1;
pub const PACK_HEADER_LEN: usize = 24;

pub struct PackInput<'a> { pub language: Language, pub rule_pack_version: u16, pub artifact: &'a [u8], pub lexicon: &'a [u8] }
pub fn encode_pack(input: &PackInput<'_>) -> Vec<u8>;

pub struct DecodedPack<'a> { pub language: Language, pub rule_pack_version: u16, pub artifact: &'a [u8], pub lexicon: &'a [u8] }
pub fn decode_pack(bytes: &[u8]) -> Result<DecodedPack<'_>, PackError>;

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("BLASPHEM_DIGEST_MISMATCH: {file} expected {expected} actual {actual}")] DigestMismatch { file: String, expected: String, actual: String },
    #[error("BLASPHEM_FORMAT_VERSION: {file} has version {found}, this build accepts {accepted}")] FormatVersion { file: String, found: u32, accepted: u32 },
    #[error("BLASPHEM_PACK_INVALID: {file} {reason}")] Invalid { file: String, reason: String },
}

pub struct PackSource<'a> { pub language: Language, pub pack: &'a [u8], pub pack_sha256: Option<[u8; 32]>, pub detect: Option<&'a [u8]>, pub detect_sha256: Option<[u8; 32]> }

impl NudgeDetector { pub fn from_pack(language: Language, pack: &[u8]) -> Result<Self, RuntimeInitError>; }
impl Judge { pub fn from_packs(sources: &[PackSource<'_>], detect_language: bool, grawlix: bool) -> Result<Self, JudgeError>; }
impl LanguageDetector { pub fn from_slices(slices: &[&[u8]]) -> Result<Self, LanguageDetectorError>; }
```

Pack layout: `0..8` magic, `8..12` u32 version, `12..14` lowercase code, `14..16` u16 rule pack version, `16..20` u32 artifact length, `20..24` u32 lexicon length, then artifact bytes, then lexicon bytes. File names in errors are `{code}.pack` and `{code}.detect`.

`JudgeError` gains `Pack(#[from] PackError)`, `NoLocales`, `DuplicateLocale(Language)`, `MissingDetect(Language)`, `DetectionUnavailable` (built without `language-detection`). Every variant's `Display` starts with a spec code: `BLASPHEM_LOCALES_EMPTY`, `BLASPHEM_PACK_INVALID`, and so on.

`from_pack` validates: declared language equals requested, rule pack version equals `registry::expected_rule_pack_version`, `SparseModel::from_bytes(artifact)` metadata equals `language.profiles()`, then `entry.rule_channel(Some(lexicon))`.

- [ ] **Step 1: Failing tests in `tests/runtime_registry.rs`**

Add helpers that build a pack from `resources/models/multilingual-v2/*.bin` and `data/raw-v1/hurtlex/*` through `blasphem::encode_pack`, and slices through `blasphem_language::slice::write_slices` on `crates/blasphem-language/data/blasphem-language-15-v2.bin` (dev-dependency on `blasphem-language` already exists through the feature). Tests: `judge_from_packs_matches_embedded_judge` over EN and ES for the README text, `judge_from_packs_rejects_a_digest_mismatch`, `judge_from_packs_rejects_a_foreign_format_version`, `judge_from_packs_requires_detect_when_detection_is_on`, `judge_from_packs_rejects_empty_sources`.

Run: `cargo test --locked --test runtime_registry from_packs 2>&1 | tail -3`
Expected: compile error, `encode_pack` missing.

- [ ] **Step 2: Implement `src/pack.rs`, `from_pack`, `from_packs`, `from_slices`**

- [ ] **Step 3: Verify**

Run: `cargo test --locked -p blasphem 2>&1 | grep -E "^test result|FAILED"`
Expected: all `0 failed`.

Run: `cargo clippy --locked -p blasphem --no-default-features --features language-detection -- -D warnings 2>&1 | tail -3`
Expected: no warnings.

---

### Task 4: `blasphem-train pack` and `locales-table`

**Files:**
- Create: `crates/blasphem-train/src/pack.rs`, `crates/blasphem-train/src/locales_table.rs`
- Modify: `crates/blasphem-train/src/main.rs`, `crates/blasphem-train/src/lib.rs`, `crates/blasphem-train/Cargo.toml` (add `blasphem-language`), `crates/blasphem-train/tests/cli.rs`

**Interfaces:**
- Produces CLI:

```
blasphem-train pack --model-manifest resources/models/multilingual-v2/manifest.json \
  --model-root resources/models/multilingual-v2 \
  --language-model crates/blasphem-language/data/blasphem-language-15-v2.bin \
  --hurtlex-root data/raw-v1/hurtlex --output packages/packs/dist
blasphem-train locales-table --output packages/core/src/locales.generated.ts
```

- `pack` verifies each artifact against `artifact_sha256` and each lexicon against `hurtlex_sha256` from the model manifest, writes `{code}.pack`, `{code}.detect`, and `manifest.json`:

```json
{ "formatVersion": 1, "files": { "ar.detect": { "bytes": 199176, "sha256": "…64 hex…" }, "ar.pack": { "bytes": 260628, "sha256": "…" } } }
```

Keys sorted (BTreeMap). Prints `status=packed locales=15 bytes=<total>`.

- `locales-table` writes:

```ts
// Generated by `blasphem-train locales-table`. Do not edit.
export const LOCALES = [
  { code: "en", aliases: [] },
  { code: "zh", aliases: [] },
  { code: "es", aliases: [] },
  { code: "ar", aliases: [] },
  { code: "ms", aliases: ["id"] },
  { code: "pt", aliases: [] },
  { code: "fr", aliases: [] },
  { code: "hi", aliases: [] },
  { code: "ru", aliases: [] },
  { code: "ja", aliases: [] },
  { code: "de", aliases: [] },
  { code: "tr", aliases: [] },
  { code: "vi", aliases: [] },
  { code: "ko", aliases: [] },
  { code: "it", aliases: [] },
] as const;
export type LocaleCode = (typeof LOCALES)[number]["code"];
```

Order is `Language::ALL`. Aliases come from `Language::storage_code() != code()`.

- [ ] **Step 1: Failing CLI test in `tests/cli.rs`**: run `pack` into a temp dir, assert 31 files, manifest digests equal file digests, and `Judge::from_packs` loads `en` with detection.
- [ ] **Step 2: Implement both subcommands.**
- [ ] **Step 3: Verify**

Run: `cargo test --locked -p blasphem-train --test cli pack 2>&1 | tail -3`
Expected: `test result: ok`.

Run: `cargo run --release --locked -p blasphem-train -- pack --model-manifest resources/models/multilingual-v2/manifest.json --model-root resources/models/multilingual-v2 --language-model crates/blasphem-language/data/blasphem-language-15-v2.bin --hurtlex-root data/raw-v1/hurtlex --output packages/packs/dist && ls packages/packs/dist | wc -l`
Expected: `31`.

---

### Task 5: wasm crate on the bytes-in engine

**Files:**
- Rewrite: `crates/blasphem-wasm/src/lib.rs`, `crates/blasphem-wasm/tests/core.rs`, `crates/blasphem-wasm/README.md`

**Interfaces:**
- Produces JS classes (wasm-bindgen):

```ts
class BlasphemEngineBuilder {
  constructor(detectLanguage: boolean, grawlix: boolean);
  add(locale: string, pack: Uint8Array, packSha256: string | undefined, detect: Uint8Array | undefined, detectSha256: string | undefined): void;
  build(): BlasphemEngine;   // consumes the builder
}
class BlasphemEngine {
  readonly locales: string[];
  judge(text: string): { safe: boolean; score: number; locale: string | null; grawlix: string | null };
  free(): void;
}
```

- Produces Rust `JudgeCore::from_sources(sources: Vec<OwnedSource>, detect_language, grawlix) -> Result<Self, String>` where the error string is `JudgeError`'s `Display` (`CODE: detail`).
- Removes `DetectorCore`, `WasmDetector`, `WasmCheckResult`, `CoreResult`, `AUTO`.

- [ ] **Step 1: Rewrite `tests/core.rs`** to build sources from repo files (same helpers as Task 3) and assert: EN judge scores the README text `0.64`, unknown locale string fails with `BLASPHEM_LOCALE_UNSUPPORTED`, digest mismatch fails with `BLASPHEM_DIGEST_MISMATCH`, `detectLanguage` with missing detect fails with `BLASPHEM_PACK_INVALID`.
- [ ] **Step 2: Implement.** `add` decodes the hex digest to `[u8; 32]` or fails with `BLASPHEM_PACK_INVALID: {locale}.pack digest is not 64 hex characters`.
- [ ] **Step 3: Verify**

Run: `cargo test --locked -p blasphem-wasm 2>&1 | grep -E "^test result|FAILED"`
Expected: `0 failed`.

Run: `CARGO_TARGET_DIR=target/npm-wasm cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm && stat -f%z target/npm-wasm/wasm32-unknown-unknown/release/blasphem_wasm.wasm`
Expected: under 1,300,000 bytes.

---

### Task 6: napi crate and `packages/node`

**Files:**
- Create: `crates/blasphem-napi/Cargo.toml`, `crates/blasphem-napi/build.rs`, `crates/blasphem-napi/src/lib.rs`
- Create: `packages/node/package.json`, `packages/node/scripts/build.mjs`, `packages/node/scripts/npm-dirs.mjs`, `packages/node/npm/{darwin-arm64,darwin-x64,linux-x64-gnu,linux-arm64-gnu,linux-x64-musl,linux-arm64-musl,win32-x64-msvc}/package.json`
- Modify: `Cargo.toml` (workspace members), `pnpm-workspace.yaml` (`packages/node/npm/*`), `.gitignore` (`*.node`)

**Interfaces:**
- Produces napi classes `EngineBuilder` and `Engine` with the same shape as Task 5, where `Engine.close()` drops the core and later `judge` fails with `BLASPHEM_CLOSED`.
- Platform package `@blasphem/node-<target>`: `{ name, version, private: true, os, cpu, libc?, main: "blasphem.<target>.node", files: ["blasphem.<target>.node"], license: "Apache-2.0" }`.
- Root `crates/blasphem-napi` depends on `blasphem` with `default-features = false, features = ["language-detection"]`.

- [ ] **Step 1: Crate** (mirror of the spike in `/tmp/blasphem-spike/napi`, plus `EngineBuilder`).
- [ ] **Step 2: `packages/node/scripts/build.mjs`**: `cargo build --release --locked -p blasphem-napi`, copy `target/release/libblasphem_napi.{dylib,so}` or `blasphem_napi.dll` to `npm/<host-target>/blasphem.<host-target>.node`. Host target from `process.platform`, `process.arch`, and `process.report.getReport().header.glibcVersionRuntime`.
- [ ] **Step 3: Verify**

Run: `pnpm --filter @blasphem/node run build && node -e 'const m=require("./packages/node/npm/darwin-arm64/blasphem.darwin-arm64.node"); console.log(Object.keys(m))'`
Expected: `[ 'EngineBuilder', 'Engine' ]`.

---

### Task 7: `packages/core`

**Files:**
- Create: `packages/core/package.json` (`@blasphem/core`, private, `check: tsc --noEmit`), `packages/core/tsconfig.json`, `packages/core/src/*.ts`

**Interfaces:**
- Produces:

```ts
// contract.ts
export interface JudgeOptions { locales: string[]; assets?: string; detectLanguage?: boolean; grawlix?: boolean }
export interface Judgement { safe: boolean; score: number; locale: string | null; grawlix: string | null }
export interface Judge { readonly locales: readonly string[]; readonly transport: "wasm" | "native"; judge(text: string): Judgement; close(): void }
// errors.ts
export type ErrorCode = "BLASPHEM_LOCALES_EMPTY" | "BLASPHEM_LOCALE_UNSUPPORTED" | "BLASPHEM_LOCALE_MISSING" | "BLASPHEM_ASSETS_REQUIRED" | "BLASPHEM_FETCH_FAILED" | "BLASPHEM_DIGEST_MISMATCH" | "BLASPHEM_FORMAT_VERSION" | "BLASPHEM_PACK_INVALID" | "BLASPHEM_CLOSED";
export function fail(code: ErrorCode, message: string): Error & { code: ErrorCode };
export function fromEngineError(error: unknown): Error & { code: ErrorCode };   // splits "CODE: detail"
// locales.ts
export function normalizeLocales(input: unknown): string[];   // throws LOCALES_EMPTY / LOCALE_UNSUPPORTED; returns ALL order, deduplicated
// manifest.ts
export interface Manifest { formatVersion: 1; files: Record<string, { bytes: number; sha256: string }> }
export function parseManifest(bytes: Uint8Array): Manifest;   // throws FORMAT_VERSION / PACK_INVALID
// transport.ts
export interface Entry { locale: string; pack: Uint8Array; packSha256: string; detect: Uint8Array | null; detectSha256: string | null }
export interface EngineHandle { judge(text: string): Judgement; free(): void }
export interface Transport { readonly name: "wasm" | "native"; read(name: string): Promise<Uint8Array>; engine(entries: Entry[], detectLanguage: boolean, grawlix: boolean): Promise<EngineHandle> }
// loader.ts
export function createJudgeWith(transport: Transport, options: JudgeOptions): Promise<Judge>;
```

- `createJudgeWith`: normalize → read `manifest.json` → require `{code}.pack` (+ `{code}.detect` when detection) → `Promise.all` reads → `transport.engine` → `Judge` with a `closed` flag.

- [ ] **Step 1: Write the files.**
- [ ] **Step 2: Verify**

Run: `pnpm --filter @blasphem/core run check`
Expected: exit 0, no output.

---

### Task 8: `@blasphem/packs`

**Files:**
- Create: `packages/packs/package.json`, `packages/packs/scripts/build.mjs`, `packages/packs/README.md`, `packages/packs/NOTICE`
- Modify: `.gitignore` (`/packages/packs/dist/`)

**Interfaces:**
- `package.json`: name `@blasphem/packs`, `private: true`, `license: "CC-BY-NC-SA-4.0"` (matches `packages/blasphem/NOTICE`), `exports: { "./package.json": "./package.json", "./manifest.json": "./dist/manifest.json", "./*": "./dist/*" }`, `files: ["dist", "README.md", "NOTICE"]`, `scripts.build` runs `node scripts/build.mjs` which runs the Task 4 `pack` command.

- [ ] **Step 1: Write files.**
- [ ] **Step 2: Verify**

Run: `pnpm --filter @blasphem/packs run build && node -e 'const m=require("./packages/packs/dist/manifest.json"); console.log(Object.keys(m.files).length, m.formatVersion)'`
Expected: `30 1`.

---

### Task 9: `blasphem` package on the new contract

**Files:**
- Create: `packages/blasphem/src/browser.ts`, `packages/blasphem/src/node.ts`, `packages/blasphem/src/native.ts`
- Delete: `packages/blasphem/src/index.ts`, `judge.ts`, `load.ts`
- Rewrite: `scripts/build.mjs`, `scripts/wasm.mjs` (add `--omit-default-module-path`), `scripts/pack-check.mjs`, `scripts/node-smoke.mjs`, `tests/cases.mjs`, `package.json`, `tsconfig.json`, `README.md`, `TOOLCHAIN.md`
- Modify: `.gitignore` (`/packages/blasphem/src/core/`)

**Interfaces:**
- `package.json` exports:

```json
{
  ".": { "types": "./dist/browser.d.ts", "browser": "./dist/browser.js", "node": "./dist/node.js", "default": "./dist/browser.js" },
  "./blasphem_bg.wasm": "./dist/blasphem_bg.wasm",
  "./package.json": "./package.json"
}
```

with `main`, `module`, `types` pointing at `dist/browser.js` / `dist/browser.d.ts`, `sideEffects: false`, and `optionalDependencies` on the seven `@blasphem/node-*` packages (`workspace:*`).

- `browser.ts`: `createJudge(options)` throws `BLASPHEM_ASSETS_REQUIRED` without `assets`; transport `read(name)` fetches `${base}/${name}`; `engine()` runs `init({ module_or_path: `${base}/blasphem_bg.wasm` })` once, then the builder.
- `node.ts`: packs dir = `options.assets ?? dirname(fileURLToPath(import.meta.resolve("@blasphem/packs/manifest.json")))`; `engine()` tries `loadNative()` from `native.ts` (package name from platform, arch, libc), else wasm from `import.meta.resolve("blasphem/blasphem_bg.wasm")` through `readFile`.
- `build.mjs`: `rm -rf src/core dist`, copy `../core/src/*.ts` into `src/core/`, build wasm, glue with `--omit-default-module-path`, `tsc`, copy glue into `dist`, print `status=built wasm_bytes=… glue_bytes=…`.
- `tests/cases.mjs`: `runCases(createJudge, assets)` builds judges per case: supplied cases `{ locales: [code], detectLanguage: false, grawlix: true }`, auto cases all 15 with detection, unknown cases through the auto judge, README example `{ locales: ["en","es"], detectLanguage: true, grawlix: true }`, alias `id` equals `ms`, invalid locales reject with `BLASPHEM_LOCALE_UNSUPPORTED`, `createJudge({})` rejects with `BLASPHEM_LOCALES_EMPTY`, `close()` then `judge()` throws `BLASPHEM_CLOSED`.
- `node-smoke.mjs`: runs the cases twice, once with `BLASPHEM_FORCE_WASM=1` (env read by `node.ts` to skip native), asserts `transport` is `"native"` then `"wasm"` and verdicts match. Prints `status=passed node=… native_cases=… wasm_cases=…`.

- [ ] **Step 1–4: Implement, build, run.**

Run: `pnpm --filter blasphem run build && pnpm --filter blasphem test`
Expected: `status=built …`, `status=packed …`, `status=passed …`.

---

### Task 10: Browser smoke on packs

**Files:**
- Rewrite: `packages/blasphem/scripts/browser-smoke.mjs`, `packages/blasphem/tests/smoke.html`
- Modify: `apps/web/src/lib/reports.ts` fields if the report schema changes

- Serve `/dist/` (blasphem dist), `/packs/` (`packages/packs/dist`), `/tests/`. `smoke.html` copies `blasphem_bg.wasm` expectation: `assets` = an origin path where both the wasm and the packs are reachable, so the server maps `/assets/blasphem_bg.wasm` to dist and `/assets/*` else to packs.
- `smoke.html` runs `runCases`, then builds an EN-only judge on a fresh page section after recording `performance.getEntriesByType("resource")` and asserts the new requests are exactly `manifest.json`, `en.pack`, `en.detect` (wasm already cached from the first judge). Report gains `en_only_requests` and `schema_version: 4`; drops `browser_builds.explicit_only`.

Run: `pnpm --filter blasphem run test:browser`
Expected: `status=passed engines="chromium …, webkit …" …`.

---

### Task 11: `@blasphem/react-native` on Nitro

**Files:**
- Create: `crates/blasphem-ffi/Cargo.toml`, `crates/blasphem-ffi/src/lib.rs`, `crates/blasphem-ffi/include/blasphem.h`
- Create: `packages/react-native/package.json`, `nitro.json`, `tsconfig.json`, `src/BlasphemEngine.nitro.ts`, `src/index.ts`, `cpp/HybridBlasphemEngine.hpp`, `cpp/HybridBlasphemEngine.cpp`, `BlasphemReactNative.podspec`, `android/build.gradle`, `android/CMakeLists.txt`, `README.md`, `scripts/build.mjs`

**Interfaces:**
- C ABI (`blasphem.h`):

```c
typedef struct blasphem_builder blasphem_builder;
typedef struct blasphem_engine blasphem_engine;
typedef struct { bool safe; double score; const char* locale; const char* grawlix; } blasphem_judgement; // strings owned by the engine call; free with blasphem_judgement_free
blasphem_builder* blasphem_builder_new(bool detect_language, bool grawlix);
int blasphem_builder_add(blasphem_builder*, const char* locale, const uint8_t* pack, size_t pack_len, const char* pack_sha256, const uint8_t* detect, size_t detect_len, const char* detect_sha256); // 0 ok, else error; message via blasphem_last_error
blasphem_engine* blasphem_builder_build(blasphem_builder*);   // consumes builder; NULL on error
blasphem_judgement blasphem_engine_judge(const blasphem_engine*, const char* text);
void blasphem_judgement_free(blasphem_judgement);
void blasphem_engine_free(blasphem_engine*);
const char* blasphem_last_error(void);   // thread-local, valid until next call
```

- Nitro spec `src/BlasphemEngine.nitro.ts`:

```ts
import type { HybridObject } from "react-native-nitro-modules";
export interface EngineBuilder extends HybridObject<{ ios: "c++"; android: "c++" }> {
  add(locale: string, pack: ArrayBuffer, packSha256: string | undefined, detect: ArrayBuffer | undefined, detectSha256: string | undefined): void;
  build(): Engine;
}
export interface Engine extends HybridObject<{ ios: "c++"; android: "c++" }> {
  readonly locales: string[];
  judge(text: string): { safe: boolean; score: number; locale: string | null; grawlix: string | null };
  close(): void;
}
export interface BlasphemFactory extends HybridObject<{ ios: "c++"; android: "c++" }> {
  createBuilder(detectLanguage: boolean, grawlix: boolean): EngineBuilder;
  readBundled(name: string): Promise<ArrayBuffer>;   // reads a pack from the app bundle
}
```

- `src/index.ts`: `createJudge(options)` uses the inlined core with a native transport: `read(name)` → `factory.readBundled(name)`, `engine()` → builder.
- `package.json`: name `@blasphem/react-native`, private, `peerDependencies: { "react-native": "*", "react-native-nitro-modules": "*", "blasphem": "workspace:*" }`, `peerDependenciesMeta.blasphem.optional = true`, exports `{ ".": { "react-native": "./dist/index.js", "browser": "./dist/web.js", "default": "./dist/index.js" } }`, `web.ts` re-exports `createJudge` from `blasphem`.
- Verification here is bounded: `cargo build --release -p blasphem-ffi --target aarch64-apple-ios` and `--target aarch64-linux-android` (targets installed), `clang -fsyntax-only` on a C file including `blasphem.h`, `tsc --noEmit` on the TypeScript. Full iOS and Android module builds need an example app and are reported as unverified.

---

### Task 12: `apps/web` on `createJudge`

**Files:**
- Modify: `apps/web/integrations/blasphem-assets.ts` (copy `browser.js`, `blasphem.js`, `blasphem_bg.wasm` from dist plus every file in `packages/packs/dist`; base hash over all of them; `__BLASPHEM_TOTAL_BYTES__` replaces `__BLASPHEM_WASM_BYTES__`), `apps/web/astro.config.ts` (pass `packsDir`), `apps/web/src/scripts/playground.ts` (`createJudge` per selection, cached in a `Map`, `assets: BASE`), `apps/web/src/lib/languages.ts` (import `LOCALES` from the copied generated table via `../../../packages/core/src/locales.generated`), `apps/web/src/components/Rite.astro` (usage snippet), `apps/web/src/lib/reports.ts` (report schema 4).

Run: `pnpm --filter web run check && pnpm --filter web run build 2>&1 | tail -3`
Expected: `Complete!` and the copied asset log line.

---

### Task 13: Workspace wiring and full verification

**Files:**
- Modify: `turbo.json`, `pnpm-workspace.yaml`, root `package.json`, `.gitignore`

- `turbo.json` tasks: `@blasphem/core#check`, `blasphem#build` (inputs add `$TURBO_ROOT$/packages/core/src/**`, drop hurtlex and models), `@blasphem/packs#build` (inputs: Cargo files, `crates/**`, `src/**`, `resources/models/**`, `data/raw-v1/hurtlex/**`, `crates/blasphem-language/data/**`), `@blasphem/node#build`, `blasphem#test` dependsOn `blasphem#build`, `@blasphem/packs#build`, `@blasphem/node#build`; `web#build` dependsOn `blasphem#build`, `@blasphem/packs#build`.

Run, in order, and paste output into the final report:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked`
4. `pnpm install` (lockfile update for new packages) then `pnpm build`
5. `pnpm test`
6. `pnpm --filter blasphem run test:browser`
7. `git status --short` (no unintended files)

Then amend the spec with the implementation decisions (wasm served from `assets`, builder API, `CODE: detail` messages, tables compiled in, measured slice sizes, packs license per NOTICE) and update `packages/blasphem/README.md`, `TOOLCHAIN.md`, `crates/blasphem-wasm/README.md`.

---

### Task 14: Go, Python, and Java bindings (added 2026-09-04)

**Files:** `crates/blasphem-ffi` (`blasphem_builder_error`, builder survives a failed build), `crates/blasphem-train/src/locales_table.rs` (`--format go|python|java`), `packages/go/*`, `crates/blasphem-python/*` (standalone workspace), `packages/python/*`, `packages/python-packs/*`, `packages/java/*`.

Verification that ran: `go vet ./... && go run ./example ../../packages/packs/dist`; `maturin develop --release` then the README snippet in a Python 3.14 venv, plus `uv build` of the data wheel and `init(["en"])` without `assets`; `javac --release 22 -Xlint:all -Werror` on JDK 25 then `java --enable-native-access=ALL-UNNAMED ... Main`. All three printed `score 0.64`, `locale en`, the grawlix, fail-open on Korean, and `BLASPHEM_LOCALE_UNSUPPORTED` for `xx`.

### Task 15: Svelte and Solid (added 2026-09-04)

Throwaway apps in `/tmp/fw` from `create-vite` templates, tarballs from `pnpm pack`, `blasphem-assets public/blasphem`, `vite build`, then Playwright in Chromium and WebKit reading the rendered verdict. Both passed. README gained the two components and the SSR note.
