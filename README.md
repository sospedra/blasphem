# Blasphem

Blasphem is a deterministic pre-send toxicity nudge. It checks a message before send, across 15 languages. It runs no neural model and no AI runtime.

It ships a native CLI and a browser WASM build. The evidence status is experimental.

Build the native CLI:

```bash
cargo build --release --locked --bin blasphem
```

See `CONTRIBUTING.md` to add training data, `LICENSE` for the first-party license, and `NOTICE` for third-party data licenses.

Blasphem is an experimental multilingual pre-send toxicity nudge. It applies deterministic moderation rules. It does not run an AI model or translate text.

There are three clients: a Rust library, a TypeScript package, and this CLI.

The runtime uses [HurtLex](https://github.com/valeriobasile/hurtlex), fixed dictionaries, deterministic context rules, and sparse integer tables.

Each language uses one independent 128 KiB sparse integer table. Offline labeled corpora produced these tables.

The runtime does not load TextDetox rows. It does not need Python, ONNX, TensorFlow, or a network connection.

## Rust

```rust
use blasphem::{Judge, JudgeOptions, Language};

let judge = Judge::new(JudgeOptions {
    locales: vec![Language::En, Language::Es],
    detect_language: true,
    grawlix: true,
})?;

let verdict = judge.judge("you are a stupid loser");

assert!(!verdict.safe);
assert_eq!(verdict.score, 0.64);
assert_eq!(verdict.locale, Some(Language::En));
assert_eq!(verdict.grawlix.as_deref(), Some("you are a @#$%&! loser"));
```

`JudgeOptions::default()` loads all 15 languages, detects the language, and returns no grawlix.

`score` is an ordinal value from 0.0 through 1.0. It is not a probability, and it is not calibrated across languages.

Text that no loaded locale routes returns `locale: None`, `score: 0.0`, and `safe: true`. The nudge fails open.

`Judge` is a struct rather than a free function because each locale carries its own lexicon and sparse table. Build one judge and reuse it.

The lexica are compiled into the binary. `blasphem::embedded_detector` exposes the same data for a single language.

## TypeScript

```ts
import { judge } from "blasphem";

const v = judge("you are a stupid loser", {
  locales: ["en", "es"],
  detectLanguage: true,
  grawlix: true,
});

v.safe;  // false
v.score; // 0.64
```

The package is isomorphic and runs the same in Node and the browser. Every option is optional. See `packages/blasphem/README.md`.

## Reproduce every artifact

This form passes today:

```bash
cargo run --release --locked -p blasphem-train -- reproduce --skip-browser
```

It verifies raw inputs, rebuilds every model and language artifact, builds
the native and WASM binaries, and runs the Rust tests and Clippy.

The spec's canonical command omits the flag:

```bash
cargo run --release --locked -p blasphem-train -- reproduce
```

The unflagged form additionally runs the npm package checks and the browser
smoke test through `pnpm`. It needs the `test:browser` script in
`packages/blasphem`, which is not yet added.

Both forms read only committed inputs. Neither downloads a corpus, lexicon,
or model source. Both write generated data to a temporary directory and
return a nonzero status after any mismatch.

## Licensing

Blasphem first-party code uses the Apache License 2.0. See `LICENSE`.

The corpora and lexica in this repository keep their own recorded licenses,
including one unresolved license and two share-alike licenses. See `NOTICE`
for the full accounting.

## Setup

Download the default HurtLex language files:

```bash
cargo run --release -p blasphem-train -- setup
```

The shipping detector supports `EN,ZH,ES,AR,MS,PT,FR,HI,RU,JA,DE,TR,VI,KO,IT`.

The parser accepts `ID` as an alias for `MS`.

Download every HurtLex 1.2 language:

```bash
cargo run --release -p blasphem-train -- setup --languages all
```

The runtime uses conservative HurtLex entries only.

## Policy checks

Run these policy checks:

```bash
cargo run --release -p blasphem --bin blasphem -- check --language EN --text "You are an idiot"
cargo run --release -p blasphem --bin blasphem -- check --language EN --text "I will kill you"
cargo run --release -p blasphem --bin blasphem -- check --language EN --text '"You are an idiot," she said'
cargo run --release -p blasphem --bin blasphem -- check --language ES --text "Te voy a matar"
cargo run --release -p blasphem --bin blasphem -- check --language ES --text "ojala se muera toda tu familia"
cargo run --release -p blasphem --bin blasphem -- check --language ES --text "No te voy a matar"
```

The output has five category values: `profanity`, `targeted_abuse`, `identity_attack`, `threat_language`, and `sentiment_support`.

The first line contains `ok`, `score`, `threshold`, and `should_nudge`.

Use `ok` or `should_nudge` for the pre-send message. The integer score ranges from 0 through 100.

The integer score is ordinal. It is not a probability or a confidence score.

The `lexical_score` field is the legacy lexical score from 0.0 through 1.0.

The `sparse_score` line contains the selected language table score.

Sentiment only modifies active lexical or context evidence. Sentiment alone cannot select `review` or `block`.

The policy layer excludes 36 verified HurtLex collisions by language and normalized lemma.

The exclusion table covers AR, DE, EN, ES, FR, IT, PT, and RU.

An excluded term remains in the raw lexical matches. The policy output adds `lexical_collision_excluded` evidence with zero points.

An exclusion does not reduce the legacy lexical `score` field.

Lexical rule evidence includes the candidate view, normalized offsets, and raw UTF-8 byte offsets.

The selected language applies only its language-specific exclusions.

Explicit German reactivates `Hund` only for the exact direct-abuse phrase `du hund`.

The runtime does not derive the exclusion table from TextDetox labels.

The CLI has context packs for all 15 supported languages.

Reply context can supply a target:

```bash
cargo run --release -p blasphem --bin blasphem -- check --language EN --reply-target person --text "Idiot"
```

Use `protected-group` for a protected-group reply target. The caller must supply all reply-target context.

Use `--language AUTO` to detect a supported language before the toxicity check:

```bash
cargo run --release --bin blasphem -- check --language AUTO --text "Cuando te vea, te rompo los dientes"
```

AUTO returns an unknown route for unreliable input.

Unsupported-language rejection is best-effort with this 15-profile model.

Unsupported input can misroute to a supported language.

An unknown route sets `ok=true`, `score=0`, and `evaluated=false`.

Use an explicit code to bypass the language detector.

Spanish uses POS-aware plural expansion for high-confidence noun and adjective forms.

Spanish also detects directed hostility, harm wishes, self-harm commands, threats, and direct group hostility.

Negation, quotes, reports, and counterspeech can suppress these semantic events.

## Spanish checks

Run the fixed Spanish behavior panel:

```bash
cargo run --release -p blasphem-train -- eval \
  --input samples/spanish-audit.tsv \
  --minimum-action review
```

The recorded Spanish test result is `previously_evaluated` experimental evidence.

Do not rerun the Spanish test split during calibration or behavior work.

The behavior panel is a contract set. Do not treat its result as an unbiased production estimate.

## TextDetox data

[TextDetox](https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset) supports offline sparse-table compilation and evaluation.

The offline pipeline downloads one pinned Parquet file for each catalog TextDetox language.

The pipeline parses each Parquet file locally and publishes the same canonical acquisition TSV.

The acquisition TSV has this header:

```text
source_id<TAB>language<TAB>toxic<TAB>text
```

The accepted TextDetox source codes are:

```text
am ar de en es fr he hi hin it ja ru tt uk zh
```

`hin` means Hinglish and maps to `HINGLISH`. `hi` means Hindi and maps to `HI`.

Acquisition records the pinned Hugging Face dataset revision in each source ID.

Acquisition rejects a URL that does not contain the pinned revision.

The source record stores separate SHA-256 values for the Parquet download and the canonical TSV.

The catalog TextDetox set is EN, ZH, AR, FR, HI, RU, JA, DE, and IT.

Hash buckets 0 through 69 select development. Buckets 70 through 84 select validation. Buckets 85 through 99 select test.

The hash split is deterministic and is not stratified. Realized split counts can differ from 70/15/15.

Exact normalized duplicates within one detector language stay in one group. Label-conflict groups do not enter evaluation.

The `provenance.tsv` file records every source row. It records duplicate, conflict, exclusion, group, and split data.

Paraphrases, copied templates, and cross-language copies can still cross splits.

Do not tune rules or thresholds on the test split.

Compile the model set from development, validation, and frozen clean behavior controls:

```bash
cargo run --release --locked -p blasphem-train -- compile \
  --prepared-root data/prepared-v1 \
  --hurtlex-root data/raw-v1/hurtlex \
  --behavior-root tests/fixtures/behavior \
  --output resources/models/multilingual-v2
```

The compiler uses word unigrams, word bigrams, and character 3-grams through 5-grams.

The compiler hashes features into 65,536 bins. It writes fixed integer weights and a validation boundary.

The clean controls set a minimum boundary. They do not enter the accuracy metrics.

The checked-in manifest is `resources/models/multilingual-v2/manifest.json`.

## Pre-test evidence

Write the 14-language validation evidence:

```bash
cargo run --release --locked -p blasphem-train -- evaluate \
  --split validation \
  --prepared-root data/prepared-v1 \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --output reports/multilingual-validation.json
```

This report is calibration evidence. It is not an independent test estimate.

The report records each language matrix, precision, recall, specificity, F1, and false warning rate.

It also projects precision at one-percent and five-percent toxic message prevalence.

Write the 360-case behavior contract evidence:

```bash
cargo run --release --locked -p blasphem-train -- behavior \
  --fixture-root tests/fixtures/behavior \
  --prepared-root data/prepared-v1 \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --output reports/multilingual-behavior.json
```

The behavior panels are authored contract evidence. They are not a production accuracy sample.

Write the 60-case native smoke evidence:

```bash
cargo run --release --locked -p blasphem-train -- cli-smoke \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/raw-v1/hurtlex \
  --output reports/multilingual-cli-smoke.json
```

The smoke report covers all 15 languages. It records two supplied cases and two context cases per language.

These commands call the shipping `NudgeDetector::check` path. They do not open a prepared test split.

The legacy `eval` command accepts manual audit TSV files. Do not use it for a frozen test split.

The audit TSV header is `language<TAB>label<TAB>text`. A label is `toxic` or `clean`.

The evaluator reports overall and per-language confusion matrices. It also reports accuracy, precision, recall, specificity, and F1.

TextDetox source labels have different definitions. Binary labels cannot measure category accuracy.

The source corpora use different label rules. They do not represent production prevalence.

Evaluation errors can identify manual audit candidates. Runtime code never generates exclusions from TextDetox labels.

Any row that influences a rule becomes audit-only. Do not report that row in later quality metrics.

## Offline dataset workflow

Observe the catalog before the source review:

```bash
cargo run --release --locked -p blasphem-train -- observe \
  --source-catalog resources/datasets/source-catalog-v1.json \
  --output data/source-observation-v1
```

Review `data/source-observation-v1/source-observation-v1.json` and every recorded license before freezing.

```bash
cargo run --release --locked -p blasphem-train -- freeze-sources \
  --observation data/source-observation-v1/source-observation-v1.json \
  --reviewed \
  --output resources/datasets/source-lock-v1.json

cargo run --release --locked -p blasphem-train -- acquire \
  --source-lock resources/datasets/source-lock-v1.json \
  --output data/raw-v1

cargo run --release --locked -p blasphem-train -- prepare \
  --source-lock resources/datasets/source-lock-v1.json \
  --raw-root data/raw-v1 \
  --output data/prepared-draft-v1
```

The commands refuse an existing output. The acquire command verifies each frozen identity and SHA-256 digest before publication.

## Limits

HurtLex contains ambiguous and outdated entries. The conservative set can still produce false positives.

The fixed exclusion table can miss other abusive uses of dual-use words.

Identity attacks require a group target plus a direct relation, net negative lexical support, or protected-group reply context.

Only Russian and Arabic accept whitespace-only zero-copula identity syntax.

The fixed dictionaries have unequal language coverage.

The detector can miss new words, unsupported morphology, code-switching, and unsupported evasions.

Quote, report, negation, and counterspeech suppression use short rule windows.

All suppressed lexical spans add at most ten profanity points for one message.

General stemming remains disabled. Spanish has a small POS-aware plural rule set.

CJK detection uses compact character rules and character n-grams.

The runtime does not need the large Japanese and Korean tokenizer dictionaries.

The `blasphem-train setup`, `observe`, and `acquire` commands use the network.

HurtLex setup reads an unpinned `master` URL. Retain downloaded files for repeatable runs.

## Performance gate

Run the release-only dense-message latency gate:

```bash
cargo perf-gate
```

The gate checks 30 dense fixtures. It requires one short and one long fixture for each supported language.

## Browser WASM

The default build includes the language detector and all 15 toxicity packs.

Build the default browser module and generate the web bindings:

```bash
cargo build --release --locked --target wasm32-unknown-unknown -p blasphem-wasm
wasm-bindgen target/wasm32-unknown-unknown/release/blasphem_wasm.wasm \
  --target web \
  --out-dir target/wasm-web \
  --out-name blasphem
```

Initialize an explicit detector or an automatic detector:

```js
import init, { BlasphemDetector } from "./blasphem.js";

await init();
const detector = new BlasphemDetector("AUTO");
const result = detector.check("أتمنى أن تموت وحيدًا هذه الليلة");

console.log(result.ok, result.score, result.shouldNudge, result.resolvedLanguage);
result.free();
detector.free();
```

Build the explicit-only module without the language detector:

```bash
cargo build --release --locked --target wasm32-unknown-unknown \
  -p blasphem-wasm --no-default-features
```

The explicit-only module rejects `AUTO`.

The measured explicit-only transfer is 1,630,271 Brotli bytes, including JavaScript glue.

The measured default transfer is 9,038,194 Brotli bytes, including JavaScript glue.

The current WASM module still embeds all 15 toxicity packs in both build modes.

A future product pack loader should fetch only the selected N language packs.

AUTO should detect first and load only the resolved available pack.

A missing pack must return unknown and fail open.

The shared language detector table does not shrink in proportion to the selected pack count.

See the automatic-language-detection report under `docs/` for route, latency, size, and browser evidence.

Run the actual Chrome smoke test:

```bash
./crates/blasphem-wasm/verify-browser.sh
```

The test runs the default and explicit-only modules in separate WASM instances.

The explicit-only check verifies an English route and the `AUTO` feature error.

The test writes experimental browser and compressed-size evidence to `reports/multilingual-wasm.json`.
