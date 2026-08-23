# Multilingual sparse nudge detector design

## Goal

The detector shall warn a sender before the sender submits a likely toxic message.

The detector shall favor low false-warning rates and high precision over recall.

The detector shall support `en,zh,es,ar,id,pt,fr,hi,ru,ja,de,tr,vi,ko,it`.

The Spanish behavior and Spanish artifact shall remain unchanged.

## Runtime boundary

The Rust runtime shall not call a network service.

The Rust runtime shall not translate text.

The Rust runtime shall not run a neural network, embedding model, Python process, or model framework.

The Rust runtime may load embedded numeric tables and fixed language packs.

Offline commands may download datasets and compile numeric artifacts.

The runtime shall return the same result for one complete behavior version.

The behavior version shall identify the model, feature schema, normalization, rules, and HurtLex resource.

The `toxcheck` binary shall contain the shipping detector path.

The `toxtrain` binary shall contain acquisition, preparation, compilation, and dataset evaluation commands.

The `toxcheck` binary shall not link network or dataset parser code.

## Public result

The library shall keep this result.

```rust
pub struct NudgeResult {
    pub score: u8,
    pub threshold: u8,
    pub should_nudge: bool,
}
```

The score shall range from 0 through 100.

The score shall be an ordinal risk score.

The score shall not represent a probability.

Scores from different languages shall not be compared.

The public threshold shall remain 50.

Each language-specific raw boundary shall map to the public threshold.

`should_nudge` shall equal `score >= threshold`.

The CLI first line shall keep `ok`, `score`, `threshold`, and `should_nudge`.

## Selected architecture

The runtime shall use one independent sparse artifact for each language.

Each language shall use an independent raw boundary and score scale.

The runtime shall combine one sparse score with one deterministic rule score.

The final score shall use the larger channel score.

For new languages, contextual rule evidence may keep the sparse channel below the public threshold.

The compiler and runtime shall apply the same contextual sparse decision.

The runtime shall not use a shared multilingual weight table.

The runtime shall not route text through an English model.

The runtime shall not detect a language automatically.

The caller shall supply one supported two-letter language code.

An unknown language code shall return an error.

The runtime shall not silently fall back to English.

## Data flow

The `toxtrain` binary shall acquire pinned source files.

Each importer shall convert one source schema into common labeled rows and provenance rows.

The preparation stage shall group duplicates, exclude conflicts, and freeze splits.

The compiler shall fit sparse weights and select one final-path boundary per new language.

The batch publisher shall write artifacts and manifests atomically.

The `toxcheck` binary shall select one registry entry from the supplied language code.

The runtime shall normalize the text with the selected immutable profile.

The runtime shall score the rule and sparse channels.

The runtime shall return the larger score and its Boolean decision.

## Error handling

Acquisition shall fail on an unexpected source revision or file hash.

An importer shall fail on a missing required column or invalid source label.

An importer shall record an exclusion reason for supported ambiguous rows.

Preparation shall fail when one required class or split is empty.

Compilation shall fail when one gate, profile, or language declaration is invalid.

Publication shall leave the previous complete artifact set unchanged after any failure.

Runtime initialization shall fail on a corrupt embedded artifact.

The CLI shall exit with a nonzero status and name the failed language.

## Language registry

The code shall define one typed registry for all 15 languages.

Each registry entry shall contain these values.

- The public language code.
- The embedded sparse artifact.
- The sparse model cache.
- The feature profile.
- The normalization profile.
- The deterministic rule pack.
- The rule-pack version and expected artifact hash.
- The expected HurtLex resource hash when the language uses HurtLex.

The registry shall validate the artifact language, feature profile, normalization profile, and feature schema.

The registry shall validate the external HurtLex hash when that resource is required.

The runtime shall parse each selected artifact once.

The runtime shall cache each selected rule pack once.

## Sparse feature profiles

Every sparse artifact shall use 65,536 feature bins.

The `EsLegacyWordChar35V1` profile shall retain the current Spanish feature extractor exactly.

ES shall use `EsLegacyWordChar35V1`.

Version-two word features shall use normalized word unigrams and word bigrams.

A version-two word token shall be one maximal run of Unicode letters, numbers, or combining marks.

A Unicode punctuation, symbol, separator, or control shall end a version-two word token.

Hindi tokens shall permit U+200C and U+200D between Devanagari token characters.

Version-two word bigrams shall not cross a clause boundary or line break.

Clause boundaries shall include `.`, `!`, `?`, `;`, `:`, `。`, `！`, `？`, `；`, `：`, `؟`, `؛`, and `।`.

The `WordChar35V2` profile shall add character n-grams of length three through five.

Its character n-grams shall remain inside one word token.

Each token shall use U+0002 and U+0003 as start and end sentinels.

EN, AR, ID, PT, FR, HI, RU, DE, TR, VI, and IT shall use `WordChar35V2`.

The `Char25V2` profile shall add character n-grams of length two through five.

ZH, JA, and KO shall use `Char25V2`.

The `Char25V2` profile shall not generate word features.

The `Char25V2` profile shall split segments at punctuation, symbols, controls, and line breaks.

The `Char25V2` profile shall remove whitespace inside each remaining segment.

The `Char25V2` profile shall generate n-grams inside one compact segment only.

Each compact segment shall use U+0002 and U+0003 as start and end sentinels.

Character features shall use normalized Unicode scalar values.

The compiler and runtime shall use the same feature profile.

The artifact shall declare the feature profile.

The artifact shall declare the normalization profile.

## Normalization profiles

The Spanish version-one artifact shall keep the current legacy Charabia normalization.

Version-two artifacts shall not use the legacy Charabia normalizer.

The generic version-two profile shall apply NFKC and Unicode lowercase conversion.

EN, ID, PT, FR, RU, DE, and IT shall use the generic profile.

The Turkish profile shall apply NFKC first.

It shall map `I` to `ı` and `İ` to `i` before Unicode lowercase conversion.

The Turkish profile shall preserve the remaining letters and marks.

The Vietnamese profile shall apply NFKC and Unicode lowercase conversion.

The Vietnamese profile shall preserve all tone marks.

The Arabic profile shall apply NFKC first.

The Arabic profile shall remove U+0640 tatweel.

It shall remove marks U+0610-U+061A, U+064B-U+065F, U+0670, and U+06D6-U+06ED.

It shall fold U+0622, U+0623, U+0625, and U+0671 to U+0627 `ا`.

The Arabic profile shall keep all other Arabic letters distinct.

The Hindi profile shall apply NFKC and preserve Devanagari vowel marks and conjuncts.

The Hindi profile shall preserve U+200C ZWNJ and U+200D ZWJ.

The Chinese, Japanese, and Korean profiles shall apply NFKC and Unicode lowercase conversion.

These profiles shall preserve their native scripts.

The Japanese profile shall preserve composed dakuten forms.

The Korean profile shall preserve composed Hangul syllables.

The CJK profiles shall create a compact script view for no-space phrase matching.

The compact script view shall retain every clause boundary.

The compact script view shall not join text across punctuation.

No profile shall use a large segmentation dictionary.

## Sparse artifact format

The runtime shall continue to read the existing `TOXSPRS1` Spanish artifact.

The existing version-one artifact shall imply the `EsLegacyWordChar35V1` profile.

New artifacts shall use a version-two header.

The version-two header shall store a feature-profile identifier.

The version-two header shall store a normalization-profile identifier.

The version-two header shall store a feature-schema identifier.

The version-two header shall keep the language, bin count, bias, boundary, and score scale.

The version-two header shall keep the payload length, false-warning limit, and weight scale.

The feature schema shall freeze FNV-1a, namespaces, sentinels, n-gram bounds, and document-level bin deduplication.

The schema shall use 64-bit FNV-1a offset `0xcbf29ce484222325`.

The schema shall use 64-bit FNV-1a prime `0x00000100000001b3`.

The bin index shall equal the low 16 hash bits.

Word hashes shall use namespace `W`, the arity byte, and one NUL byte before each UTF-8 token.

Character hashes shall use namespace `C`, the length byte, and each scalar encoded as UTF-8.

Each artifact shall store 65,536 signed 16-bit weights.

The loader shall reject an invalid magic value, version, profile, language, size, or scale.

Manifest tests shall reject an artifact SHA-256 mismatch.

The build shall embed one raw artifact per language.

The build shall not embed complete HurtLex source files.

The first implementation shall not add artifact compression.

## Rejected alternatives

One shared multilingual table would reduce size but would mix hash collisions and calibration.

English translation would add latency, privacy exposure, slang loss, and another runtime dependency.

A universal POS tagger would add large dictionaries and uneven language behavior.

## Sparse training

The compiler shall use Bernoulli document log odds.

The compiler shall count each feature bin once per document.

The compiler shall use a minimum document frequency of two.

The compiler shall quantize weights to signed 16-bit integers.

The compiler shall fit weights on development rows only.

The compiler shall calculate weights from development rows only.

The authored clean controls shall set a minimum raw boundary.

The compiler shall select the raw boundary from validation candidates at or above that minimum.

Clean controls shall guard product behavior. They shall not count as accuracy evidence.

The compiler shall not read any test row.

The rule pack shall freeze before validation boundary selection.

Boundary selection shall score the final contextual `max(rule, sparse)` path.

Negation, quotation, reporting, and counterspeech evidence shall suppress the sparse decision for new languages.

The compiler shall fail when the frozen rule channel already exceeds one validation gate.

The compiler shall fail when the frozen rule channel warns on one clean control.

The compiler shall search every distinct validation raw score and each adjacent boundary.

The compiler shall maximize validation recall under both validation gates.

A recall tie shall select the boundary with fewer false warnings.

A remaining tie shall select the higher raw boundary.

The clean false-warning rate shall stay at or below three percent.

Ordinary precision shall stay at or above 90 percent.

Ordinary precision shall equal `TP / (TP + FP)` on the validation rows.

The gate comparisons shall use integer arithmetic.

The false-warning gate shall require `10000 * FP <= 300 * (FP + TN)`.

The precision gate shall require `100 * TP >= 90 * (TP + FP)`.

An admissible boundary shall warn on at least one validation toxic row.

The compiler shall fail when no admissible boundary exists.

These validation gates shall apply to the 14 new language artifacts.

The existing Spanish validation and test evidence shall remain unchanged.

## Rule channel

The rule channel shall remain small and deterministic.

The rule channel shall cover four high-confidence event types.

- A speaker directly threatens a target.
- A speaker wishes harm or death on a target.
- A speaker tells a target to harm themselves.
- A speaker directs a strong insult at a target.

A direct threat shall require intent or an imperative, a target, and a harmful predicate.

A harm wish shall require a wish cue, a target, and a harmful outcome.

A self-harm command shall require an imperative and a direct or reflexive target.

A directed insult shall require a target cue and a strong insult cue.

A pack may encode an implicit target only through an exact whole-proposition event phrase.

This exact path may cover threats, harm wishes, and directed insults with inflected target cues.

Each pack shall define its proposition boundary phrases.

Negation, balanced quotation, and linked report cues shall suppress one matched event.

Quotation shall suppress an event only when one balanced quote contains the complete event frame.

A question about violence shall not become a threat without speaker intent.

Sentiment shall remain a support signal.

Sentiment shall not create a nudge by itself.

The implementation shall not add a universal POS tagger.

The implementation shall not add a general stemmer.

Each language pack may list a small set of surface forms and safe affix patterns.

Chinese, Japanese, and Korean phrase rules shall scan code-point sequences.

Arabic shall use one rule pack for MSA and common social Arabic.

The Arabic pack may contain a small Arabizi phrase list.

The Arabic pack shall not detect a dialect or run general transliteration.

## Dataset sources

The pipeline shall pin every source revision or downloaded file hash.

The pipeline shall pin every required HurtLex file hash.

The source manifest shall record the observed revision and file SHA-256 value.

The existing Spanish source and split shall remain frozen.

The Spanish artifact SHA-256 value shall remain `3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36`.

TextDetox shall supply EN, ZH, AR, FR, HI, RU, JA, DE, and IT rows.

TextDetox acquisition shall pin revision `01907546324b0330d2d8b7669648cc18823323e5`.

Hindi shall use TextDetox code `hi`.

The pipeline shall not map TextDetox Hinglish code `hin` to Hindi.

TextDetox label `1` shall mean toxic.

TextDetox label `0` shall mean clean.

Ibrohim-Budi `re_dataset.csv` shall supply Indonesian rows.

An Indonesian row shall be toxic when `HS=1` or `Abusive=1`.

An Indonesian row shall be clean when both fields equal zero.

ToLD-Br `ToLD-BR_alpha.csv` shall supply Brazilian Portuguese rows.

Each Portuguese annotator shall vote toxic when that annotator marks any category.

A Portuguese row shall be toxic when at least two annotators vote toxic.

A Portuguese row shall be clean when no annotator votes toxic.

The importer shall exclude rows with exactly one toxic annotator.

OffensEval-TR shall supply Turkish rows.

Turkish label `OFF` shall mean toxic.

Turkish label `NOT` shall mean clean.

ViHOS shall supply Vietnamese rows.

A Vietnamese row with one or more harmful spans shall be toxic.

A Vietnamese row without a harmful span shall be clean.

K-MHaS shall supply Korean rows.

The Korean label set `{8}` shall mean clean.

A Korean label set shall mean toxic when it intersects `{0,1,2,3,4,5,6,7}` and omits `8`.

The importer shall reject a Korean row that mixes label `8` with a toxic label.

The Vietnamese importer shall reject malformed or out-of-range harmful spans.

## Dataset splits and provenance

TextDetox, Indonesian, and Portuguese rows shall use the existing deterministic group split.

The group hash shall use the uppercase detector code encoded as UTF-8.

The hash shall then read one NUL byte and the split-normalized text encoded as UTF-8.

The group hash shall use FNV-1a-64 over that exact byte sequence.

Hash buckets zero through 69 shall select development.

Hash buckets 70 through 84 shall select validation.

Hash buckets 85 through 99 shall select test.

The split shall group exact normalized duplicates within one language.

The split shall exclude normalized groups with conflicting labels across every source split.

Turkish shall keep the official test split.

Turkish training hash buckets zero through 84 shall select development.

Turkish training hash buckets 85 through 99 shall select validation.

Vietnamese and Korean shall keep their official development, validation, and test splits.

If one normalized text crosses official splits, the protected split shall keep the smallest source identifier.

Test shall have higher protection than validation.

Validation shall have higher protection than development.

Every non-representative duplicate row shall receive the `duplicate` exclusion status.

Every source row shall produce one provenance record.

The provenance record shall include these values.

- The dataset and source row identifiers.
- The immutable source URL, revision, file path, file hash, and acquisition time.
- The license identifier, license URL, citation, and upstream lineage.
- The source and detector language codes.
- The source and detector labels.
- The label-conversion, split, and normalization versions.
- The canonical group and representative identifiers.
- The source and detector splits.
- The inclusion status and exclusion reason.

The source manifest shall count every source label, detector label, split, and exclusion reason.

The source manifest shall mark the unresolved TextDetox lineage for Chinese and French.

Unresolved lineage shall not block this experimental spike.

Authored examples shall never enter reported dataset metrics.

Any dataset row used to change a rule shall become audit-only.

## Generated manifest

The batch compiler shall generate one model manifest.

The manifest shall record all 15 registry entries.

Each entry shall include these values.

- The language and feature profiles.
- The dataset name, revision, and input hashes.
- The rule-pack version and expected HurtLex hash.
- The frozen clean-control count and SHA-256 value.
- The development, validation, test, duplicate, conflict, and excluded counts.
- The raw boundary, score scale, and false-warning limit.
- The validation confusion matrix and metrics.
- The artifact size and SHA-256 value.

The compiler shall write artifacts and the manifest through an atomic staging directory.

The compiler shall reject a partial 15-language output set.

The compiler shall produce identical artifacts from identical inputs.

## Behavior panels

Each new language shall have one fixed behavior panel with at least 24 cases.

Each panel shall contain at least eight high-confidence toxic cases.

The toxic cases shall contain two threats, two harm wishes, two self-harm commands, and two directed insults.

Each toxic case shall have one minimally edited clean control when the language permits it.

A panel shall add a replacement clean context when one paired edit is not valid.

Each new panel shall contain at least 16 clean cases.

The minimally edited controls shall cover negation, quotation, reporting, and violence questions.

Each panel shall contain at least eight extra clean collision or context controls.

The extra controls shall cover benign sentiment and language-specific collisions.

The extra controls shall include Unicode, mixed-script, fiction, news, or medical cases.

The panel shall contain original expectations.

Each panel row shall record dataset evidence, native-speaker review, or clearly labeled authored evidence.

Authored evidence is permitted for this experimental pass because the user requested invented phrases.

Authored evidence shall never be represented as native-speaker review.

The detector shall not generate panel expectations.

The panels shall be contract tests, not unbiased accuracy samples.

Every new language panel shall match every expected Boolean result.

The existing Spanish behavior panel shall remain unchanged.

## Evaluation protocol

The implementation shall freeze each language before opening its test rows.

The test evaluator shall run once against each frozen language version.

The evaluator shall report the confusion matrix for each language.

The evaluator shall report precision, recall, specificity, and F1.

The evaluator shall report the clean false-warning rate.

The false-warning rate shall equal `FP / (FP + TN)`.

Precision shall equal `TP / (TP + FP)`.

Recall shall equal `TP / (TP + FN)`.

Undefined precision shall fail one gate.

The evaluator shall report projected precision at one-percent and five-percent toxic prevalence.

Projected precision shall use `p * recall / (p * recall + (1 - p) * FWR)`.

Every metric and gate shall apply to one language.

Pooled metrics shall not rescue a failed language.

The untouched test shall have no recall minimum.

Each new language validation and test split shall contain at least 300 rows from each class.

The untouched test false-warning rate shall stay at or below three percent.

The untouched test precision shall stay at or above 90 percent.

The test gates shall apply to the 14 new languages.

The Spanish report shall retain its existing metrics and shall not claim the new gates.

The report shall mark every validation gate as passed or failed.

The report shall show test results without post-test threshold changes.

A failed test gate shall require a new sealed test version before another final quality claim.

The report shall identify dataset labels that exceed the product policy scope.

## Performance and size

The release build shall embed all 15 sparse artifacts.

Each sparse artifact shall remain below 0.25 MiB.

The `aarch64-apple-darwin` `toxcheck` binary shall remain at or below 7,340,032 bytes.

Fourteen added raw tables estimate a 6,453,440-byte binary before new runtime code.

The final release build shall control the size result.

The offline `toxtrain` binary shall not use the shipping size gate.

The benchmark shall use 90 checked-in and hashed message fixtures.

Each language shall have clean, toxic, and dense-match fixtures at two lengths.

The short fixtures shall contain exactly 280 Unicode scalar values.

The long fixtures shall contain exactly 4,096 valid UTF-8 bytes.

The benchmark shall run every language in a release build.

The build shall use `--release --locked` and the shipping feature set.

The benchmark shall reuse one initialized detector.

The benchmark shall run 100 warm-up calls per fixture.

The benchmark shall collect 5,000 samples per short fixture.

The benchmark shall collect 1,000 samples per long fixture.

The benchmark shall report p50, p95, p99, and maximum latency.

The benchmark shall report checks per second, bytes per second, and peak resident memory.

The target 95th-percentile latency shall stay below one millisecond for 280 characters.

The target 95th-percentile latency shall stay below ten milliseconds for 4 KiB.

Every benchmark fixture shall meet the target for its length.

Performance results shall name the computer, target triple, and Rust version.

The 4 KiB benchmark length shall not limit the public API.

The runtime shall not truncate longer input.

## Tests

Unit tests shall cover both artifact versions and all three feature profiles.

Unit tests shall cover invalid languages, profiles, sizes, and model-language mismatches.

Manifest tests shall cover missing artifacts and SHA-256 mismatches.

Registry tests shall cover every supported language exactly once.

Feature tests shall cover Turkish case conversion and Vietnamese tone preservation.

Feature tests shall cover Arabic normalization and CJK no-space text.

Compiler tests shall prove deterministic output and validation gate enforcement.

Importer tests shall cover every source label conversion and split rule.

Provenance tests shall cover duplicates, conflicts, official splits, and exclusions.

Policy tests shall run every language behavior panel.

Integration tests shall confirm the public score and Boolean invariant for every language.

The final verification shall run `cargo fmt --check`.

The final verification shall run `cargo test --all-targets`.

The final verification shall run `cargo clippy --all-targets -- -D warnings`.

The final verification shall run `cargo build --release --locked --bin toxcheck`.

## Final report

The final report shall list the source and artifact version for each language.

The final report shall list the rule-pack version and HurtLex hash for each language.

The report shall list validation metrics for each language.

The report shall label earlier test metrics as previously evaluated experimental evidence.

The final run shall not reopen the test splits.

The report shall include the complete behavior-panel result matrix.

The report shall include release size, artifact sizes, speed, and memory.

The report shall list external HurtLex resource bytes separately.

The report shall state that the score is ordinal and not a probability.

The report shall state that the source datasets do not represent production prevalence.

The report shall state that dataset toxicity labels differ by source.

The report shall disclose any language that misses one approved gate.

## Deferred work

This version shall not add automatic language detection.

This version shall not add a translation fallback.

This version shall not add a universal POS tagger.

This version shall not add large Chinese, Japanese, or Korean dictionaries.

This version shall not add separate Arabic dialect models.

This version shall not claim production accuracy from balanced dataset metrics.
