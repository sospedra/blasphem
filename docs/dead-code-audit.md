# Dead code audit

Date: 2026-09-03

Tree: branch `development` at `363a202`, plus the uncommitted workspace-lints diff in the five `Cargo.toml` files, `src/detector.rs`, `src/text.rs`, and `tests/detector.rs`.

## Verdict

True dead code is small. API that only tests keep alive is the larger finding. Every dependency, module, fixture, and report is in use.

| Scope | Dead | Alive only through tests |
|---|---|---|
| Rust lib `blasphem` | 2 items | 6 items |
| Rust lib `blasphem-train` | 2 items | 55 items (28 public, 27 private helpers) |
| Rust lib `blasphem-bench` | 0 | 1 item |
| Rust lib `blasphem-language` | 0 | 0 |
| Web | 2 symbols, 2 CSS classes, 2 CSS tokens | none |
| Files and config | 1 sample, 1 tool script, 1 profile table | none |

## Dead code, no caller anywhere

Rust:

- `src/text.rs:98` field `original` and `src/text.rs:158` method `TextDocument::original`. No `.original` access exists outside the definition.
- `crates/blasphem-train/src/evaluation_lock.rs:88` `compute_sealed_partitions`. Only the definition matches.
- `crates/blasphem-train/src/regenerate.rs:26` `PUBLICATION_STEP`. Only the definition matches.

Web:

- `apps/web/src/lib/languages.ts:43` export `languageByCode`. knip reports it. No import exists.
- `apps/web/src/scripts/playground.ts:10` const `NUDGE_THRESHOLD`. `astro check` with `noUnusedLocals` reports it.
- `apps/web/src/styles/codex.css:152` `.inline-code` and `:337` `.ledger .them`. No markup uses either class.
- `apps/web/src/styles/tokens.css:4` `--burgundy` and `:20` `--font-poster`. No `var()` reads them.

Files and config:

- `samples/eval.tsv`. No path reference exists. The two `eval.tsv` hits at `crates/blasphem-train/tests/cli.rs:201` and `:233` are temp files.
- `crates/blasphem-language/tools/generate-parity-fixture.py`. Only a comment in `tools/c-oracle.c` names it. No doc mentions it.
- `Cargo.toml:61` `[profile.bench]`, in the uncommitted diff. The workspace has no bench target and no `cargo bench` call.

## Alive only through tests

Root crate `blasphem`:

- `src/detector.rs:40` `Detector::is_match`. Callers: `tests/detector.rs`, `tests/policy.rs`, `tests/text.rs`.
- `src/policy.rs:139` `PolicyResult::has_rule`. Caller: `tests/policy.rs`.
- `src/registry.rs:13` `LanguageSpec`, `:21` `LanguageSpec::new`, `:185` `LANGUAGE_SPECS`, `:356` `language_spec`. Caller: `tests/profile_contract.rs:189`. The re-export at `src/lib.rs:43` serves only that test.

`blasphem-bench`:

- `crates/blasphem-bench/src/lib.rs:215` `canonical_json_bytes`. Caller: `tests/auto_validation_contract.rs:111`.

`blasphem-train`, 28 public items:

- TextDetox acquisition in `crates/blasphem-train/src/acquisition.rs`: `:27 TEXTDETOX_REVISION_URL`, `:119 source_record_from_request`, `:656 sha256_digest`, `:753 AcquiredTextDetox`, `:759 TextDetoxAcquisitionError`, `:814 TextDetoxFetchError`, `:849 acquire_textdetox`, `:951 fetch_textdetox`. The binary uses its own `acquire_textdetox_source` at `main.rs:596`.
- TextDetox preparation in `crates/blasphem-train/src/datasets/textdetox.rs`: `:122 TEXTDETOX_PREPARATION_VERSION`, `:168 detector_code`, `:253 ProvenanceStatus`, `:262 ProvenanceRow`, `:273 TextDetoxSummary`, `:283 PreparedTextDetox`, `:293 rows`, `:303 split_for_key`, `:339 prepare_textdetox`, `:750 textdetox_rows_url`, `:784 write_textdetox_eval_tsv`, `:801 write_textdetox_provenance_tsv`.
- TextDetox publication in `crates/blasphem-train/src/publication.rs`: `:121 TextDetoxPublicationError`, `:139 publish_prepared_textdetox`.
- `crates/blasphem-train/src/calibration.rs:50` `calibrate`. The compiler uses `calibrate_at_or_above` instead, see `compiler.rs:19`.
- `crates/blasphem-train/src/datasets/ibrohim_budi.rs:47` `import_indonesian` and `datasets/told_br.rs:64` `import_told_br`. The prepare step imports through `DatasetAdapter` structs, see `preparation.rs:170`.
- `crates/blasphem-train/src/evidence.rs:75` `parse_canonical_json`. `textdetox_http.rs:82` `requester` and `:87` `sleeper`.

`blasphem-train`, 27 private helpers reachable only from the items above:

- `crates/blasphem-train/src/acquisition.rs`: `:845 RevisionDocument`, `:975 publish_acquired_with`, `:1000 map_atomic_fetch_error`, `:1016 acquisition_staging_path`, `:1025 source_row_index`, `:1032 read_revision`
- `crates/blasphem-train/src/datasets/ibrohim_budi.rs`: `:11 IBROHIM_BUDI_SOURCE_FILE_ID`
- `crates/blasphem-train/src/datasets/textdetox.rs`: `:123 FNV_OFFSET`, `:124 FNV_PRIME`, `:209 parse_detector_code`, `:311 fnv_hash`, `:324 group_id`, `:331 TextGroup`, `:346 prepare_textdetox_with_group_id`, `:473 validate_included_languages`, `:485 validate_source_ids`, `:498 unclassified_provenance`, `:846 evaluation_label`, `:853 split_name`, `:861 provenance_status_name`
- `crates/blasphem-train/src/datasets/told_br.rs`: `:11 TOLD_BR_SOURCE_FILE_ID`
- `crates/blasphem-train/src/datasets/types.rs`: `:140 dataset_id`, `:141 label_conversion_version`
- `crates/blasphem-train/src/publication.rs`: `:160 write_textdetox_staged_file`, `:171 publish_directory_with`, `:195 map_atomic_publication_error`, `:211 publication_staging_path`

## Duplicates and loose ends

- `data/hurtlex/*.tsv`, 15 files, are byte-identical to `data/raw-v1/hurtlex/*/1.2/*.tsv`. Only the CLI defaults at `crates/blasphem-train/src/main.rs:166` and `:174` and `tests/reproduce.rs:15` point at them. The corpus single-source spec defers this at `docs/superpowers/specs/2026-09-03-blasphem-corpus-single-source-design.md:196`.
- `data/source-observation-v1/source-observation-v1.json` is a stale copy of `data/raw-v1/source-observation-v1.json`. It has older timestamps and no `source_role`. The spec removes it at line 185. `README.md:341` and `:348` still point at it.
- `resources/models/es-chargram-v1.bin` duplicates `multilingual-v2/es-chargram-v1.bin`. `regenerate.rs:32` writes the copy on purpose.
- Three docs have no inbound link: `docs/multilingual-dataset-audit.md`, `docs/multilingual-experimental-report.md`, `docs/spanish-long-text-benchmark.md`.
- `export` is unnecessary on 10 types in `apps/web/src/lib/reports.ts`, on `Tone` at `apps/web/src/scripts/playground-state.ts:37`, and on `crateManifest` at `packages/blasphem/scripts/crate.mjs:7`. Each is used only in its own file.
- The wasm classes `BlasphemDetector` and `BlasphemResult` have no consumer in `packages/` or `apps/`. Only `crates/blasphem-wasm/tests/browser-smoke.html` and the READMEs use them. The npm package uses `BlasphemJudge` only.

## Clean results

- Every Cargo dependency is used by at least one target of its crate. Source: the `unused_crate_dependencies` lint, aggregated per target.
- `unreachable_pub` reports nothing.
- Every `.rs` file under `src/` and `crates/*/src/` is declared as a module.
- knip finds no unused npm dependency.
- All 7 `reports/*.json` files are loaded by `apps/web/src/lib/reports.ts`.
- All fixtures, models, fonts, and assets are referenced by path.
- The one "unused file" from knip, `crates/blasphem-wasm/tests/run-browser-smoke.mjs`, is a false positive. `verify-browser.sh:22` runs it.
- `#[allow(dead_code)]` at `crates/blasphem-train/src/atomic_publish.rs:10` is justified. The variant is built only under the `cfg(not(...))` branch at line 85.

## Method

Rust, per lib crate, in a temp copy of the tree with its own `CARGO_TARGET_DIR`:

1. Turn every `pub` into `pub(crate)` and run `cargo check -p <crate> --lib --message-format=json`. rustc then reports every item, which yields the full inventory.
2. For each item that was `pub`, grep the callers outside the lib: the crate's own `main.rs`, `tests/`, `examples/`, and the other workspace crates. Match methods and fields as `.name` or `::name`, and everything else as a whole word.
3. Turn only the items without a caller into `pub(crate)`. Split each `pub use` re-export the same way. Compile the lib alone and read the `dead_code` output. Types named by a `private_interfaces` warning stay `pub`. Repeat until stable.
4. Run step 3 twice. Once with tests as callers, once without. The difference is the API that only tests keep alive.

Web: `pnpm dlx knip --no-progress`, then `astro check --tsconfig <copy with noUnusedLocals and noUnusedParameters>`, then a grep of every CSS class selector and custom property against the `.astro` and `.ts` sources.

Files: for every tracked non-source file, grep the basename and the parent-relative path across the tree, then `cmp` the suspected duplicates.

Limits. Callers are matched by name, so a dead method that shares a name with a live one can hide. Enum variants were not analyzed. Every finding above was confirmed by a direct grep of the definition and its callers.

## Evidence

```
===== blasphem: inventory=541 dead=2 test-only=6
===== blasphem-language: inventory=44 dead=0 test-only=0
===== blasphem-bench: inventory=228 dead=0 test-only=1
===== blasphem-train: inventory=893 dead=2 test-only=55
$ cargo check --workspace --all-targets --locked   # original tree, after the temp builds
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.83s
$ pnpm dlx knip --no-progress
Unused files (1)  crates/blasphem-wasm/tests/run-browser-smoke.mjs
Unused exports (2)  languageByCode  apps/web/src/lib/languages.ts:43:17
                    crateManifest   packages/blasphem/scripts/crate.mjs:7:14
Unused exported types (11)  reports.ts x10, playground-state.ts:37 Tone
$ astro check --tsconfig <noUnusedLocals>
src/scripts/playground.ts:10:7 - error ts(6133): 'NUDGE_THRESHOLD' is declared but its value is never read.
$ cmp data/hurtlex/hurtlex_XX.tsv data/raw-v1/hurtlex/XX/1.2/hurtlex_XX.tsv   # identical=15 of 15
```

## Verdicts

Date: 2026-09-03. Each unused item exposes one of two defects. Either the code is a leftover and goes, or the live path lacks what the code does and must adopt it.

| Verdict | Meaning | Count |
|---|---|---|
| Remove | Delete the item, its re-export, and its tests | 17 |
| Use | Wire the item into the live path, delete the parallel copy | 9 |
| Move | Delete from the library, keep a helper in the tests | 2 |
| Decide | Product call between two designs | 1 |

### Dead code

- `TextDocument::original` (`src/text.rs:98`, `:158`). Remove. `Detector::check` and `PolicyResult.original_text` keep the caller's text. The field copies the whole input in `TextDocument::build` for nothing. Delete the field, the getter, and the copy.
- `compute_sealed_partitions` (`crates/blasphem-train/src/evaluation_lock.rs:88`). Use. The live path only verifies (`verify_sealed_partitions`, `reproduce.rs:208`, `preparation.rs:120`). No command can produce or reseed `resources/datasets/evaluation-lock-v1.json`. The single-source spec requires reseeded digests that equal the current ones (spec lines 240-246). This function computes them. Expose it in the reseed step of that migration. If the spec drops the reseed, delete it.
- `PUBLICATION_STEP` (`crates/blasphem-train/src/regenerate.rs:26`). Remove. `RegenerateReport` holds files, not steps (`regenerate.rs:101`). Publication errors carry the path. Delete the constant.
- `languageByCode` (`apps/web/src/lib/languages.ts:43`). Remove. Every component maps over `LANGUAGES`. The playground prints the raw `locale` on purpose (`playground.ts:144`). Delete the function.
- `NUDGE_THRESHOLD` (`apps/web/src/scripts/playground.ts:10`). Use. The meter draws the threshold tick at a hardcoded `left: 50%` (`Detector.astro:311-318`). Two copies of one number. Write the constant into a CSS custom property that `.meter::after` reads, so the tick follows the constant. Otherwise delete the constant.
- `.inline-code` (`apps/web/src/styles/codex.css:152`). Remove. Every inline `<code>` sits inside `.copy`, which the sibling selector `.copy code` styles (`Colophon.astro:18,21`, `Rite.astro:41-43,75`). Delete the selector.
- `.ledger .them` (`codex.css:337`). Remove. `Ledger.astro:18` renders every data cell as `.us`. No comparison column exists. Delete the rule. Consider neutral names for `.us` and `th.mine`.
- `--burgundy` (`apps/web/src/styles/tokens.css:4`). Remove. No rule reads it. No hardcoded `#4a1418` exists.
- `--font-poster` (`tokens.css:20`). Remove, with the font. The plan's stamp used Archivo 800 (web plan lines 2287, 2298). The shipped `Seal.astro:19` uses `--font-goth`. So `@fontsource/archivo/800.css` (`Codex.astro:10`) ships font bytes nobody renders, and `@fontsource/archivo` (`apps/web/package.json:18`) is a dependency for nothing. Delete the token, the import, and the dependency.
- `samples/eval.tsv`. Remove. A three-row English sample for `eval`. README documents `eval` with `samples/spanish-audit.tsv` (`README.md:204-207`). The reproduce test copies `samples/` wholesale (`tests/reproduce.rs:15`), so the file only adds bytes to every reproduction. `git rm samples/eval.tsv`.
- `crates/blasphem-language/tools/generate-parity-fixture.py`. Use, by documenting it. It regenerates `tests/fixtures/c-parity-v1.jsonl`, which `tests/parity.rs` and the bench `verify_c_parity_fixture` consume as evidence. It calls `build-c-oracle.sh` (line 109). Both default to the machine-local path `/private/tmp/eldc-audit-20260902`. `UPSTREAM.md` explains the pin, not the regeneration. Add a "Regenerate the parity fixture" section to `UPSTREAM.md` with both commands and the two variables `ELDC_UPSTREAM_ROOT` and `ELDC_UPSTREAM_DIR`. Add the script to the path list in `tests/rename_contract.rs:31`.
- `[profile.bench]` (`Cargo.toml:61`). Use, or drop. `profile.release` strips symbols, so `cargo run --release --example profile_dense` gives an unsymbolized profile. This profile is the fix, and nothing invokes it. Document `cargo run --profile bench -p blasphem-bench --example profile_dense` under "Performance gate" (`README.md:392`). Otherwise delete the table.

### Alive only through tests, root crate

- `Detection::is_match` (`src/detector.rs:40`). Move. Production reads `.matches` and `.score` (`src/main.rs:126,142`, `src/workflow.rs:120`). Only tests want the boolean, 18 call sites. Delete the method. Tests use `!detection.matches.is_empty()`.
- `PolicyResult::has_rule` (`src/policy.rs:139`). Move. No production code asks whether one rule fired. The CLI prints the evidence list. Tests call it 36 times (`tests/policy.rs`). Delete the method and add a helper in `tests/policy.rs`.
- `LanguageSpec`, `LanguageSpec::new`, `LANGUAGE_SPECS`, `language_spec` (`src/registry.rs:13-29`, `:185-201`, `:356`). Remove. `LanguageSpec::new` copies `Language::profiles()`. `RegistryEntry` copies the same profiles again (`registry.rs:60`). The only test asserts the copy equals its source (`tests/profile_contract.rs:187-199`), which cannot fail. Delete the struct, the table, the function, the `pub use` at `src/lib.rs:43`, and that test.
- `canonical_json_bytes` (`crates/blasphem-bench/src/lib.rs:215`). Use. `write_canonical` in `crates/blasphem-bench/src/main.rs:185` calls `serde_jcs::to_vec` directly. Call `canonical_json_bytes` there, so the crate has one canonicalization site.

### Alive only through tests, train crate

- The legacy TextDetox pipeline. Remove. Two pipelines exist. The live one: `observe` and `acquire` download the Parquet file, `parse_textdetox_parquet` and `write_textdetox_source_tsv` canonicalize it (`main.rs:551-613`), `prepare` imports through `TextDetoxAdapter` (`preparation.rs:170-175`), `publish_prepared` writes the corpus with provenance (`publication.rs:220`, `:704`), and `compiler.rs:19` calibrates with `calibrate_at_or_above`. The legacy one pages the Hugging Face rows API (`textdetox_rows_url`, `acquire_textdetox`, `fetch_textdetox`, `TEXTDETOX_REVISION_URL`), splits by FNV hash (`split_for_key`, `prepare_textdetox`, `TEXTDETOX_PREPARATION_VERSION`), writes `development.tsv`, `validation.tsv`, `test.tsv`, and `provenance.tsv` into `data/textdetox/es-prepared` (`publish_prepared_textdetox`), and calibrates with `calibrate`, a one-line wrapper (`calibration.rs:50-55`). It produced the Spanish artifact once; `es-chargram-v1.json` still records `split_method: FNV-1a`. `regenerate` now republishes that artifact from the compiled manifest (`regenerate.rs:224-238`). Delete: `acquisition.rs:753-1040` and `:27`, `source_record_from_request` (`acquisition.rs:119`; `main.rs:398` uses the `_with_download` variant), `sha256_digest` at `acquisition.rs:656` (a copy of `evidence.rs:83`), `datasets/textdetox.rs:122-124`, `:168`, `:209`, `:253-360`, `:473-510`, `:750-764`, `:784-830`, `:846-870`, `publication.rs:121-219`, `calibration.rs:50-55`, the matching names in `lib.rs:22-41`, the `.gitignore` line `/data/textdetox/es-prepared/`, and the tests `acquisition.rs`, `provenance.rs`, and the wrapper cases in `calibration.rs`, `preparation.rs`, `publication.rs`. Keep `TextDetoxLanguage`, `TextDetoxSourceRow`, `parse_textdetox_*`, `write_textdetox_source_tsv`, and `TextDetoxError`. They are live.
- `import_indonesian`, `import_told_br`, `IBROHIM_BUDI_SOURCE_FILE_ID`, `TOLD_BR_SOURCE_FILE_ID` (`datasets/ibrohim_budi.rs:11,47`, `datasets/told_br.rs:11,64`). Remove. Both wrap the private `import_source` that `IbrohimBudiAdapter::import` and `ToldBrAdapter::import` already call. The other five adapters have no such wrapper. Only `tests/dataset_adapters.rs:6,10,116-182` call them. Delete both functions and both constants. Test through the adapters like the other five.
- `DatasetAdapter::dataset_id` and `::label_conversion_version` (`datasets/types.rs:140-141`). Use. Every adapter declares its label conversion version, for example `told_br.rs:51-53`, yet `publication.rs:553-566` hardcodes the same nine strings in a second table. Two sources of truth for a value that lands in the prepared manifest. Make `publish_prepared` read the version from the adapters and delete the duplicate match. Do the same with `dataset_id`, or drop it from the trait.
- `parse_canonical_json` (`crates/blasphem-train/src/evidence.rs:75`). Use. It is the only reader that rejects non-canonical bytes. `json_value_is_current` (`regenerate.rs:414-427`) parses the committed file as a loose `Value`, so a hand-edited but equivalent file counts as current and is never rewritten in canonical form. The lock and manifest readers (`reproduce.rs:435`, `evaluation_lock.rs:60`, `model_manifest.rs:301`) accept non-canonical files too. Compare `canonical_json_bytes(value)?` with the committed bytes in `json_value_is_current`. Parse the committed locks with `parse_canonical_json`.
- `RetryingTextDetoxClient::requester` and `::sleeper` (`textdetox_http.rs:82-89`). Remove. They exist so three tests can read the fake's counters back out of the client (`tests/textdetox_http.rs:23-61`). Give the fakes shared handles that the tests keep, then delete the two getters.

### Duplicates and loose ends

- `data/hurtlex/`, 15 files. Remove. The live readers use the nested layout `XX/1.2/hurtlex_XX.tsv` (`verification.rs:1286-1289`, `regenerate.rs` `HURTLEX_ROOT`, every README command). Two legacy commands touch the flat copy. `setup` downloads it from GitHub (`main.rs:650-690`), and `eval` reads it through `load_lexica` (`main.rs:174`, `src/workflow.rs:66`). The committed raw-v1 lexica make the download redundant. Retire `setup`, point `load_lexica` at the nested layout or at `embedded_hurtlex_bytes`, drop `data/hurtlex` from `COPIED_ENTRIES` (`tests/reproduce.rs:15`), then `git rm -r data/hurtlex`. The spec deferred this (line 196). It is one commit.
- `data/source-observation-v1/source-observation-v1.json`. Remove. The spec removes it (line 185). Point `README.md:341-348` at `data/raw-v1/source-observation-v1.json` for both `observe --output` and `freeze-sources --observation`.
- `resources/models/es-chargram-v1.bin` and `.json`. Remove the copies. The runtime includes the same file from two paths: `src/sparse.rs:16` for `embedded_model` (`policy.rs:338`), and `src/registry.rs:226` from `multilingual-v2`. Whether the linker merges the two blobs is unverified. `regenerate.rs:224-238` republishes both copies, and `es-chargram-v1.json` repeats the manifest entry (`spanish_record`, `regenerate.rs:361`) with no reader. Read the Spanish model from `registry_entry(Language::Es)` in `sparse.rs`, point `tests/sparse_v2.rs:24` and `tests/spanish_compatibility.rs:12` at the `multilingual-v2` path, delete `publish_spanish_artifact` and `publish_spanish_record`, and `git rm` both files.
- Orphan docs. `docs/multilingual-dataset-audit.md`: use, link it from CONTRIBUTING "Rules the pipeline enforces" (line 81); it is the rationale for source roles and sealed splits. `docs/eldc-auto-report.md`: use, `README.md:453` describes it without a link. `docs/multilingual-precision-recall-benchmark.md` and `docs/spanish-proof-report.md`: use, link from README "Pre-test evidence" (line 275) and "Spanish checks" (line 199); `spanish-proof-report.md:118` still names the legacy `data/textdetox/es-prepared/test.tsv`. `docs/multilingual-experimental-report.md`: remove or rewrite; lines 108-111 document `toxcheck_wasm.js` and `WasmDetector`, names that no longer exist, and README carries its verdict. `docs/spanish-long-text-benchmark.md`: remove; Spanish-only numbers from 2026-09-01 that `reports/multilingual-performance.json` and the perf gate supersede.
- `export` on 10 nested types in `apps/web/src/lib/reports.ts`, on `Tone` (`playground-state.ts:37`), on `crateManifest` (`packages/blasphem/scripts/crate.mjs:7`). Remove the keyword. Each is used only in its own file.
- `BlasphemDetector` and `BlasphemResult` (`crates/blasphem-wasm/src/lib.rs:165-246`) with `DetectorCore` and `CoreResult`. Decide. The product API is `judge()` in the npm package. The classes survive because `browser-smoke.html:112-130` asserts on `threshold`, `evaluated`, `resolvedLanguage`, `languageReliable`, and `languageScore`, which `BlasphemJudge` does not expose, and because `README.md:419-425` still leads with `BlasphemDetector`. Two JS APIs are two things to keep in sync and glue bytes in every download. Preferred: add the diagnostic fields to the judge result or a `BlasphemJudge.inspect()`, port the smoke, remove `WasmDetector`, `WasmCheckResult`, `DetectorCore`, `CoreResult`, the `tests/core.rs` cases, and the `REQUIRED_CLASSES` entries in `packages/blasphem/scripts/build.mjs:10`, and rewrite `README.md:419-425` around `judge()`. Alternative: keep both and demote `BlasphemDetector` in README to a diagnostics API.

### Keep as is

- `#[allow(dead_code)]` on `AtomicPublishError::Unsupported` (`atomic_publish.rs:10`). The variant is built under `cfg(not(...))` at line 85.
- The `BlasphemResult` getters. `browser-smoke.html` reads all eight.
- The `run-browser-smoke.mjs` file. `verify-browser.sh:22` runs it.
