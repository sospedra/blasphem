# How blasphem works

Blasphem flags a message when either of two channels reaches 50 points. The model channel scores the text. The rule channel reads the lexicon and the sentence around each hit. Everything is compiled into one binary per build.

```
                     corpus/XX.tsv                         lexicon/XX.tsv
                (split, label, text)               (lemma, category, level)
                          |                                    |
          +---------------+---------------+                    |
          |               |               |                    v
     development      validation         test          rule pack (per language)
          |               |               |                    |
          v               v               |                    v
   train weights --> calibrate boundary <-------------- rule channel
          |               |               |
          v               v               |
      model.bin  +  boundary, scale       |
          |                               |
          v                               v
   blasphem binary  ----------------> benchmark/  (recall, precision per language)


   one message at run time

   text --> model  --> score 0..100 ------------------+
                                                      +--> max >= 50 ? flag : safe
   text --> rules  --> lexicon hits + frames -> points +
                  \--> suppression: caps the model at 49
```

## Corpus

One file per language, `corpus/XX.tsv`. Three columns: `split`, `label`, `text`. Labels are `clean` or `toxic`. Sources are textdetox and the community corpora, locked in `resources/datasets/`.

| split | rows do |
|---|---|
| development | train the model weights |
| validation | choose the decision boundary |
| test | measure. Never used for tuning |

Malay is stored under `ID`.

## Lexicon

One file per language, `lexicon/XX.tsv`, a copy of `data/clean-room-v1/`. Columns: `id`, `pos`, `category`, `stereotype`, `lemma`, `level`. Built from Wiktionary, LDNOOBW, washyourmouth, and textdetox. No HurtLex rows.

`category` is one of 17 codes. Five mark identity groups: `ps`, `rci`, `om`, `ddf`, `ddp`. `level` is `conservative` or `inclusive`. Only conservative rows load. Inclusive rows are inert (`src/rules/channel.rs`).

The runtime does no stemming. Each inflected form is its own row.

## Model

A hashed linear classifier, `src/sparse.rs`. Features: word unigrams, word bigrams, character 3-, 4-, and 5-grams of the normalized text. Each feature hashes into one of 65,536 bins. Each bin holds one 16-bit log-odds weight. One artifact is 131 KB.

Training (`crates/blasphem-train/src/compiler.rs`):

1. Count clean and toxic document frequencies per bin on the development split.
2. Drop bins seen in fewer than 2 documents.
3. Store the log-odds ratio per bin, quantized to 16 bits, plus a bias.

The raw score is the bias plus the weights of the message bins.

For 13 languages the model also reads the lexicon. One marker word per matched lexicon category is appended to the text before scoring. Training and run time do the same (`lexicon_marked_text`, `src/detector.rs`). The model learns the marker weights like any word. ZH and JA run without markers: their lexica cover too few toxic rows.

## Calibration

The boundary turns a raw score into a verdict. The compiler picks it on the validation split.

For each candidate boundary it predicts `rule nudge OR (not suppressed AND raw >= boundary)`. It keeps the boundary with the most true positives that passes three gates:

1. False warnings at most 3% of clean rows.
2. Precision at least 90%.
3. Boundary above every clean control. The 16 clean fixtures per language in `tests/fixtures/behavior/` must not flag.

The scaled score is 50 at the boundary. The scale comes from the 10th and 90th percentile raw scores on validation.

## Rule channel

The rule channel reads the lexicon and the words around each hit. Points:

| evidence | points |
|---|---|
| conservative lexicon hit | 30 |
| hit plus a target word ("you") nearby | 70 |
| identity hit plus a group word nearby | 85 |
| hostile wish ("I hope you die") | 85 |
| direct threat ("I will kill you") | 95 |

A negator, quote, reporting verb, or counterspeech marker near the hit suppresses it. Suppression cuts the hit to 10 points and caps the model at 49.

Two engines exist. `uses_policy_rules` in `src/rules/channel.rs` picks one per language.

| engine | languages | how the lexicon is used |
|---|---|---|
| policy pack (`src/rule_pack.rs`, `src/policy.rs`) | ES | inside every frame above |
| V2 tables (`src/rules/packs/`) | the other 14 | flat 30-point hit only |

A 30-point hit alone never flags. In the V2 languages the lexicon changes scores, not verdicts.

## Verdict

```
score = max(model score, rule points)
flag  = score >= 50
```

## Build

`blasphem-train regenerate` retrains all 15 models and recalibrates them. It rewrites `resources/models/multilingual-v2/` and `reports/`. `src/embedded.rs` pins the artifact and lexicon digests. `src/registry.rs` pins the rule identity per language. A digest mismatch fails at start.

## Benchmark

```sh
cargo run --release --locked -p blasphem-bench -- accuracy
```

Retrains, syncs digests, rebuilds, and judges every test row through the binary. Writes `benchmark/runs/<sha>.json` and prints each language against `benchmark/baseline.json`. See `benchmark/README.md`.
