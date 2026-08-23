# Multilingual precision and recall benchmark

Date: 2026-09-02

Evidence status: `previously_evaluated` experimental evidence.

This historical report does not authorize a new test-data run.

## Verdict

English and Spanish give the best current balance.

Arabic, Hindi, and Russian have high precision with very low recall.

Chinese and Japanese detect no labeled toxic rows in these corpora.

Portuguese and Turkish have precision below 53 percent.

## Results

The product nudge threshold is 50.

`clean warned` is the false-positive rate among clean rows.

| Language | Rows | TP | FP | FN | Precision | Recall | Clean warned |
|---|---:|---:|---:|---:|---:|---:|---:|
| EN | 721 | 75 | 3 | 272 | 96.2% | 21.6% | 0.8% |
| ZH | 741 | 0 | 0 | 361 | N/A | 0.0% | 0.0% |
| ES | 819 | 177 | 14 | 242 | 92.7% | 42.2% | 3.5% |
| AR | 759 | 10 | 1 | 373 | 90.9% | 2.6% | 0.3% |
| ID | 1,988 | 27 | 5 | 1,059 | 84.4% | 2.5% | 0.6% |
| PT | 2,440 | 137 | 125 | 521 | 52.3% | 20.8% | 7.0% |
| FR | 681 | 67 | 9 | 246 | 88.2% | 21.4% | 2.4% |
| HI | 690 | 6 | 0 | 352 | 100.0% | 1.7% | 0.0% |
| RU | 734 | 30 | 3 | 326 | 90.9% | 8.4% | 0.8% |
| JA | 674 | 0 | 0 | 342 | N/A | 0.0% | 0.0% |
| DE | 637 | 15 | 6 | 265 | 71.4% | 5.4% | 1.7% |
| TR | 3,528 | 34 | 35 | 682 | 49.3% | 4.7% | 1.2% |
| VI | 1,102 | 54 | 23 | 475 | 70.1% | 10.2% | 4.0% |
| KO | 21,888 | 5 | 4 | 10,078 | 55.6% | 0.05% | 0.03% |
| IT | 732 | 52 | 20 | 333 | 72.2% | 13.5% | 5.8% |
| Pooled | 38,134 | 689 | 248 | 15,927 | 73.5% | 4.1% | 1.2% |

The pooled result is informational. Korean supplies 57 percent of all rows.

Precision is undefined for Chinese and Japanese. The detector produced no positive predictions for those languages.

## Corpora

TextDetox supplied EN, ZH, ES, AR, FR, HI, RU, JA, DE, and IT rows.

Ibrohim-Budi supplied ID rows. ToLD-Br supplied PT rows. OffensEval supplied TR rows.

ViHOS supplied VI rows. K-MHaS supplied KO rows.

TextDetox, ID, and PT use the deterministic project test split after duplicate and conflict removal.

TR, VI, and KO use their official test splits.

These corpora use different toxicity definitions. Do not compare languages as one controlled ranking.

The labels measure generic toxicity, offense, hate, or harmful spans. They do not measure only pre-send regret.

## Source repairs

The Indonesian source contains invalid UTF-8 sequences. The conversion inserted 953 replacement characters without changing labels.

The Portuguese source mixes `0` with `0.0` and `1` with `1.0`. The conversion normalized only these numeric forms.

The Turkish TSV contains unescaped quotes. The conversion preserved each physical message row and all official labels.

## Run identity

TextDetox revision: `01907546324b0330d2d8b7669648cc18823323e5`

Benchmark TSV SHA-256: `c8c78146cc54570a84f1e6744a5e7ff916c83b1e4a90203c4e69c14da8e07d8d`

Evaluator binary SHA-256: `fccd2a7cdf6e704c0fc3954a1e52aa7d1a1c3f359e52c05b124de431e570fc15`

Spanish sparse table SHA-256: `3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36`

Ordered HurtLex bundle SHA-256: `1bc4914c146b0c624c394a7f7dc4b17a00e7a07774f861605fb5dc521b6c7f97`

The benchmark produced the same confusion matrices in two consecutive runs.

The first run processed 38,134 messages in 3.26 seconds. This time includes detector initialization.

The combined benchmark TSV used temporary storage. The project does not retain the source messages.

The historical run used the legacy `toxtrain eval` path and a temporary benchmark test file.
