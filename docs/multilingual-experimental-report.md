# Multilingual detector experimental report

Date: 2026-09-02

## Verdict

The detector is ready for experimental pre-send nudge tests in all 15 target languages.

The runtime uses deterministic rules, HurtLex, and fixed sparse integer tables. It uses no AI runtime, translation, network call, or Python.

All 30 supplied messages pass through the native CLI and an actual Chrome WASM run.

All 15 languages meet the selected validation limits.

## Current final-path validation

These results use the final shipping path. The boundary selection used these validation rows.

Therefore, this table is calibration evidence. It is not an independent production estimate.

`Clean warned` means the percentage of clean validation messages that received a nudge.

| Language | Rows | Precision | Recall | Clean warned |
| --- | ---: | ---: | ---: | ---: |
| EN | 746 | 100.0% | 59.9% | 0.00% |
| ZH | 754 | 92.9% | 3.5% | 0.26% |
| ES | 762 | 90.1% | 27.6% | 2.75% |
| AR | 750 | 93.5% | 37.4% | 2.74% |
| ID | 1,968 | 94.9% | 41.6% | 2.66% |
| PT | 2,416 | 90.1% | 33.6% | 1.28% |
| FR | 675 | 97.0% | 85.8% | 2.68% |
| HI | 764 | 95.4% | 61.4% | 2.81% |
| RU | 754 | 99.0% | 76.1% | 0.84% |
| JA | 756 | 90.1% | 23.5% | 2.71% |
| DE | 1,971 | 90.1% | 40.0% | 2.71% |
| TR | 4,607 | 92.3% | 2.7% | 0.05% |
| VI | 1,103 | 94.2% | 42.5% | 2.47% |
| KO | 8,726 | 93.0% | 41.5% | 2.48% |
| IT | 742 | 90.7% | 23.8% | 2.42% |

Every language has precision at or above 90 percent. Each clean-warning rate is at or below three percent.

Spanish now meets those gates. Its validation precision is 90.1 percent, and its clean-warning rate is 2.75 percent.

Chinese and Turkish have very low recall. This result follows the selected precision-first policy.

## Behavior evidence

The 15 behavior panels pass 360 of 360 authored contract cases.

The native smoke set passes 60 of 60 cases. It includes the 30 supplied messages and 30 separate context cases.

The final Spanish runtime matches all 88 frozen legacy decisions. The legacy expected-label matrix contains 85 correct decisions and three misses.

The actual Chrome WASM run passes all 30 supplied messages. It also passes the Boolean-score invariants.

The Chrome run rejects an empty language, `AUTO`, `EN-US`, and `XX`.

## Training and calibration data

The prepared source set contains 243,742 rows from 37 locked files. The pipeline excludes 7,882 rows and marks 51 rule-shaping rows audit-only.

The sources include TextDetox, Ibrohim-Budi, ToLD-Br, GermEval 2018, OffensEval-TR, ViHOS, and K-MHaS.

Each language uses one pinned HurtLex 1.2 file. The runtime uses conservative HurtLex entries only.

Development rows fit the sparse weights. Validation rows select the highest-recall boundary that passes the precision limits.

Sixteen frozen clean controls set a minimum boundary for each language. Those controls do not enter the accuracy metrics.

The final work did not reopen any prepared test split. The older test report remains `previously_evaluated` experimental evidence.

## Native performance

The release benchmark ran 270,000 checks across 90 fixed fixtures. Each language has clean, toxic, and dense cases at two lengths.

The benchmark reused an initialized detector. It ran on an Apple M5 Pro with the `aarch64-apple-darwin` target.

| Input set | Fixtures | Median p95 | Worst p95 | Limit |
| --- | ---: | ---: | ---: | ---: |
| 280 Unicode scalars | 45 | 0.181 ms | 0.454 ms | 1 ms |
| 4,096 UTF-8 bytes | 45 | 1.765 ms | 3.514 ms | 10 ms |

All 90 fixtures pass their latency limit. Peak resident memory is 41,566,208 bytes, or 39.6 MiB.

The dense-path fix caches collision decisions and bounds token searches. It preserves the full diagnostic result and the single decision path.

## Size

| Item | Raw | Gzip | Brotli |
| --- | ---: | ---: | ---: |
| Native `toxcheck` binary | 4,018,224 bytes | Not measured | Not measured |
| External HurtLex files | 2,561,810 bytes | Not measured | Not measured |
| Browser WASM core | 5,671,596 bytes | 1,997,203 bytes | 1,626,006 bytes |
| Browser JavaScript glue | 11,287 bytes | 2,888 bytes | 2,595 bytes |

The native binary is 3.83 MiB. The native binary plus external HurtLex files is 6.28 MiB.

The full browser package is 5.42 MiB raw. It is 1.91 MiB with gzip and 1.55 MiB with Brotli.

The 15 sparse tables total 1,966,672 bytes. Every table is about 128 KiB.

## Browser use

The browser caller must select the language. The module does not detect the language.

```js
import init, { WasmDetector } from "./toxcheck_wasm.js";

await init();
const detector = new WasmDetector("AR");
const result = detector.check("أتمنى أن تموت وحيدًا هذه الليلة");

console.log(result.ok, result.score, result.shouldNudge);
result.free();
detector.free();
```

Run `./crates/toxcheck-wasm/verify-browser.sh` to rebuild the module and repeat the actual Chrome check.

## Limits

The score is ordinal from 0 through 100. It is not a probability or a confidence score.

The current release targets native scripts and standard Latin spellings. It has no dedicated Pinyin, Arabizi, or Hinglish model.

The source datasets use different toxicity definitions. Their class balance does not represent live message traffic.

The validation precision can overstate live precision when toxic messages are rare. The clean-warning rate gives the clearer nuisance-warning measure.

The historical test snapshot is in `docs/multilingual-precision-recall-benchmark.md`.
