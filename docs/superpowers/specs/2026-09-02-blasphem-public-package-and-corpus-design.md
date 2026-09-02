# Blasphem public package and corpus design

## Status

The user approved this design in chat on 2026-09-02.

## Goals

The project shall use the Blasphem name for every first-party package and product artifact.

The repository shall build the native binary and browser package from a fresh clone.

The repository shall contain every corpus and model input required for data-offline regeneration.

Contributors shall be able to add labeled corpus rows without changing sealed evaluation data.

The repository shall export an npm package named `blasphem`.

The npm package shall remain unpublished during this work.

The public repository shall be `sospedra/blasphem`.

## Non-goals

This work shall not add a neural model or an AI runtime.

This work shall not add selected-language browser packs.

This work shall not publish a crate or npm package.

This work shall not promise stable public APIs before the experimental release matures.

## Naming

The root Cargo package, library, and CLI shall use `blasphem`.

The language detector crate shall use `blasphem-language` and `blasphem_language`.

Its directory shall be `crates/blasphem-language`.

The browser crate shall use `blasphem-wasm` and `blasphem_wasm`.

Its directory shall be `crates/blasphem-wasm`.

The training crate shall use `blasphem-train` and `blasphem_train`.

Its directory shall be `crates/blasphem-train`.

The evidence crate shall use `blasphem-bench` and `blasphem_bench`.

Its directory shall be `crates/blasphem-bench`.

The language model builder shall use the `blasphem-language-model` binary name.

The language artifact shall use `blasphem-language-15-v1.bin`.

Its eight-byte magic shall use `BLASPHEM`.

The npm package shall use the exact unscoped name `blasphem`.

The JavaScript constructor shall use `BlasphemDetector`.

The JavaScript result class shall use `BlasphemResult`.

The generated browser files shall use `blasphem.js` and `blasphem_bg.wasm`.

The term ELDC shall remain only in third-party attribution and pinned upstream records.

Neutral domain names such as HurtLex, language, corpus, model, and detector shall remain unchanged.

## Repository structure

The repository shall track all Rust source, tests, fixtures, documentation, and lock files.

The repository shall track the 37 current raw corpus files under `data/raw-v1`.

The repository shall move the Spanish source to `data/raw-v1/textdetox/es.tsv`.

The source lock shall register Spanish as the 38th raw input.

The repository shall exclude the derived `data/textdetox/es-prepared` directory.

The repository shall track the four pinned language source headers.

Those headers shall live under `crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8`.

The repository shall verify each header against its recorded SHA-256 value.

The repository shall track the imported language artifact.

The repository shall track all sparse runtime models and their manifest.

The repository shall track behavior fixtures and corpus audit exclusions.

The repository shall exclude `target`, `dist`, npm archives, and internal task snapshots.

The repository shall exclude both prepared data directories.

Those directories contain derived data and two files above GitHub's 100 MB file limit.

The reproduction command shall regenerate the excluded prepared data in a temporary directory.

Data-offline means that this command shall not download a corpus, lexicon, or model source.

The first dependency setup may download pinned Rust, npm, and browser-test dependencies.

## Reproduction command

The command shall be:

```bash
cargo run --release --locked -p blasphem-train -- reproduce
```

The read-only command shall perform these actions in order:

1. Verify every raw file against `source-lock-v1.json`.
2. Generate prepared data in a temporary directory.
3. Verify sealed validation and test hashes.
4. Compile all 15 toxicity model artifacts.
5. Rebuild the language detector artifact from its vendored source tables.
6. Compare every generated model hash with the committed manifest.
7. Build the native release binary.
8. Build the default and explicit-only WASM modules.
9. Run the Rust tests, Clippy, npm checks, and browser smoke test.

The command shall return a nonzero status after any mismatch or failed check.

The repository shall provide a separate intentional update command:

```bash
cargo run --release --locked -p blasphem-train -- regenerate
```

The update command shall write reviewed model artifacts, manifests, and evidence reports.

The repository shall pin Rust, Node, npm, `wasm-bindgen-cli`, Playwright, and its Chromium revision.

The model and language artifact identity checks shall use one documented target triple.

The canonical target shall be `x86_64-unknown-linux-gnu` in GitHub Actions.

Other targets shall prove functional native and WASM builds without claiming native binary identity.

An optional macOS job shall compare native bytes on `aarch64-apple-darwin`.

## Current reproduction corrections

Malay shall retain `MS` as its public code and `ID` as its frozen storage code.

Corpus split hashing shall use the storage code.

This rule shall preserve the existing Malay evaluation partition.

The model manifest shall record the regenerated Malay clean-control hash.

The Spanish source shall join the 15-language source lock and prepared manifest.

The Spanish compiler shall train from prepared Spanish data.

It shall not copy the legacy Spanish artifact as the final implementation.

The language model builder shall receive all pinned upstream table inputs from the repository.

## Corpus contribution contract

Each source shall declare one role: `baseline`, `training_only`, or `sealed_evaluation`.

Current mixed sources shall use `baseline` and preserve their frozen partitions.

New community corpora shall default to `training_only`.

Training-only rows shall enter only the development partition.

New training rows shall not move current validation or test rows.

`resources/datasets/evaluation-lock-v1.json` shall store the sealed partition hashes.

The preparation command shall reject a change to any sealed hash.

The lock shall seed its hashes from the accepted 15-language validation and test files.

Sealed baseline rows shall win when duplicate text appears in a new training source.

The preparation command shall exclude an exact duplicate from the new training source.

The preparation command shall reject a duplicate with a conflicting label.

A simple corpus contribution shall use this canonical TSV schema:

```text
native_id\tlabel\ttext
```

The source record shall support `pinned_url` and `repository_file` origins.

The source record shall provide the language, source identity, source role, license, and citation.

The label shall use the canonical values `toxic` or `clean`.

A custom source format shall use a typed Rust adapter and fixture tests.

Any row used to create a rule shall enter `rule-audit-v1.tsv`.

Such a row shall not enter later quality evidence.

`CONTRIBUTING.md` shall describe both contribution paths.

Pull request checks shall not fetch contributor-defined URLs.

Pull request checks shall read only committed raw inputs and pinned dependencies.

## Npm package

The npm package shall live at `packages/blasphem`.

Its `package.json` shall set `name` to `blasphem` and `private` to `true`.

It shall export browser ESM and TypeScript declarations from `dist`.

Consumers shall call `await init()` before creating a detector.

Consumers shall construct `new BlasphemDetector(language)`.

The check method shall return `BlasphemResult`.

Consumers shall call `free()` on both exported classes.

The first npm package shall support browsers only.

Its build script shall compile `blasphem-wasm` with the pinned tool versions.

Its pack check shall inspect the generated npm archive without publishing it.

The package shall exclude raw corpora, training code, reports, and Rust build output.

The default package shall include automatic language detection and all 15 toxicity models.

The package API shall accept an explicit language code or `AUTO`.

Unknown automatic routes shall remain fail-open for the pre-send nudge.

## Data flow

The native CLI shall read one requested HurtLex file from the tracked runtime data directory.

The browser build shall embed all 15 HurtLex files and sparse model artifacts.

The automatic route shall identify the language before the toxicity check.

The training pipeline shall read only development and validation data during compilation.

The test partition shall remain unavailable to model training and threshold selection.

## Error behavior

The reproduction command shall stop after a missing file, hash mismatch, split change, or artifact mismatch.

The corpus parser shall report the source identifier and row identifier for invalid input.

The npm build shall stop after a tool version mismatch.

The runtime shall return a typed unknown route for unreliable automatic language detection.

The product result shall keep `ok=true` for an unknown route.

## Tests

Rename contract tests shall fail while old first-party names remain in active source files.

Malay regression tests shall prove that `MS` keeps the frozen `ID` split identity.

Raw-input tests shall reject one changed byte in any locked source.

Evaluation-lock tests shall reject moved validation or test rows.

Spanish tests shall prove deterministic training from prepared input.

Language artifact tests shall rebuild and compare the committed artifact.

Npm tests shall verify the package name, private flag, exports, and archive contents.

The browser smoke test shall import `dist/blasphem.js` through the pinned Playwright Chromium browser.

Two clean canonical builds shall produce identical model, language, native, and WASM bytes.

Other hosts shall compare model, language, and WASM bytes and run a functional native test.

## Public repository delivery

The initial public history shall contain no secret, local absolute path, task snapshot, or generated build directory.

First-party code shall use the Apache License 2.0.

The repository shall include the Apache License 2.0 text.

Third-party data shall retain its recorded source license and attribution.

The repository shall record unresolved source-license status without claiming permission.

The npm archive shall include the notices for embedded third-party data.

GitHub Actions shall run format, tests, Clippy, reproduction, npm packing, and browser smoke checks.

The first source release shall include `README.md`, `CONTRIBUTING.md`, and the reproduction command.

GitHub shall create `sospedra/blasphem` as a public repository.

The local `main` branch shall push to the `origin` remote.

The npm package shall remain private and unpublished.

## Acceptance criteria

All active first-party names use the Blasphem naming scheme.

A fresh clone builds the native CLI with `cargo build --release --locked --bin blasphem`.

A fresh clone builds and packs the private npm package.

The data-offline reproduction command verifies every corpus and generated artifact.

The full Rust workspace tests pass.

Clippy passes for the full workspace and all targets.

The actual Chrome smoke test passes for explicit and automatic language selection.

The public GitHub repository contains the verified commit on `main`.

No npm publish command runs.
