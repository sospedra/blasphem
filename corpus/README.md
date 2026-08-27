# The Blasphem corpus

One file per language holds every labeled row for that language. These files are
the single source of truth. No second copy of any corpus text exists in this
repository.

`corpus verify` is the gate. Run it before you open a pull request:

```bash
cargo run --release --locked -p blasphem-train -- corpus-verify \
  --corpus-root corpus \
  --evaluation-lock resources/datasets/evaluation-lock-v1.json
```

## Columns

Each file is a tab separated table with a header and three columns.

| Column | Meaning |
| --- | --- |
| `split` | `development`, `validation`, or `test`. |
| `label` | `clean` or `toxic`. |
| `text` | The message. |

The text is the row's identity. No two rows in one file may carry the same
text. Provenance is per file, not per row: `resources/datasets/source-lock-v1.json`
names the upstream sources of each language, and `NOTICE` holds the full record.

## Escape rule

The text column carries no raw tab, newline, or carriage return. Write `\t`,
`\n`, and `\r` instead, and write a literal backslash as `\\`. Every other byte
is literal. The rule keeps one row on one line, so a diff stays readable.

## Sort rule

Rows are sorted by the whole line, as a byte comparison. A new row goes in its
sorted position, not at the end of the file.

## Sealed partitions

`validation` and `test` rows are sealed. `resources/datasets/evaluation-lock-v1.json`
records one SHA-256 per language per sealed split, over the label and the
escaped text of each row in file order.

Do not edit a sealed row. `corpus verify` fails and names the language.

Add new rows with `split` set to `development`. That is the only split a
contribution may extend.

## Licenses

Every file inherits the license of the upstream sources it carries. `NOTICE`
holds the full record, including the citation and the license year.

| File | Rows | Upstream license |
| --- | ---: | --- |
| `AR.tsv` | 4,987 | CC-BY-4.0 |
| `DE.tsv` | 12,714 | CC-BY-4.0 and NOASSERTION |
| `EN.tsv` | 4,891 | CC-BY-4.0 |
| `ES.tsv` | 4,995 | CC-BY-4.0 |
| `FR.tsv` | 4,746 | CC-BY-4.0 |
| `HI.tsv` | 4,953 | CC-BY-4.0 |
| `ID.tsv` | 12,946 | CC-BY-4.0 |
| `IT.tsv` | 4,953 | CC-BY-4.0 |
| `JA.tsv` | 4,812 | CC-BY-4.0 |
| `KO.tsv` | 108,663 | CC-BY-SA-4.0 |
| `PT.tsv` | 16,045 | CC-BY-4.0 |
| `RU.tsv` | 4,975 | CC-BY-4.0 |
| `TR.tsv` | 35,238 | CC-BY-4.0 |
| `VI.tsv` | 10,941 | CC-BY-4.0 |
| `ZH.tsv` | 4,996 | CC-BY-4.0 |

`KO.tsv` carries K-MHaS under CC-BY-SA-4.0. Commercial use is permitted. The
share-alike term applies to any redistribution, including the Korean model
artifact.

`DE.tsv` carries GermEval 2018 rows whose upstream license is unresolved. That
record claims no permission.

The corpus never ships in a published package.
