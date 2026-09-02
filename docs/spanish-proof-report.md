# Spanish proof report

Verdict: The Spanish detector is a useful conservative pre-send nudge prototype. It is not a full moderation classifier.

The runtime uses no AI service or neural-model runtime. It uses rules, lexicon data, POS metadata, and one sparse integer table.

## Public result

Use `ok` or `should_nudge`. The integer score is ordinal and ranges from 0 through 100.

The threshold is 50. The score is not a probability.

```text
Te voy a matar
ok=false score=95 threshold=50 should_nudge=true action=block lexical_score=0.000

ojala se muera toda tu familia
ok=false score=85 threshold=50 should_nudge=true action=block lexical_score=0.000

No te voy a matar
ok=true score=24 threshold=50 should_nudge=false action=allow lexical_score=0.000

Me das asco
ok=false score=60 threshold=50 should_nudge=true action=review lexical_score=0.000

Los inmigrantes son una plaga
ok=false score=85 threshold=50 should_nudge=true action=block lexical_score=0.000
```

## Runtime design

The detector combines four channels.

1. HurtLex finds normalized words and phrases.
2. Spanish POS rules add high-confidence plural forms.
3. Semantic rules resolve targets, threats, wishes, hostility, and group predicates.
4. A sparse TextDetox table adds distributional word and character evidence.

Quote, report, negation, and counterspeech rules suppress inactive evidence.

Separate untargeted lexicon matches do not add into one nudge. This rule limits false warnings from unrelated words.

The final score is the maximum rule score or sparse score. The runtime does not add unrelated channels.

## Quality results

| Split | Rows | TP | TN | FP | FN | Precision | Recall | Specificity |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Fixed behavior panel | 88 | 39 | 46 | 0 | 3 | 100.0% | 92.9% | 100.0% |
| Validation | 762 | 159 | 382 | 18 | 203 | 89.8% | 43.9% | 95.5% |
| Untouched test | 819 | 177 | 386 | 14 | 242 | 92.7% | 42.2% | 96.5% |

The fixed panel is a behavior contract. Its result is not an unbiased accuracy estimate.

The untouched test false-warning rate is 3.5%. The detector misses 57.8% of labeled toxic rows.

This trade favors fewer interruptions. That choice fits an optional pre-send warning.

The TextDetox test split has 419 toxic rows and 400 clean rows. Its precision does not predict production prevalence.

The three fixed-panel misses are:

- `Puta, vuelve a la cocina.`
- `Si no me mandas fotos desnuda, publicaré las que tengo.`
- `Pinche idiota.`

## Sparse table provenance

The source is [TextDetox multilingual toxicity](https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset).

The fetched dataset revision is `01907546324b0330d2d8b7669648cc18823323e5`.

The source has 5,000 Spanish rows. Deduplication produced 4,999 evaluation rows.

The deterministic split has 3,418 development rows, 762 validation rows, and 819 test rows.

The compiler did not read the test split. The test split ran once after the detector logic froze.

The sparse-only validation matrix is TP 152, TN 388, FP 12, and FN 210.

The artifact SHA-256 is `3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36`.

## Size

The stripped release binary is 4,617,984 bytes. The Spanish HurtLex file is 183,640 bytes.

The deployable Spanish runtime is 4,801,624 bytes, or 4.58 MiB.

The binary already embeds the 131,104-byte sparse table. Do not deploy the table twice.

Training and evaluation TSV files are development data. The runtime does not need them.

## Language status

The CLI accepts the requested language codes after the related HurtLex files exist.

The codes are `EN,ZH,ES,AR,ID,PT,FR,HI,RU,JA,DE,TR,VI,KO,IT`.

EN, ES, AR, PT, FR, RU, DE, and IT have context rule packs.

ZH, ID, HI, JA, TR, VI, and KO currently use the lexical path only.

Spanish is the only language with this sparse table and the new semantic adapter.

Each next language needs its own fixed panel, sparse table, collision audit, and morphology rules.

## Reproduction

```bash
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release --bin toxcheck

target/release/toxcheck eval \
  --input samples/spanish-audit.tsv \
  --minimum-action review

target/release/toxcheck eval \
  --input data/textdetox/es-prepared/test.tsv \
  --minimum-action review
```

Use explicit `--language ES`. Automatic mode is lexical-only.
