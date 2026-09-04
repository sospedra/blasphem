# Blasphem benchmarks

This crate owns the benchmark code and its comparison baseline.
The [benchmark implementation](src/accuracy.rs) defines the measurement path.

## Reports

| File | Purpose |
| --- | --- |
| [baseline.json](baseline.json) | Recorded validation and test baseline |
| `reports/benchmarks/<label>.json` | Generated measurement with its commit and binary digest |
| `reports/multilingual-validation.json` | Generated calibration evidence |

Generated reports stay outside Git.
[CI](https://github.com/sospedra/blasphem/actions/workflows/ci.yml) retains full reports as artifacts for 90 days.
The website reads one committed [snapshot](../../apps/web/src/data/evidence.json).
It retains measurements, report hashes, and playground examples.
Historical source paths identify the original measurements.

Run `pnpm evidence` to generate the full evidence set and a candidate snapshot.
This command downloads the pinned Tatoeba corpus and reads the sealed test split.
The AUTO command verifies corpus hashes before measurement.
CI stores the candidate at `reports/website-snapshot.json`.
Run `pnpm evidence:snapshot` to refresh the committed website snapshot after review.

Each measurement contains separate `validation` and `test` sections.
Blasphem supports [16 languages](../../packages/javascript-packs/README.md#locales).
Recorded reports retain their original evaluation groups and counts.

## Run

Run from the repository root:

```sh
cargo run --release --locked -p blasphem-bench -- accuracy
```

This command changes generated files and reads the sealed test split.
Reserve it for the agreed final evaluation.

The command performs five steps:

1. Regenerate models and evidence reports from the current inputs.
2. Refresh embedded artifact and lexicon digests.
3. Build the release CLI.
4. Read validation evidence and judge test rows through the CLI.
5. Write the run report and print the baseline comparison.

The default output is `reports/benchmarks/<short-sha>.json`.
A repeated label replaces its previous report.
Use a distinct `--label` or `--output` to retain each measurement.

To measure an existing binary without regeneration:

```sh
cargo run --release --locked -p blasphem-bench -- accuracy \
  --binary /path/to/blasphem \
  --validation-report /path/to/validation.json \
  --label candidate
```

Use the validation report from the same build.
This mode still reads test rows and writes a report.

## Options

| Flag | Purpose |
| --- | --- |
| `--binary` | Skip regeneration and build steps |
| `--validation-report` | Select the validation report |
| `--commit` | Record the measured revision |
| `--label` | Name the run and default output file |
| `--output` | Select the report path |
| `--baseline` | Select the comparison report |
| `--project-root` | Select the source checkout |

## Interpret results

Toxic rows are positives.
A verdict with `safe: false` counts as a positive prediction.
Reports include confusion matrices, precision, recall, F1, accuracy, and specificity.
A metric is `null` when its denominator is zero.
The `pooled` matrix sums the model profiles.
The `dirty` field records tracked changes for a measurement of HEAD.

Validation results measure calibration data.
Behavior panels measure authored contracts.
Repeated test inspection does not provide independent evidence.
Do not tune rules, lexica, or thresholds from test results.

## Size

The native CLI budget is 10 MiB (10,485,760 bytes), inclusive.
The binary includes all bundled models, lexicons, and routing data.
Each model artifact must remain below 256 KiB (262,144 bytes).
The size command checks these limits and each resource digest.

## Latency

The dense-message gate runs separately:

```sh
cargo perf-gate
```

See [the gate](tests/dense_runtime_regression.rs) for fixtures and limits.
