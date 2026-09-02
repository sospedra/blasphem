# Benchmark

`baseline.json` is the number to beat. Its `validation` section is the committed validation report at `e782855`, before the clean-room lexica, copied verbatim. These are the numbers the web page shows. Its `test` section is the `e782855` binary, built from the committed artifacts with no retrain, judged over the test split.

`runs/<sha>.json` is one measurement per commit. Regenerate after any lexicon, corpus, model, or rule change.

## Run

```sh
cargo run --release --locked -p blasphem-bench -- accuracy
```

The command runs the whole pipeline:

1. `blasphem-train regenerate`: retrains and republishes every model artifact and evidence report from `corpus/` and the lexica.
2. Syncs the artifact digests from the model manifest into `src/embedded.rs`.
3. `cargo build --release --locked --bin blasphem`.
4. Reads `reports/multilingual-validation.json`, the report step 1 wrote. This is the `validation` section, the headline, comparable to the baseline row for row.
5. Judges every `test` row of `corpus/*.tsv` through `blasphem judge --locales XX --no-detect --json`, one process per language. This is the `test` section.
6. Writes `runs/<short-sha>.json` and prints every language against the baseline as
   `XX had R: a% and P: b%, now it has R: c% and P: d%`, validation first.

`--binary <path>` measures a foreign build as is, skipping steps 1 to 3. `--validation-report <path>` reads another report. Together they produced `baseline.json`. `--commit`, `--label`, `--output`, `--baseline` override the defaults.

## Measurement

Toxic rows are positives. A row is a hit when the verdict is not safe. Each language entry has the confusion matrix and `precision`, `recall`, `f1`, `accuracy`, `specificity`. A metric is `null` when its denominator is zero. `pooled` sums the fifteen matrices. `dirty` is true when the working tree had uncommitted tracked changes at measurement time, including artifacts the retrain rewrote.
