# Blasphem corpus

These TSV files contain the labeled messages for offline training and evaluation.
The compiler reads this directory directly.
Published runtime packages contain compiled tables, without corpus text.

## Files and format

Each model profile has one file, such as [EN.tsv](EN.tsv).
See [all 16 supported input languages](../../packages/javascript-packs/README.md#locales).

| Column | Accepted values |
| --- | --- |
| `split` | `development`, `validation`, `test` |
| `label` | `clean`, `toxic` |
| `text` | Escaped message text |

The exact header uses tabs:

```text
split	label	text
```

Escape tabs as `\t`, newlines as `\n`, and carriage returns as `\r`.
Escape a literal backslash as `\\`.
Keep every message on one physical line.

Sort data rows by the complete escaped line, using byte order.
Keep the header first.
Each file must contain unique text and unique normalized text.

The parser and validator are in [corpus.rs](../../crates/blasphem-train/src/corpus.rs).

## Contribute

1. Select the language file.
2. Add or correct a `development` row.
3. Place the row in byte-sorted order.
4. Run the verification command below.
5. Submit a pull request with the label rationale and source attribution.

Use `toxic` for an example of abuse.
Include clean examples for ambiguous words and benign contexts.
For source imports, follow [the import guide](../../CONTRIBUTING.md#import-an-upstream-corpus).

Do not change validation or test rows.
The [evaluation lock](../../crates/blasphem-train/metadata/evaluation-lock-v1.json) seals both partitions.
Do not change its digests to accept a contribution.

## Verify

Run from the repository root:

```sh
cargo run --release --locked -p blasphem-train -- corpus-verify \
  --corpus-root resources/corpus \
  --evaluation-lock crates/blasphem-train/metadata/evaluation-lock-v1.json
```

The command checks columns, escapes, ordering, duplicates, and sealed digests.
It downloads no data.

## Training and evaluation

Development rows train the tables.
Validation rows calibrate the decision boundaries.
Test rows provide final evaluation evidence.

A row used to design a rule becomes audit-only.
Keep it out of later quality measurements.
Read [the development guide](../../CONTRIBUTING.md#update-generated-artifacts) before regenerating artifacts.

## Sources and licenses

The [source lock](../../crates/blasphem-train/metadata/source-lock-v1.json) records upstream provenance.
[NOTICE](../../NOTICE) records the source licenses and unresolved terms.
Korean data includes K-MHaS.
German data includes GermEval 2018 with an unresolved license record.
