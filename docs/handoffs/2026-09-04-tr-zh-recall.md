# Handoff: Turkish and Chinese recall

Turkish catches 13% of toxic test messages. Chinese catches 5%. Every other language is above 25%. Numbers from `benchmark/runs/d442452.json`, test split, after the lexicon-marker change.

| lang | rows | dev toxic : clean | test recall | test precision | validation TP / FP |
|---|---:|---|---:|---:|---|
| TR | 35,238 | 5,220 : 21,883 | 13.3% | 84.1% | 116 / 12 |
| ZH | 4,996 | 1,763 : 1,738 | 5.0% | 94.7% | 13 / 1 |

## What we know

Turkish is not short of rows. It has the second largest corpus, from OffensEval-TR and the community set. The model still separates poorly. At the chosen boundary it catches 116 of 904 validation toxic rows. Precision sits at the 90% gate, so calibration cannot lower the boundary. Two suspects, both unverified. First, label definition: OffensEval marks "offensive", the corpus means "toxic". Second, `TurkishV2` normalization with word features on an agglutinative language.

Chinese is short of rows and short of signal. 3,501 development rows. The model is `Char25V2`, character 2- to 5-grams. The word-boundary bug in `src/detector.rs` is fixed. Han, kana, and Hangul neighbours now count as boundaries. Lexicon hits fire. Coverage is the limit. Lexicon hits appear on 5% of Chinese toxic validation rows. Japanese 22%, Korean 38%. With markers on, the boundary rose above toxic rows with no lexicon word. Recall fell. So ZH, JA, and KO run without markers (`uses_lexicon_features`). Korean has one more blocker. Clean control ko-c12 carries two lexicon words. With markers on it pins the boundary.

## Steps

1. Turkish: read 50 false negatives from the validation split. Decide whether they are toxic under our definition. Command: `blasphem check --language TR --text=... --data-dir data/clean-room-v1` prints the model score.
2. Turkish: if labels are sound, try the character-only profile `Char25V2` for TR in `Language::profiles` (`src/language.rs`) and recompile. One benchmark run answers it.
3. Chinese: grow the lexicon until hits cover most toxic rows. Then turn the markers on for ZH and rerun. Same for JA and KO. KO also needs a decision on ko-c12.
4. Chinese: add corpus rows. Development split only, sorted, escaped, per `corpus/README.md`. Sources must carry a permissive license and an entry in `resources/datasets/source-lock-v1.json`. Update `NOTICE`.
5. Measure every step with `cargo run --release --locked -p blasphem-bench -- accuracy`.

## Gates to respect

False warnings at most 3% of clean rows. Precision at least 90%. Clean controls in `tests/fixtures/behavior/tr.tsv` and `zh.tsv` must not nudge. Validation and test rows are sealed by `resources/datasets/evaluation-lock-v1.json`. Do not edit them.
