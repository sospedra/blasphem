# Handoff: clean controls that pin the boundary

In some languages one clean fixture sets the decision boundary. The corpus does not. Lowering the boundary would flag that fixture, so calibration stops there and recall is capped.

## Mechanism

`clean_control_boundary` in `crates/blasphem-train/src/compiler.rs:629`. For each of the 16 `expected_nudge=false` rows in `tests/fixtures/behavior/XX.tsv`, the compiler scores the model. The boundary must sit above the highest score. A control is skipped when the rule channel suppresses the model on it: quote, negation, reporting verb, or counterspeech marker nearby.

A control that scores 49 at run time sits directly under the boundary. That is the pin.

## Current pins

Measured with `blasphem check --language XX --text=... --data-dir data/clean-room-v1`, current models.

| lang | score | control | text |
|---|---:|---|---|
| EN | 49 | EN-C15 | I will kill your process |
| RU | 49 | RU-C16 | Ты написал thank you |
| KO | 45 | ko-c12 | 북괴돼지새끼 살아있으면 좋겠네 |
| FR | 46 | FR-C11 | Tu es mauvais aux échecs |

The other 11 languages sit at 43 or lower. Their boundaries come from the gates, not from a control.

## Why it matters

EN recall is 81%, RU 82%. Both are held there by one sentence each. The model scores "I will kill your process" as nearly toxic. That is a model weakness, and the fixture is right to exist.

## Options

1. Teach the model. Add clean development rows with threat vocabulary in benign use: processes, games, jokes, quoted speech. Then recompile. The pin drops when the fixture scores lower.
2. Suppress through the rules. The English policy pack already lists "kill your process" in `benign_harm_phrases` (`src/rule_pack.rs`), but English runs the V2 rules, which have no such list. A benign-phrase suppression in V2 would skip the control. This is a rule change and needs the behavior panel to stay green.
3. Leave it. The fixture guards a real false alarm.

## How to check

1. `blasphem check` on the fixture text prints `sparse_score`. Under 45 means unpinned.
2. `boundary` per language in `resources/models/multilingual-v2/manifest.json` should drop after a recompile.
3. `cargo run --release --locked -p blasphem-bench -- accuracy` for the recall effect.
