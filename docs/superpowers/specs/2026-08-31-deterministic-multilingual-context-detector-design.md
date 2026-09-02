# Deterministic multilingual context detector design

## Goal

The CLI shall detect toxic message patterns without model inference.

The detector shall keep lexical matches separate from context rules.

The detector shall explain every nonzero category score with stable rule evidence.

## Runtime boundary

The runtime shall use HurtLex, fixed language packs, and deterministic rules.

The runtime shall not call a translation service.

The runtime shall not load neural weights or fitted classifier parameters.

The TextDetox labels shall support evaluation only.

TextDetox evaluation errors may identify audit candidates. Runtime code shall not generate exclusions from dataset labels.

The detector shall call every numeric result a rule score or risk point value.

The detector shall never call a rule score a probability.

## Supported language packs

The first release shall include EN, ES, FR, DE, IT, PT, RU, and AR packs.

Each pack shall define target terms, negators, threat verbs, report cues, and counterspeech cues.

Each pack shall define small positive and negative sentiment lexicons.

Unsupported explicit languages shall use raw HurtLex scoring only.

Automatic mode shall use lexical scoring and per-entry collision exclusions.

The fallback shall not create target, threat, sentiment, capitalization, or punctuation evidence.

## Pipeline

The detector shall run this fixed pipeline.

1. Preserve the original text.
2. Create normalized, confusable, and evasion candidate views.
3. Find HurtLex phrase matches in every candidate view.
4. Tokenize the original text into punctuation-bounded clauses.
5. Find target, negation, quotation, report, counterspeech, and threat events.
6. Calculate clause-local sentiment support.
7. Calculate category risk points.
8. Select an action from the category values.

The normalizer shall use Charabia.

The confusable view shall use the Unicode security skeleton.

The evasion view shall join runs of three or more separated single-letter tokens.

The evasion view shall map digits only inside mixed letter-and-digit tokens.

The digit map shall be `0:o`, `1:i`, `3:e`, `4:a`, `5:s`, and `7:t`.

An evasion transform shall create a candidate view only.

The detector shall never replace the original text with an evasion view.

## Output types

`PolicyResult` shall contain the original text, one `Detection`, category scores, one action, and rule evidence.

`Detection` shall contain normalized text, the legacy lexical score, and lexical matches.

`CategoryScores` shall contain these unsigned integer fields.

- `profanity`
- `targeted_abuse`
- `identity_attack`
- `threat_language`
- `sentiment_support`

`PolicyAction` shall contain `Allow`, `Review`, and `Block`.

`RuleEvidence` shall contain a stable rule ID, category, risk points, language, and matched text.

`LexiconMatch` shall contain the candidate view, normalized offsets, and raw UTF-8 byte offsets.

Lexical-derived `RuleEvidence` shall contain the candidate view, normalized offsets, and raw UTF-8 byte offsets.

Rule-only evidence shall set the candidate view and normalized offsets to `None`.

The candidate view and both normalized offsets shall be all present or all absent.

## Context rules

A target exists when a clause contains a second-person term or an `@name` mention.

A reply context also supplies a target.

A lexical match and a target shall form targeted abuse within the same clause.

A four-token distance shall limit the target and lexical match relation.

A direct threat shall contain a threat verb and a target in the same clause.

A five-token distance shall limit the threat verb and target relation.

An intent marker shall support a threat when it occurs within three tokens of the threat verb.

The English `kill yourself` form shall count as a direct threat without an intent marker.

A negator within three tokens before an event shall suppress that event.

Quotation marks around an event shall suppress targeted abuse, identity attack, and threat escalation.

A report cue in the same clause shall suppress the same escalation.

A counterspeech cue within four tokens before an event shall suppress the same escalation.

Suppression shall not remove the underlying lexical match.

Suppression shall cap the combined suppressed profanity score at 10 points for one message.

A multi-token negator shall suppress a lexical event that overlaps the negator phrase.

## Lexical collision rules

The policy layer shall exclude verified HurtLex collisions by source language and normalized lemma.

The exclusion shall remove only the lexical event from policy scoring.

The exclusion shall preserve the raw `Detection` match.

The exclusion shall emit zero-point `LexicalCollisionExcluded` evidence.

Explicit language mode shall apply only exclusions for the selected language.

Automatic mode shall apply each exclusion by the matched HurtLex entry language.

An unsupported explicit language shall not enable automatic exclusions.

The first exclusion inventory shall contain these 28 language-term pairs:

```text
AR: فتاة, عامل, تجمع
DE: volk, rock, hund, dose
EN: do, people, know, love, proud
ES: gente, pueblo, policía
FR: gens, peuple, police
IT: popolo, sito, arte, gente
PT: gente, povo, polícia
RU: девушка, член, юбка
```

TextDetox labels shall never create a runtime exclusion.

Targets, groups, sentiment, threats, and active lexical matches shall remain independent of excluded lexical events.

Explicit German mode shall reactivate `hund` only when the normalized phrase is exactly `du hund`.

Automatic mode shall not apply the German reactivation.

## Identity attack rules

The detector shall treat `ps`, `rci`, `ddp`, `ddf`, and `om` as identity-related categories.

The HurtLex `stereotype=yes` flag shall support an identity event.

The stereotype flag shall never create an identity candidate by itself.

An active stereotype flag shall add zero-point `IdentityStereotypeSupport` evidence.

Identity metadata alone shall not create an identity attack.

An identity attack shall require a group target in the same clause.

A protected-group reply context shall also supply a group target.

An identity attack shall also require direct hostile syntax or negative sentiment support.

A direct relation shall match the complete token gap against a fixed language link phrase.

Whitespace-only adjacency shall support zero-copula Russian and Arabic syntax.

Caps and punctuation support shall not open the identity gate.

The report, quotation, counterspeech, and negation rules shall suppress identity escalation.

## Sentiment rules

Sentiment shall be a modifier only.

Negative terms shall add one support point.

An intensifier before a negative term shall add one extra support point.

A diminisher before a negative term shall remove one support point.

A negator before a sentiment term shall reverse its sign.

All-caps text shall add at most one support point.

Repeated terminal punctuation shall add at most one support point.

The sentiment support value shall stay between zero and eight.

Sentiment support alone shall never select `Review` or `Block`.

## Risk points and action

A conservative active HurtLex match shall assign 30 profanity points.

An inclusive HurtLex match shall assign 20 profanity points.

A confusable or evasion match shall add five profanity points to that candidate match.

A grouped event shall use the strongest complete candidate score.

Targeted abuse shall assign 70 points.

An identity attack shall assign 85 points.

A direct threat shall assign 95 points.

Sentiment support shall add at most five points to an active context category.

Every category shall clamp at 100 points.

Any direct threat or identity attack shall select `Block`.

Any targeted abuse or profanity score of at least 20 shall select `Review`.

No active toxic category shall select `Allow`.

## TextDetox data path

The CLI shall download selected TextDetox splits through the Hugging Face rows API.

The downloader shall read the current dataset revision before and after the row requests.

The downloader shall fail when the dataset revision changes during acquisition.

Each generated source ID shall contain the observed dataset revision.

The default TextDetox languages shall match the eight runtime packs when available.

The data parser shall accept all 15 source codes.

The `hin` source code shall map to `HINGLISH`.

The `hi` source code shall map to `HI`.

The importer shall preserve each source language and source row index.

The importer shall reject labels other than zero and one.

The importer shall canonicalize text with Charabia before duplicate grouping.

The duplicate grouping key shall contain the detector language and canonical text.

The importer shall remove exact duplicate groups with one shared label.

The importer shall remove complete groups that contain conflicting labels.

The importer shall assign one stable split per detector-language and canonical-text group.

The stable split shall hash the detector language, a NUL byte, and canonical text with fixed FNV-1a.

Hash buckets 0 through 69 shall select development.

Hash buckets 70 through 84 shall select validation.

Hash buckets 85 through 99 shall select test.

The hash split shall not claim stratification.

The importer shall write one TSV file per split.

Each evaluation row shall contain `language`, `label`, and `text`.

A separate provenance row shall retain every `source_id` and its canonical group mapping.

The evaluator shall accept an extra `source_id` field without changing binary metrics.

## Evaluation controls

The evaluator shall report the overall confusion matrix and each language matrix.

The evaluator shall predict toxic for `Review` and `Block` actions by default.

The evaluator shall allow `Block` as a stricter minimum action.

The evaluator shall never select thresholds from the test split.

Any row used to change a rule shall become audit-only and shall not support later quality metrics.

The documentation shall identify TextDetox source label differences and duplicate risks.

## Deferred work

The release shall not add stemming before per-language collision tests exist.

The release shall not enable large Japanese or Korean dictionaries by default.

The release shall not derive a production blocklist from TextDetox labels.

The release shall not claim broad language quality from one English sentiment probe.
