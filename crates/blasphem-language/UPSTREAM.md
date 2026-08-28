# Upstream ELDC

This crate ports selected behavior and generated data from [nitotm/eldc](https://github.com/nitotm/eldc).

The pinned upstream commit is `a0301db809ff2e48a418018aa5359fb0c4354eb8`.

The upstream project uses the Apache License 2.0. The upstream author is Nito.

The importer accepts only files with these SHA-256 digests.

| File | SHA-256 |
| --- | --- |
| `large_db.h` | `4f9f3d9741e5f594b0a50da9bf1d26cfba2b8f049a1b75627114a6cc9c0dfe64` |
| `eld_unicode_bits.h` | `e620b9feb08eb32ce751a7148a51b19c5eb2774d2dff74f5dd2d1363184df23b` |
| `eld_tolower.h` | `97722a4d9765e609631ce527ff42b27a4e589d7e673d17e8bf1da68068da1d2b` |
| `eld_unicode.h` | `26b6b645823f81796dcdafdf8eedb41299d769d8c06579eab9ec4ffa3e519cf0` |

The repository vendors these four files under
`crates/blasphem-language/vendor/a0301db809ff2e48a418018aa5359fb0c4354eb8`.
The build reads them from that directory. It downloads nothing.

The generated artifact contains only these upstream language indexes.

`1 9 11 12 17 20 25 26 29 36 42 44 54 57 59`

The compact order is `AR DE EN ES FR HI IT JA KO MS PT RU TR VI ZH`.

## Artifact layout

The artifact is format version 2. The 76-byte header holds the magic `BLASPHEM`, the version, the language count, the table length, the blob length, the three unicode table lengths, and the upstream commit. The 15 averages, the letter bits, the CJK bits, and the lowercase table follow.

The upstream hash table has 2,097,152 positions. Version 2 stores two bitmaps over those positions instead of the table. The occupied bitmap marks every position the upstream table filled. The live bitmap marks the positions whose scores survive the 15-language filter. The live slots follow in position order as fingerprint and metadata pairs. The score blob comes last.

A dead slot is occupied and not live. Every score it held belonged to a dropped language. It stays occupied so each probe chain ends where the upstream table ended it, and it stores nothing else. The reader derives the rank of every live slot from the live bitmap and reads scores only from live slots. Detection output is identical to the version 1 table. Version 1 was 18,498,380 bytes. Version 2 is 5,048,468 bytes.
