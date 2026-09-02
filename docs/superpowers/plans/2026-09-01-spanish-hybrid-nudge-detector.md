# Spanish hybrid nudge detector implementation plan

> Required sub-skill: `superpowers:test-driven-development`

**Goal:** Build a testable Spanish nudge detector with deterministic Rust inference and no AI runtime.

**Architecture:** Keep the current policy engine. Add generic semantic frames and an embedded sparse Spanish scorer. Merge both channels through one `NudgeResult`.

**Tech stack:** Rust 2024, Charabia, Aho-Corasick, CSV, fixed binary model data, and the TextDetox Spanish split.

**Spec:** `docs/superpowers/specs/2026-09-01-spanish-hybrid-nudge-detector-design.md`

**Constraints:** Preserve the 15-language contract. Train offline. Never tune on test rows. Keep runtime network-free. Keep the installed proof below 6 MB.

## Task 1: Add the Spanish audit panel

**Files:**

- Create: `samples/spanish-audit.tsv`
- Modify: `tests/policy.rs`

1. Add literal Spanish toxic and clean rows to the audit TSV.
2. Add table-driven tests for the current misses and related grammatical forms.
3. Run the focused tests.
4. Confirm each new behavior test fails for the expected missing semantic event.

## Task 2: Add the nudge result contract

**Files:**

- Modify: `src/policy.rs`
- Modify: `src/lib.rs`
- Modify: `src/main.rs`
- Modify: `tests/policy.rs`
- Modify: `tests/cli.rs`

1. Add failing tests for the score, threshold, and Boolean invariant.
2. Run the focused tests and confirm the expected compile or assertion failure.
3. Add `NudgeResult` and the `PolicyResult::nudge` method.
4. Print `ok`, `score`, `threshold`, and `should_nudge` on the CLI first line.
5. Run the focused tests and confirm they pass.

## Task 3: Add generic semantic event frames

**Files:**

- Modify: `src/rule_pack.rs`
- Modify: `src/policy.rs`
- Modify: `tests/policy.rs`

1. Add failing tests for hostile wishes, self-harm commands, and implicit second-person abuse.
2. Run the focused tests and confirm the expected failures.
3. Add shared cue classes to `RulePack`.
4. Add one generic semantic event scanner to the policy engine.
5. Apply the existing suppression rules to each semantic event.
6. Run the focused tests and confirm they pass.

## Task 4: Add the compiled sparse scorer

**Files:**

- Create: `src/sparse.rs`
- Create: `src/bin/compile_sparse.rs`
- Create: `tests/sparse.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

1. Add failing parser, feature, score, and malformed-artifact tests.
2. Run `cargo test --test sparse` and confirm the expected compile failure.
3. Implement the shared deterministic feature extractor.
4. Implement the validated compiled-artifact parser and scorer.
5. Implement the offline Bernoulli log-odds compiler.
6. Run `cargo test --test sparse` and confirm it passes.

## Task 5: Train and integrate the Spanish model

**Files:**

- Create: `data/textdetox/es-source.tsv`
- Create: `data/textdetox/es-prepared/`
- Create: `resources/models/es-chargram-v1.bin`
- Modify: `src/detector.rs`
- Modify: `src/policy.rs`
- Modify: `tests/policy.rs`
- Modify: `tests/cli.rs`

1. Download all 5,000 Spanish TextDetox rows with the existing acquisition command.
2. Prepare the deterministic development, validation, and test splits.
3. Compile weights from development rows.
4. Select the boundary from validation rows under the three-percent false-positive limit.
5. Add a failing integration test for the embedded Spanish model.
6. Embed the model and combine its score with the rule score.
7. Run the focused tests and confirm they pass.

## Task 6: Verify quality, size, and documentation

**Files:**

- Modify: `README.md`
- Create: `docs/spanish-proof-report.md`

1. Freeze the implementation before reading test metrics.
2. Evaluate the untouched TextDetox test split.
3. Evaluate the independent Spanish audit panel separately.
4. Run the full test suite.
5. Run Rustfmt and Clippy with warnings denied.
6. Build the release binary.
7. Measure the binary, model, and Spanish lexicon sizes.
8. Record exact commands, metrics, limits, and sample outputs.
