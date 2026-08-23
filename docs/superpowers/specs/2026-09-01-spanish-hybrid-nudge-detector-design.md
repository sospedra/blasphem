# Spanish hybrid nudge detector design

## Goal

The Spanish proof shall detect broad hostile messages before the sender submits them.

The public result shall contain one score and one Boolean decision.

The same runtime contract shall support `en,zh,es,ar,id,pt,fr,hi,ru,ja,de,tr,vi,ko,it`.

## Runtime boundary

The Rust runtime shall not call a network service.

The Rust runtime shall not run a neural network, an embedding model, Python, or a model framework.

The Rust runtime may load compiled numeric tables and fixed language packs.

An offline compiler may use labeled datasets, statistical training, translation, and AI-assisted resource drafting.

The runtime shall produce the same result for the same text, language pack, and pack version.

## Public contract

The library shall expose this result.

```rust
pub struct NudgeResult {
    pub score: u8,
    pub threshold: u8,
    pub should_nudge: bool,
}
```

The score shall range from 0 through 100.

The score shall be an ordinal risk score.

The score shall not be described as a probability.

`should_nudge` shall equal `score >= threshold`.

The initial threshold shall be 50.

The CLI shall print `ok`, `score`, `threshold`, and `should_nudge` on its first line.

## Detection channels

The Spanish proof shall use a rule channel and a sparse channel.

The final score shall use the larger channel score.

### Rule channel

The rule channel shall keep the existing HurtLex and context behavior.

The rule channel shall add generic semantic event frames.

The first frames shall cover these events.

- A speaker threatens harm to a person.
- A speaker wishes harm or death on a person.
- A speaker tells a person to harm themselves.
- A speaker uses an implicit second-person copula with an abusive lexical event.
- A speaker uses a directed hostility phrase with an encoded target.
- A speaker applies a hostile predicate to a protected group.

Each frame shall combine separate predicates, targets, and grammatical cues.

The Spanish pack shall contain inflected surface forms for each cue class.

The matcher shall use POS metadata for high-confidence Spanish plural forms.

The matcher shall preserve the canonical HurtLex entry and the raw input span.

The engine shall not contain a branch for one full example sentence.

The existing quote, report, negation, and counterspeech suppressions shall apply to semantic events.

Sentiment shall remain a support signal.

Sentiment shall not create a nudge by itself.

### Sparse channel

The sparse channel shall use a fixed 65,536-bin feature table.

The feature extractor shall use normalized word unigrams, word bigrams, and character n-grams of length three through five.

The feature extractor shall hash features with a versioned deterministic hash.

The offline compiler shall estimate one Bernoulli log-odds weight for each bin.

The compiler shall quantize each weight to a signed 16-bit integer.

The compiled artifact shall store the intercept, decision boundary, score scale, and feature weights.

The runtime shall parse and validate the artifact before scoring text.

The sparse score shall map the validation decision boundary to 50 points.

## Spanish data controls

The compiler shall use the Spanish TextDetox split as training data.

The existing deterministic group hash shall create development, validation, and test splits.

The compiler shall fit feature weights on development rows only.

The compiler shall select the decision boundary on validation rows only.

The sparse boundary shall keep its validation clean false-positive rate at or below three percent.

The compiler shall maximize validation toxic recall under that limit.

The compiler shall not read the test split.

The final evaluator shall read the test split once after the implementation is fixed.

User examples and authored Spanish cases shall be audit cases.

Audit cases shall not contribute to reported test metrics.

## Spanish audit panel

The panel shall cover direct threats, indirect threats, death wishes, targeted abuse, and identity attacks.

The panel shall cover profanity, sexual abuse, imperative harm, and regional forms.

The panel shall include accents and common missing accents.

The clean panel shall cover quotes, reports, negation, counterspeech, news, health, fiction, and benign word collisions.

The panel shall contain no generated expectation from the detector itself.

## Evidence and diagnostics

The public product path needs only `NudgeResult`.

The internal policy result shall retain categories and rule evidence.

The internal result shall record separate rule and sparse scores.

The CLI may print these diagnostics after its first line.

## Size target

The compiled Spanish sparse table shall remain below 0.25 MB.

The Spanish rule data shall remain below 0.10 MB.

The release binary and Spanish data shall remain below 6 MB on the current macOS target.

## Deferred work

This proof shall not add automatic language detection.

This proof shall not claim quality for the other 14 languages.

This proof shall not add a full parser or a large morphology dictionary.

Later language packs may replace the Spanish surface-form strategy with language-specific token profiles.
