# Contributing to Blasphem

This document covers two ways to add training data: a simple community
submission and a custom typed adapter for a new upstream source.

## Licensing

By submitting a contribution, you agree it can be distributed under this
project's terms. See `LICENSE` and `NOTICE`.

## The simple path: a community TSV

Add rows without writing code.

1. Create `data/raw-v1/community/{language}/{source_file_id}.tsv` using the
   canonical schema: three tab-separated columns, `native_id`, `label`,
   `text`, with that exact header row.
2. The `label` column holds `toxic` or `clean`.
3. Add a source record to **two** files: `resources/datasets/source-lock-v1.json`
   and `data/raw-v1/source-observation-v1.json`. `validate_observation_matches_lock`
   in `crates/blasphem-train/src/acquisition.rs` rejects either file alone.

   Each record needs these fields: `dataset`, `detector_language`,
   `source_role`, `source_file_id`, `immutable_source_url`, `file_path`
   (the path under `data/raw-v1`), `file_sha256`, `license_id`,
   `license_url`, `citation`, `upstream_lineage`, and `lineage_status`.
   The observation record also needs `acquired_at_unix_seconds`; the lock
   record does not.

   A community contribution uses `"dataset": "community"` and
   `"source_role": "training_only"`. That path is wired and working; see
   `crates/blasphem-train/src/community_corpus.rs`.

   Worked example, the existing `textdetox-en` entry in
   `resources/datasets/source-lock-v1.json`:

   ```json
   {
     "dataset": "textdetox",
     "detector_language": "EN",
     "source_role": "baseline",
     "source_file_id": "textdetox-en",
     "immutable_source_url": "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/01907546324b0330d2d8b7669648cc18823323e5/data/en-00000-of-00001.parquet",
     "file_path": "textdetox/en.tsv",
     "file_sha256": "9d17c991b87c4b43ea5f69c9950f3ad852c26a0a7b1aa4a5849323a1ae738988",
     "license_id": "CC-BY-4.0",
     "license_url": "https://creativecommons.org/licenses/by/4.0/",
     "citation": "TextDetox multilingual toxicity dataset",
     "upstream_lineage": [
       "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset"
     ],
     "lineage_status": "resolved"
   }
   ```

   `crates/blasphem-train/src/publication.rs` rejects a source whose
   `upstream_lineage` is empty. The error reads "prepared provenance has
   incomplete source metadata".
4. Run `prepare` with the evaluation lock:

   ```bash
   cargo run --release --locked -p blasphem-train -- prepare \
     --source-lock resources/datasets/source-lock-v1.json \
     --raw-root data/raw-v1 \
     --evaluation-lock resources/datasets/evaluation-lock-v1.json \
     --output data/prepared-draft-v1
   ```

5. Open a pull request.

## The custom path: a typed adapter

Add a new upstream source with its own parser.

1. Add a typed adapter under `crates/blasphem-train/src/datasets/`, modeled
   on the existing adapters in that directory (for example
   `germ_eval_2018.rs` or `vihos.rs`).
2. Add fixtures under `crates/blasphem-train/tests/fixtures/`.
3. Add adapter tests that exercise the new fixtures.

## Rules the pipeline enforces

- The canonical schema is `native_id`, `label`, `text`, tab separated, with
  that header.
- The label is `toxic` or `clean`.
- A new source declares `source_role` `training_only`.
- Training-only rows enter only the development partition.
- A sealed baseline row wins a duplicate. The pipeline excludes the new
  copy.
- A duplicate with a conflicting label fails preparation.
- A row used to create a rule goes into
  `resources/datasets/rule-audit-v1.tsv` and never into later quality
  evidence.
- Pull request checks read only committed inputs. They fetch no
  contributor URL.
