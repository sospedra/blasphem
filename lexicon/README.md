# Blasphem lexicon

These files define words, categories, and match levels for the rule channel.
They also provide category markers for some model profiles.

## Files

Each model profile uses an uppercase storage code.
Indonesian and Malay share `ID`.

| File | Purpose |
| --- | --- |
| `XX.tsv` | Runtime lexicon |
| `XX.senses.tsv` | Human category and match-level decisions |
| `XX.drops.txt` | Excluded candidate lemmas, one per line |
| `XX.provenance.json` | Harvest digest and assignment records |

The build currently reads [data/clean-room-v1](../data/clean-room-v1/).
This directory contains a separate copy of those lexica.
Changes must keep both copies consistent.
See [the pack build](../packages/packs/scripts/build.mjs) and
[embedded resources](../crates/blasphem/src/embedded.rs).

## Row format

The runtime TSV has six tab-separated columns:

```text
id	pos	category	stereotype	lemma	level
```

| Column | Meaning |
| --- | --- |
| `id` | Storage code and row number, such as `EN00001` |
| `pos` | Part-of-speech code |
| `category` | Category code from the table below |
| `stereotype` | `yes` for identity categories, otherwise `no` |
| `lemma` | Word or phrase |
| `level` | `conservative` or `inclusive` |

The rule channel loads `conservative` entries.
An `inclusive` entry does not activate that channel.
The builder sorts entries by category and lemma, then assigns IDs.

Accepted categories:

```text
ps rci pa ddf ddp dmc is or an asm asf pr om qas cds re svp
```

Identity categories are `ps`, `rci`, `om`, `ddf`, and `ddp`.
See [the TSV parser](../crates/blasphem/src/lexicon.rs) and
[the assignment rules](../crates/blasphem-train/src/lexicon.rs).

## Contribute

1. Select the language and collect evidence for the word's meaning.
2. Record the category and level in `XX.senses.tsv`.
3. Update the runtime rows and their assignment records.
4. Mirror the changed files into `data/clean-room-v1/`.
5. Submit a pull request with source links and example contexts.

Use `XX.drops.txt` for rejected candidate lemmas.
Include the source license and attribution.
Keep related inflected forms consistent.
Use development data and authored examples to assess the change.
Do not use sealed test rows to choose words or categories.

The sense table has this header:

```text
lemma	category	level
```

Do not assume a new word will produce a warning by itself.
The runtime combines lexicon evidence with rules and model scores.
Read [the architecture](../HOW.md#rule-channel) before changing match levels.

## Rebuild from a harvest

The builder reads `XX.harvest.json` from the harvest directory.
It reads sense tables and drop lists from the output directory.
It rewrites the runtime TSV and provenance JSON there.

For an existing English harvest, run from the repository root:

```sh
cargo run --release --locked -p blasphem-train -- lexicon-build \
  --harvest /path/to/harvests \
  --storage-code EN \
  --output lexicon
```

The original harvest JSON files are not committed.
The [lexicon report](../docs/clean-room-lexicon-report.md) records this reproduction limit.
A new harvest uses the network and can contain different upstream data:

```sh
cargo run --release --locked -p blasphem-train -- lexicon-harvest \
  --language-name English \
  --storage-code EN \
  --output /path/to/new-harvests
```

Retain the harvest JSON and review the generated diff.
Do not replace the recorded harvest digest with an unrelated digest.

## Verify

For an English change, compare the runtime copies:

```sh
cmp lexicon/EN.tsv data/clean-room-v1/EN.tsv
shasum -a 256 data/clean-room-v1/EN.tsv
```

Run `lexicon-build` when the matching harvest is available.
It checks sense-table categories, levels, duplicates, and supported sibling constraints.
Update the matching `file_sha256` in [the source lock](../resources/datasets/source-lock-v1.json).
Its clean-room entries retain the legacy `dataset: "hurtlex"` identifier.
The [reproduction check](../crates/blasphem-train/src/reproduce.rs) verifies these input digests before compilation.
Follow [artifact regeneration](../CONTRIBUTING.md#update-generated-artifacts) before shipping changed lexica.
Runtime initialization checks the pinned lexicon and artifact digests.

## License

Source terms are recorded in [NOTICE](../NOTICE).
The lexica draw from Wiktionary and other recorded sources.
Do not describe all language data with one blanket code license.
