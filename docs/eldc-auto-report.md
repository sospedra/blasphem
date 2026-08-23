# ELDC automatic route report

## Verdict

The Rust detector matches the pinned C route counts on 418,882 Tatoeba sentences.

Known supported routes have 99.91% precision.

The default browser build transfers 9,038,194 Brotli bytes.

The explicit-only browser build transfers 1,630,271 Brotli bytes.

These results are experimental evidence.

## Corpus identity

The command used the full pinned `tatoeba_50_v3` ELDC benchmark corpus.

| Field | Value |
| --- | ---: |
| Total rows | 418,882 |
| Supported rows | 147,432 |
| Unsupported rows | 271,450 |
| Text SHA-256 | `8c67c444dec9216991532dee6fdcf4b84843c349fbee218cf70fc6df3d8c5786` |
| Label SHA-256 | `f88ed093f49c0715b75cd6a2d66ad55db936183e35278515925de31c034d8549` |

The text file has no final newline.

The reader rejects unequal parallel file termination.

## Route results

| Metric | Count | Denominator | Rate |
| --- | ---: | ---: | ---: |
| Correct supported route | 144,150 | 147,432 | 97.7739% |
| Supported unknown route | 3,150 | 147,432 | 2.1366% |
| Supported misroute | 132 | 147,432 | 0.0895% |
| Correct known route | 144,150 | 144,282 | 99.9085% |
| Unsupported rejection | 249,593 | 271,450 | 91.9481% |
| Unsupported false route | 21,857 | 271,450 | 8.0519% |

The Rust counts match the pinned C sanity counts exactly.

The AUTO command compared every frozen C field through `eldc::Detector`.

The frozen C parity gate matched 100 of 100 rows.

The score tolerance was `0.000001`.

## End-to-end timing

The first ELDC initialization took 14,765,667 nanoseconds.

The process recorded a 104,284,160-byte peak resident set.

The timed path includes language detection, toxicity pack lookup, and toxicity evaluation.

| Input pool | Samples | p50 | p95 | p99 | Maximum | Checks/s | Bytes/s |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 280 scalars | 215,000 | 174,209 ns | 414,500 ns | 446,167 ns | 1,783,333 ns | 5,010.25 | 2,263,584 |
| 4,096 bytes | 43,000 | 1,675,917 ns | 3,223,875 ns | 3,507,792 ns | 6,276,083 ns | 554.27 | 2,270,296 |

The process checked all 90 fixtures before timing.

AUTO returned unknown for four dense fixtures.

The process rejected `EN-dense-280`, `EN-dense-4096`, `IT-dense-280`, and `IT-dense-4096`.

The process timed the remaining 86 fixtures.

No new AUTO latency gate applies to this first measurement.

## Artifact size

| Artifact | Raw bytes | Gzip bytes | Brotli bytes |
| --- | ---: | ---: | ---: |
| Default WASM | 24,183,756 | 10,469,563 | 9,035,494 |
| Default JavaScript glue | 12,541 | 2,989 | 2,700 |
| Default transferred total | 24,196,297 | 10,472,552 | 9,038,194 |
| Explicit-only WASM | 5,672,999 | 1,997,636 | 1,627,571 |
| Explicit-only JavaScript glue | 12,541 | 2,989 | 2,700 |
| Explicit-only transferred total | 5,685,540 | 2,000,625 | 1,630,271 |

The ELDC artifact has 18,498,380 raw bytes.

Its SHA-256 is `69dd5c22723bbe60073575a67fb94fc1fb8ba60c3ed1ac150ddbef1935dd84da`.

The full native binary has 22,677,568 bytes.

Its SHA-256 is `c0bb1f37fb72a760610d64b986c6a47248182329f5ca120e7db82a684f18efd8`.

The explicit-only WASM dependency tree contains no `eldc` package.

The old 7,340,032-byte native gate does not apply to the full AUTO binary.

The old explicit native size gate remains unchanged.

## Browser result

Chrome 152.0.7977.66 passed 50 runtime contracts.

The default module passed 30 explicit checks, 15 AUTO routes, and four unknown routes.

The explicit-only module passed one English route check.

The explicit-only module rejected `AUTO` with the exact feature error.

The browser confirmed that the two WASM module instances use separate memory.

The browser made zero runtime network requests after both module initializations.

## Packaging boundary

The default build includes ELDC.

The explicit-only build omits ELDC.

Both current WASM builds embed all 15 toxicity packs.

A future product should publish one toxicity pack per language.

The product should load only the selected N packs.

AUTO should detect first and load only the resolved available pack.

A missing pack must return unknown and fail open.

The current shared ELDC table does not shrink in proportion to N.

## Limits

Tatoeba measures sentence language routing.

It does not measure social-message toxicity accuracy.

The corpus does not cover code-switching or romanized chat well.

Unsupported-language rejection is best-effort with this 15-profile model.

Unsupported input can misroute to a supported language.

The unsupported rejection result applies only to the corpus language set.

## Reproduction

Run the full evidence command:

```bash
cargo run --release --locked -p toxbench -- auto \
  --texts /private/tmp/eldc-audit-20260902/benchmark/text_files/tatoeba_50_v3.txt \
  --labels /private/tmp/eldc-audit-20260902/benchmark/text_files/tatoeba_50_v3.languages.txt \
  --fixtures tests/fixtures/benchmark/messages.jsonl \
  --hurtlex-root data/raw-v1/hurtlex \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --native-binary target/task7-native/release/toxcheck \
  --eldc-artifact crates/eldc/data/eldc-15-v1.bin \
  --browser-report reports/multilingual-wasm.json \
  --c-parity-fixture crates/eldc/tests/fixtures/c-parity-v1.jsonl \
  --project-root . \
  --output reports/eldc-auto-validation.json \
  --computer "MacBook Pro (Apple M5 Pro, 48 GB)" \
  --target-triple aarch64-apple-darwin
```

The canonical machine report is `reports/eldc-auto-validation.json`.
