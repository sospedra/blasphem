# Contributing to Blasphem

Contributions can change code, documentation, labeled messages, or lexica.
Keep each pull request focused on one change.

## Code and documentation

Start with [the architecture](HOW.md) and the relevant [distribution guide](README.md#distributions).

| Path | Purpose |
| --- | --- |
| `crates/blasphem/` | Rust runtime, rules, and command-line interface |
| `crates/blasphem-train/` | Corpus checks, lexicon tools, compilation, and reproduction |
| `crates/blasphem-bench/` | Accuracy, latency, and size measurements |
| `crates/blasphem-{ffi,jni,napi,python,wasm}/` | Runtime bindings |
| `packages/` | Language packages, loaders, and packaging scripts |
| `apps/web/` | Website and browser playground |

### Set up

Use the Rust version in [rust-toolchain.toml](rust-toolchain.toml).
Use the runtime versions in [.tool-versions](.tool-versions).
Use the pnpm version in [package.json](package.json).
CI reads these same files.
Any installer can supply these versions. asdf is optional.

Install only the tools needed for your contribution:

| Contribution | Tools |
| --- | --- |
| Rust engine and command-line interface | Rust |
| Website with committed WASM | Node, pnpm. See the [prebuilt build](packages/javascript/README.md#build-without-rust). |
| JavaScript packages from source | Rust, Node, pnpm, wasm-bindgen-cli |
| Go packages | Go. Rust also rebuilds the embedded engine. |
| Python packages | Rust, Python |
| Android package | Rust, Node, JDK, Android SDK/NDK, Gradle wrapper |
| Swift and React Native packages | See their package READMEs for platform tools. |

The pins define the contributor and release build environment.
Go and Python CI also check their minimum and current stable versions.
Minimum requirements stay in `go.mod` and `pyproject.toml`.
JavaScript packages retain their declared Node requirement in `package.json`.

Run these commands from the repository root:

```sh
pnpm install --frozen-lockfile
cargo build --locked -p blasphem
```

JavaScript builds also need the pinned `wasm-bindgen` CLI:

```sh
cargo install wasm-bindgen-cli --version 0.2.127 --locked
pnpm --filter @blasphem/packs run build
pnpm --filter blasphem run build
```

Native packages have additional requirements in their READMEs.

Edit third-party attributions in the root [NOTICE](NOTICE) only.
Package scripts copy it into distribution artifacts.
Generated NOTICE copies are not tracked.

### Dependency versions

Pin registry dependencies to exact versions, including development, build, and peer dependencies.
pnpm saves exact versions through `saveExact: true` in `pnpm-workspace.yaml`.
Cargo dependencies require `=major.minor.patch`.
A plain Cargo version permits compatible updates.
Use `cargo add 'crate@=major.minor.patch'` for new Rust dependencies.
Python dependencies require `package==version`.
Local Cargo paths and `workspace:*` npm references identify packages in this repository.
Keep runtime compatibility requirements, such as `requires-python`, separate from dependency pins.

Keep the lockfiles in version control. Use `pnpm install --frozen-lockfile` and Cargo `--locked`.
Go builds use `-mod=readonly`. Gradle rejects dynamic and changing dependencies, including transitive dependencies.
Pin GitHub Actions to full commit hashes.
Keep generated release action pins in `[dist.github-action-commits]`, then run `dist generate-ci`.

Python build tools and their transitive dependencies live in `packages/python/requirements-build.txt`.
Install that file with `pip install --require-hashes -r packages/python/requirements-build.txt`.
Use `--no-build-isolation` for local package installation and `--no-isolation` with `python -m build`.
To update the Python build tools, edit `requirements-build.in` and the matching `pyproject.toml` pins.
Regenerate the lockfile with:

```sh
uv pip compile --universal --python-version 3.10 --generate-hashes --no-header \
  --output-file packages/python/requirements-build.txt packages/python/requirements-build.in
```

Run `pnpm deps:check` after dependency changes. CI runs the same check.

### Check a change

Run the checks relevant to the changed code:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

Each package README lists its package checks.
Keep examples consistent with the public API.
Include the commands and results in the pull request.
Distinguish source checks from browser or device checks.

### Reproduce artifacts

Install the browser engines before the full reproduction check:

```sh
pnpm --filter blasphem exec playwright install chromium webkit
cargo run --release --locked -p blasphem-train -- reproduce
```

For the Rust and artifact checks without JavaScript:

```sh
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
```

Reproduction verifies the corpus, sealed partitions, model artifacts, and language data.
It builds native and WASM binaries, then runs the configured checks.
It reads local dataset inputs and generates comparison artifacts in a temporary directory.
Package installation can still require network access.
See [the reproduction implementation](crates/blasphem-train/src/reproduce.rs) for the exact checks.

### Update generated artifacts

Changes to training data or rules can require new artifacts:

For lexicon changes, first refresh the [source input digests](resources/lexicon/README.md#verify).

```sh
cargo run --release --locked -p blasphem-train -- regenerate
```

This command rewrites model artifacts, locks, and evidence reports.
Review its diff before submission.
Refresh the artifact and lexicon digests in [embedded.rs](crates/blasphem/src/embedded.rs).
Use the values from [the model manifest](resources/metadata/model-manifest.json).
Rebuild affected packages after the runtime changes.

Use development data, validation reports, and behavior panels during iteration.
Reserve [test-split benchmarks](crates/blasphem-bench/README.md) for the agreed final evaluation.
Never tune rules or thresholds from test results.

## Corpus contributions

Read [the corpus guide](resources/corpus/README.md).
It covers direct edits, labels, escaping, sorting, and sealed partitions.

## Lexicon contributions

Read [the lexicon guide](resources/lexicon/README.md).
It covers categories, sense tables, source records, and the current mirrored files.

## Import an upstream corpus

Direct corpus edits need no adapter.
Use the import pipeline for a new external dataset.

1. Record the source, revision, license, citation, and lineage.
2. Add matching source records to the lock and acquisition observation.
3. Prepare the source files with the sealed evaluation lock.
4. Merge accepted development rows into the corpus.
5. Run the corpus verification command.

The lock is [source-lock-v1.json](crates/blasphem-train/metadata/source-lock-v1.json).
The observation belongs at `crates/blasphem-train/metadata/source-observation-v1.json`.
That observation is historical and does not cover every current locked source.
The repository does not supply the raw upstream corpus files.

Community inputs use `target/raw/community/{language}/{source_file_id}.tsv`.
Each import needs `target/raw/source-observation-v1.json` with the same source records as its reviewed lock.
Their header is `native_id<TAB>label<TAB>text`.
Labels are `clean` or `toxic`.
Set `dataset` to `community` and `source_role` to `training_only`.
See [the community adapter](crates/blasphem-train/src/community_corpus.rs) and
[the source schema](crates/blasphem-train/src/source_manifest.rs).

```sh
cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock crates/blasphem-train/metadata/source-lock-v1.json \
  --raw-root target/raw \
  --audit-exclusions crates/blasphem-train/metadata/behavior-audit-v1.tsv \
  --evaluation-lock crates/blasphem-train/metadata/evaluation-lock-v1.json \
  --output target/prepared-draft
```

Use a new output directory.
Training-only rows enter the development split.
Sealed baseline rows take precedence over duplicates.
Conflicting labels fail preparation.

For a new format, follow the existing [typed adapters](crates/blasphem-train/src/datasets/).
Record rule-derived audit examples in [behavior-audit-v1.tsv](crates/blasphem-train/metadata/behavior-audit-v1.tsv).
Exclude those examples from later quality measurements.

## License

Code contributions use the [Apache-2.0 license](LICENSE).
Dataset contributions retain their recorded source terms.
Record attribution and license information in [NOTICE](NOTICE).
