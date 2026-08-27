# Blasphem corpus single-source design

## Status

The user approved this design in chat on 2026-09-03.

The user chose the fold variant over a rebuild variant on 2026-09-03.

Fold keeps the source catalog, the source lock, the evaluation lock, `source_role`, and the
community adapter that landed in commits `8cb14d8` through `6131ce6`. It changes only the corpus
layer.

## Goals

The repository shall host one normalized corpus file per supported language.

Each corpus file shall be the single source of truth for its rows.

No second copy of any corpus text shall exist in the repository.

Every row shall carry attribution to its origin.

Contributors shall add rows and correct labels through pull requests against the corpus files.

The corpus shall never ship in a published package.

## Decisions taken

The corpus shall be a full re-hosted merge, not an overlay on external files.

One file shall hold all rows for one language, across all splits.

The corpus files shall be hand-edited. They shall not be regenerated from raw inputs.

The corpus format shall be TSV.

Attribution shall record the license year that the upstream record states.

The existing metadata files shall stay. Metadata is not a second copy of the text.

## What already exists

`resources/datasets/source-catalog-v1.json` and `source-lock-v1.json` hold 38 source records, each
carrying `source_role` since `8cb14d8`.

`resources/datasets/evaluation-lock-v1.json` seals a `validation_sha256` and a `test_sha256` per
language, keyed by storage code.

`crates/blasphem-train/src/community_corpus.rs` accepts a three-column community TSV and routes it
to the development partition.

`NOTICE` is generated from the source lock. `CONTRIBUTING.md` documents the community TSV path.

`crates/blasphem-train/src/reproduce.rs` runs nine ordered steps, named in `STEP_NAMES`.

## Layout

The corpus shall live at `corpus/` in the repository root.

`corpus/` shall not be an npm workspace package, because it never publishes.

The directory shall contain one TSV file per language and one `README.md`.

File names shall use the frozen storage code, so Malay rows live in `corpus/ID.tsv`.

`corpus/` shall replace the gitignored `data/prepared-v1` output as the compiler input.

There shall be no `corpus/sources.json`. The source catalog and the source lock keep that role.

## Row schema

Each corpus file shall begin with one header line.

The columns shall be:

```text
row_id	source	split	label	origin_label	text
```

`row_id` shall be stable and shall never be reused.

An imported row shall use `<source_file_id>:<native_id>`, for example `kmhas-train:41822`.

A first-party row shall use `blasphem:<first ten hex characters of the SHA-256 of the normalized
text>`.

`source` shall be a `source_file_id` present in `resources/datasets/source-lock-v1.json`.

The `source` key shall be the only per-row attribution. License and citation strings shall not
repeat per row.

`split` shall be `development`, `validation`, or `test`.

`label` shall be `toxic` or `clean`.

`origin_label` shall hold the upstream label as imported.

`origin_label` shall be empty when the label is unchanged and when the row is first-party.

`text` shall be one line. A tab shall be written `\t`. A newline shall be written `\n`.

## Sort order

Each corpus file shall stay sorted by `row_id` after the header.

Content-derived first-party identifiers scatter new rows across the file.

Two concurrent contributions therefore land at different offsets and merge without conflict.

The sort also makes a duplicate text impossible to add twice.

## Sealed evaluation partitions

`evaluation-lock-v1.json` shall keep its schema and its per-language `validation_sha256` and
`test_sha256`.

The digest input shall change. Today each digest covers one file. It shall instead cover the rows
of that split, extracted from the merged language file in `row_id` order.

`corpus verify` shall recompute both digests and shall reject any change.

The development partition shall grow freely.

## License year

`FrozenSource` and `SourceRequest` shall gain a `license_year` field.

`license_year` shall be the year the upstream record states.

`license_year` shall not be the acquisition date and shall not be the year we read the record.

When an upstream record states no year, `license_year` shall be the year of the pinned revision
commit.

`NOTICE` shall render the year in each section.

## Reproducibility hole to close

`freeze-sources` cannot regenerate the committed lock. It aborts because `textdetox-es` carries no
Parquet download digest.

The fix shall record that digest, so the lock can be regenerated from its inputs.

## Commands

`blasphem-train corpus verify` shall run offline and shall be the pull-request gate.

`corpus verify` shall check the header, the column count, the sort order, duplicate identifiers,
duplicate text under `blasphem::normalize_text`, label values, split values, escape correctness,
unknown source keys, and the sealed digests.

`compile` and `evaluate` shall read `corpus/*.tsv` instead of `data/prepared-v1`.

`prepare` shall remain, and shall serve the one-time migration and future source imports. The
reproduction path shall no longer call it.

The eight typed adapters under `crates/blasphem-train/src/datasets/` shall remain.

`import_all_rows` shall keep dispatching the community adapter. That dispatch was missing before
`ba389ca` and silently dropped every community row.

## Reproduce

`STEP_NAMES` shall drop from nine steps to eight.

`verify-raw-inputs` and `generate-prepared-data` shall be replaced by one `verify-corpus` step.

`verify-corpus` shall check the corpus files, and shall check the HurtLex raw inputs against the
source lock.

`verify-sealed-partitions` shall read the merged corpus files.

`compile-model-artifacts` shall read the merged corpus files.

The remaining five steps shall not change.

`GENERATION_STEPS` shall fall from 5 to 4.

## Deletions

`data/raw-v1/datasets/**` shall be removed. That is 21 MB.

`data/raw-v1/textdetox/**` shall be removed. That is 11 MB.

`data/source-observation-v1/**` shall be removed, because `data/raw-v1/source-observation-v1.json`
holds the same records.

The `/data/prepared-v1/` and `/data/prepared-draft-v1/` entries shall be removed from `.gitignore`,
and both directories shall be removed.

`data/raw-v1/hurtlex/**` shall stay. It is the canonical HurtLex path.

`crates/blasphem-wasm/src/lib.rs:249-263` embeds those 15 files with `include_bytes!`. Four bench
and runtime test files read the same path. No HurtLex path shall move.

`data/hurtlex/` is a second tracked copy of the same 15 files, written by `setup`. Removing it is
out of scope for this design and shall be raised separately.

## Attribution

`NOTICE` shall stay generated, never hand-edited.

Each section shall state the license, the license URL, the license year, the citation, the source
count, and the row count in the corpus.

`corpus/README.md` shall state the license of each corpus file.

A corpus file's license shall be the strictest license among its contributing sources.

Share-alike stays inside the repository, because the corpus never ships.

## Contribution

A contributor shall add a row with `source` set to `blasphem` and `split` set to `development`.

A contributor shall correct a label by editing `label` and writing the previous value into
`origin_label`.

A contributor shall not edit a validation or test row.

Continuous integration shall run `corpus verify` and shall fetch nothing.

`CONTRIBUTING.md` shall be updated. Its current three-column community TSV path describes a file
handed to `community_corpus.rs`, not a direct edit of `corpus/`.

## Turborepo

The repository has no `package.json` and no `turbo.json` today.

When turbo arrives, `corpus verify` shall run as a turbo root task, `//#corpus-verify`, with
`inputs` set to `corpus/**`.

Turbo shall not build the Cargo workspace.

Nothing in this design depends on turbo landing.

## Migration

`prepare` shall run once to produce the merged corpus files.

The merged files, the deletions, the retargeted reproduce steps, and the reseeded evaluation lock
shall land together.

The reseeded digests shall equal the current sealed digests over the same rows. A difference means
the migration changed a sealed row and shall block the change.

## Measured baseline

Measured on 2026-09-03.

| Fact | Check | Value |
| --- | --- | --- |
| Raw corpus rows | `wc -l` over `data/raw-v1/datasets/*/*` and `data/raw-v1/textdetox/*.tsv` | 262594 |
| Raw corpus size | `du -sh` | 32 MB |
| Largest merged file | `corpus/KO.tsv` | about 110000 rows, about 9 MB |
| Second largest | `corpus/TR.tsv` | about 35000 rows |
| Smallest | `corpus/ZH.tsv` | 620 KB |
| Commits ahead of `origin/main` | `git log --oneline @{u}..HEAD` | 28 |

## Rejected alternatives

The rebuild variant replaced the catalog, the lock, and the evaluation lock with a new
`corpus/sources.json` and a new `corpus-lock-v1.json`. It was rejected because it discards working
reviewed code to gain tidier metadata, and the metadata holds no corpus text.

JSONL removes the custom escape rule but adds about 60 bytes of repeated keys per row.

CSV quoting turns every hand edit into a hazard, because corpus text carries commas and quotes.

Parquet defeats git diff and pull-request review.

Sharding a language across split files was rejected. A small diff renders inside a large file, and
a per-split Korean file is still about 2 MB, so sharding buys no browsability.

An overlay of label deltas over untouched upstream files was rejected. It creates four places to
look instead of one.

## Out of scope

The two license values for `germeval-2018` and `k-mhas` are open with the user in another session.
This design records the `license_year` field. It does not decide either value.

Task 18 of `docs/superpowers/plans/2026-09-02-blasphem-public-package-and-corpus.md` pushes the
repository to GitHub. It shall run after this design lands.
