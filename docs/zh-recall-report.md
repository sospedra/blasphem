# Chinese recall report

## Decision

Retain `ChineseScriptChar15V3` as the current control.
Do not accept it as the 50% recall candidate.

The profile adds Han unigrams. It separates Han and Latin grams.
The combined sparse change produced the strongest shipped result.
No non-leaky experiment reached 50% validation recall.

## Audit

The baseline had 359 false negatives and one false positive.
The false-negative score bands were:

| model score | false negatives |
|---:|---:|
| 0-9 | 93 |
| 10-19 | 110 |
| 20-29 | 91 |
| 30-39 | 41 |
| 40-49 | 24 |

The conservative lexicon matched 17 of 372 toxic rows.
It matched four of 382 clean rows.
No validation true positive had a lexicon match.

The only baseline false positive was `corpus/ZH.tsv:4584`.
It had model score 58 and no lexicon match.

A stratified sample covered 200 false negatives.
It included all 17 false negatives with lexicon matches.
The sample showed attacks, reports, counterspeech, and disputed labels.
No new native reviewers were available.
Therefore, no new hand-labelled row entered the corpus.

## Validation ablations

Each ablation used the sealed validation split.
The test split stayed sealed during selection.

| candidate | TP | FN | FP | TN | decision |
|---|---:|---:|---:|---:|---|
| `Char25V2` control | 13 | 359 | 1 | 381 | reject |
| shared character unigrams | 13 | 359 | 1 | 381 | reject |
| script namespaces, character 2-5 grams | 16 | 356 | 1 | 381 | partial |
| script namespaces, Han unigrams, character 2-5 grams | 27 | 345 | 3 | 379 | freeze |
| frozen profile plus lexicon markers | 14 | 358 | 1 | 381 | reject |

The frozen profile promoted 19 toxic rows.
It demoted five toxic rows.
The exact McNemar test gives `p = 0.00661`.

## 50% target audit

The target requires 186 true positives from 372 toxic rows.
The false-warning gate permits 11 false positives.
That gate is stricter than the precision gate at the target.

All new selection runs used validation only.
The test split stayed sealed.

| candidate | TP | FN | FP | TN | recall | precision |
|---|---:|---:|---:|---:|---:|---:|
| current sparse control | 27 | 345 | 3 | 379 | 7.26% | 90.00% |
| uncompressed character TF-IDF | 38 | 334 | 4 | 378 | 10.22% | 90.48% |
| COLD rows, linear model, local windows | 58 | 314 | 6 | 376 | 15.59% | 90.62% |
| ZHateBench pairs and linear model | 60 | 312 | 6 | 376 | 16.13% | 90.91% |
| COLD RoBERTa without adaptation | 49 | 323 | 4 | 378 | 13.17% | 92.45% |
| COLD RoBERTa adapted on development | 127 | 245 | 11 | 371 | 34.14% | 92.03% |
| BGE-small embeddings and RBF SVM | 108 | 264 | 11 | 371 | 29.03% | 90.76% |
| independent SWSR XLM-R classifier | 20 | 352 | 2 | 380 | 5.38% | 90.91% |

The contextual experiment is a 409 MB research model.
It cannot fit the current 131 KB sparse artifact contract.
Its result remains 59 true positives below the target.

The surface audit found 58 toxic-labelled questions.
It found 53 report or counterspeech markers.
It found 176 negation markers.
These overlaps need native review before label changes.

The source lock marks TextDetox Chinese lineage unresolved.
No Chinese native-review evidence exists in the repository.
Therefore, adding more unreviewed rows cannot resolve the label contract.

The strongest next path changes the architecture.
It needs a contextual Chinese cross-encoder.
It also needs domain-matched, native-reviewed minimal pairs.
Fusion training must use development folds only.

## Data ablations

[COLDataset](https://github.com/thu-coai/COLDataset) was pinned at
`a0c55a945497ab58e94aef491180e1cf88ffb864`.
Its 25,618 unique training rows reduced true positives to six.

[ToxiRewriteCN](https://github.com/PostMindLab/ToxiRewriteCN) was pinned at
`d42f2412d159502c7e1ef3d00700affa7fb3509b`.
Its 1,439 non-colliding pairs reduced true positives to 22.
A targeted 56-pair batch reduced true positives to 21.

Both repositories declare Apache-2.0 licenses.
All added rows passed `corpus-verify` in temporary roots.
No rejected row entered `corpus/ZH.tsv`.

Lexicon markers reduced true positives from 27 to 14.
The V2 lexical rule gives 30 points.
That score cannot reach the 50-point verdict threshold.
Therefore, this change does not expand the lexicon.

## Confidence intervals

Intervals use the Wilson 95% method.

| split | candidate | recall | precision | false-warning rate |
|---|---|---:|---:|---:|
| validation | baseline | 3.49% [2.05%, 5.89%] | 92.86% [68.53%, 98.73%] | 0.26% [0.05%, 1.47%] |
| validation | frozen | 7.26% [5.04%, 10.35%] | 90.00% [74.38%, 96.54%] | 0.79% [0.27%, 2.28%] |
| validation | contextual upper bound | 34.14% [29.51%, 39.10%] | 92.03% [86.29%, 95.49%] | 2.88% [1.62%, 5.08%] |
| test | baseline | 4.71% [2.96%, 7.41%] | 94.44% [74.24%, 99.01%] | 0.26% [0.05%, 1.48%] |
| test | frozen | 8.31% [5.88%, 11.62%] | 96.77% [83.81%, 99.43%] | 0.26% [0.05%, 1.48%] |

## Prior final evidence

The final benchmark used the frozen candidate once.
It wrote [`benchmark/runs/0a8d974.json`](../benchmark/runs/0a8d974.json).

```text
validation: TP=27 FN=345 FP=3 TN=379
test:       TP=30 FN=331 FP=1 TN=379
validation gates: 15 languages passed
behavior contract: 360 cases passed
CLI smoke: 60 cases passed
```

The Chinese artifact SHA-256 is
`605d1a49534b84e498e8978a1e61eda1ba489c0de4ac9904cce4c571a1e9060f`.

The benchmark command was:

```sh
cargo run --release --locked -p blasphem-bench -- accuracy
```
