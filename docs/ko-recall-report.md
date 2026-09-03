# Korean recall report

## Decision

Freeze `KoreanWordChar25V3` without corpus changes.

The profile keeps the existing character grams. It adds word unigrams.
The added namespace supplies word-boundary evidence without removing phrase signal.

KO now uses class-weighted L2 logistic training. Its selected cost is 0.15.
Validation selected the profile and cost. Treat validation as tuned evidence.

## Audit

The audit covered all 146 validation false positives.
It also covered 200 stratified validation false negatives.
The sample used score deciles and Unicode-scalar length bands.

Each audit row recorded these fields:

- Product score.
- Sparse score.
- Byte and scalar lengths.
- Lexicon score and matched forms.
- The pinned K-MHaS source label.

The K-MHaS validation file had this SHA-256:
`cb0df9e3cd665125b554d4c5e6f48b0801d5535ead51269134de5b85ae469c18`.
The source lock pins revision `ec7a7e775d650b825872f6f538fc717822cdfc1a`.

A deterministic rubric assigned one cause per audited row.
No independent native reviewer was available.
Therefore, no reviewed corpus or lexicon row was added.

| error | cause | rows |
|---|---|---:|
| false negative | context-dependent attack | 56 |
| false negative | identity attack | 68 |
| false negative | slang or abbreviation | 44 |
| false negative | spelling evasion | 32 |
| false positive | decomposed Hangul | 1 |
| false positive | label mismatch | 32 |
| false positive | quote or report | 20 |
| false positive | slang or abbreviation | 25 |
| false positive | spaced phrase collision | 68 |

The false positives included 36 lexicon matches.
The false-negative sample included no lexicon matches.
These results did not support a larger conservative lexicon.

## Feature ablation

The first factorial used the existing log-odds trainer.
It kept lexicon markers active.

| profile | TP | FN | FP | TN | recall | decision |
|---|---:|---:|---:|---:|---:|---|
| `Char25V2` control | 1,691 | 2,160 | 146 | 4,729 | 43.91% | control |
| whitespace boundaries | 1,498 | 2,353 | 75 | 4,800 | 38.90% | reject |
| Jamo grams | 1,473 | 2,378 | 89 | 4,786 | 38.25% | reject |
| boundaries and Jamo | 1,299 | 2,552 | 59 | 4,816 | 33.73% | reject |

Whitespace removal was not the sole problem.
Removing cross-word grams discarded useful Korean phrase evidence.
Broad Jamo decomposition added weak, correlated features.

The second ablation retained the original character grams.

| candidate | TP | FN | FP | TN | recall | decision |
|---|---:|---:|---:|---:|---:|---|
| logistic `Char25V2`, cost 1.0 | 1,711 | 2,140 | 146 | 4,729 | 44.43% | partial |
| isolated marker plus logistic | 1,740 | 2,111 | 146 | 4,729 | 45.18% | partial |
| word unigrams plus character grams | 1,938 | 1,913 | 146 | 4,729 | 50.32% | freeze |

Word bigrams reduced true positives to 1,762.
Word unigrams retained the existing character feature set.

## Cost selection

This grid used `KoreanWordChar25V3`.

| cost | TP | FN | FP | TN | recall |
|---:|---:|---:|---:|---:|---:|
| 0.100 | 1,922 | 1,929 | 140 | 4,735 | 49.91% |
| 0.125 | 1,934 | 1,917 | 141 | 4,734 | 50.22% |
| 0.150 | 1,938 | 1,913 | 146 | 4,729 | 50.32% |
| 0.175 | 1,913 | 1,938 | 145 | 4,730 | 49.68% |
| 0.200 | 1,890 | 1,961 | 146 | 4,729 | 49.08% |
| 0.250 | 1,873 | 1,978 | 146 | 4,729 | 48.64% |

Cost 0.15 produced the most admissible true positives.

## Paired validation changes

The frozen candidate changed 839 validation verdicts.

| change | rows |
|---|---:|
| recovered true positives | 461 |
| lost true positives | 214 |
| fixed false warnings | 82 |
| new false warnings | 82 |

The toxic-row net gain is 247.
The clean-row net change is zero.
The exact paired McNemar value is `p = 9.94e-22`.

## Confidence intervals

Intervals use the Wilson 95% method.

| split | candidate | recall | precision | false-warning rate |
|---|---|---:|---:|---:|
| validation | baseline | 43.91% [42.35%, 45.48%] | 92.05% [90.73%, 93.20%] | 2.99% [2.55%, 3.51%] |
| validation | frozen | 50.32% [48.75%, 51.90%] | 92.99% [91.82%, 94.01%] | 2.99% [2.55%, 3.51%] |
| test | baseline | 45.24% [44.27%, 46.22%] | 91.72% [90.92%, 92.45%] | 3.49% [3.17%, 3.84%] |
| test | frozen | 51.90% [50.92%, 52.87%] | 93.58% [92.91%, 94.19%] | 3.04% [2.75%, 3.37%] |

## Final evidence

The test split stayed sealed during selection.
One final benchmark used the frozen candidate.
It finished at `2026-09-05 00:33:56 CEST`.

```text
validation: TP=1938 FN=1913 FP=146 TN=4729
test:       TP=5233 FN=4850 FP=359 TN=11446
validation gates: 15 languages passed
behavior contract: 360 cases passed
CLI smoke: 60 cases passed
```

The run is `benchmark/runs/0a8d974.json`.
Its binary SHA-256 is
`f7f49b55798098aea5c6c9ab26eedeb0173647be3b8ee4d746a6b51d306fd4df`.

The validation compile command was:

```sh
cargo run --release --locked -p blasphem-train -- compile \
  --corpus-root corpus \
  --source-lock resources/datasets/source-lock-v1.json \
  --hurtlex-root data/clean-room-v1 \
  --behavior-root tests/fixtures/behavior \
  --output "$experiment_root/models"
```

The final benchmark command was:

```sh
cargo run --release --locked -p blasphem-bench -- accuracy
```
