# Deterministic multilingual context detector implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic context, sentiment, and TextDetox evaluation to the existing multilingual Rust detector.

**Architecture:** Keep `Detector::check` as the lexical engine. Add typed text, rule-pack, policy, and TextDetox modules around it.

**Tech Stack:** Rust 2024, Charabia, Aho-Corasick, serde, csv, serde_json, reqwest, and thiserror.

**Spec:** `docs/superpowers/specs/2026-08-31-deterministic-multilingual-context-detector-design.md`

**Status:** Complete. The progress ledger records the test-driven fix rounds and final verification.

## Global constraints

The runtime shall not use model inference or translation.

The runtime shall use integer risk points and shall not call them probabilities.

The first context packs shall cover EN, ES, FR, DE, IT, PT, RU, and AR.

An unknown or automatic language shall use lexical evidence only.

TextDetox labels shall support evaluation only.

The implementation shall preserve `Detector::check` and `Detection.score`.

The directory has no Git repository. Workers shall not initialize one.

Every production change shall follow a failing-test and passing-test cycle.

---

## File map

- `src/text.rs`: token spans, clauses, quote scope, and candidate views.
- `src/detector.rs`: lexical matching across candidate views.
- `src/rule_pack.rs`: eight fixed native language packs.
- `src/policy.rs`: context events, sentiment support, risk points, and action selection.
- `src/textdetox.rs`: TextDetox parsing, provenance, grouping, splits, and TSV output.
- `src/workflow.rs`: policy evaluation orchestration.
- `src/main.rs`: policy output and TextDetox commands.
- `src/lib.rs`: public exports.
- `tests/text.rs`: normalized-view and span contracts.
- `tests/policy.rs`: policy behavior across context and languages.
- `tests/textdetox.rs`: dataset preparation contracts.
- `tests/workflow.rs`: binary policy evaluation.
- `tests/cli.rs`: user-visible policy output.
- `README.md`: commands, score meaning, data controls, and limits.

### Task 1: Text views and lexical evidence spans

**Files:**

- Create: `src/text.rs`
- Create: `tests/text.rs`
- Modify: `src/detector.rs`
- Modify: `src/lib.rs`

**Interfaces:**

- Produces: `CandidateViewKind`, `CandidateView`, `TextDocument`, and `TextSpan`.
- Produces: lexical match view and original byte span fields.
- Preserves: `normalize_text`, `Detector::check`, and `Detection.score`.

- [x] **Step 1: Write the failing candidate-view tests**

```rust
use toxcheck::{CandidateViewKind, TextDocument};

#[test]
fn preserves_original_byte_spans_in_the_normalized_view() {
    let document = TextDocument::new("ERES ESTÚPIDO");
    let view = document.view(CandidateViewKind::Normalized);

    assert_eq!(view.text(), "eres estupido");
    assert_eq!(view.original_span(5, 13), Some(5..14));
}

#[test]
fn joins_only_a_run_of_separated_single_letters() {
    let document = TextDocument::new("you are i.d.i.o.t, class stays whole");
    let view = document.view(CandidateViewKind::Evasion);

    assert_eq!(view.text(), "you are idiot class stays whole");
}

#[test]
fn maps_digits_only_inside_a_mixed_token() {
    let document = TextDocument::new("you are 1d10t and scored 101");
    let view = document.view(CandidateViewKind::Evasion);

    assert_eq!(view.text(), "you are idiot and scored 101");
}
```

- [x] **Step 2: Run the candidate-view tests and verify the missing API failure**

Run: `cargo test --test text`

Expected: compilation fails because `TextDocument` and `CandidateViewKind` do not exist.

- [x] **Step 3: Add the text API**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateViewKind {
    Normalized,
    Confusable,
    Evasion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}

pub struct CandidateView {
    text: String,
    segments: Vec<ViewSegment>,
}

pub struct TextDocument {
    original: String,
    tokens: Vec<TextToken>,
    normalized: CandidateView,
    confusable: CandidateView,
    evasion: CandidateView,
}
```

Build every view from Charabia tokens. Use token `byte_start` and `byte_end` values.

Map `0`, `1`, `3`, `4`, `5`, and `7` only when one token contains letters and digits.

Join a sequence only when it contains three or more single-letter tokens.

- [x] **Step 4: Run the candidate-view tests and verify they pass**

Run: `cargo test --test text`

Expected: three tests pass.

- [x] **Step 5: Write the failing lexical-span tests**

```rust
#[test]
fn reports_the_original_span_for_a_normalized_match() {
    let detector = detector("estúpido");
    let result = detector.check("ERES ESTÚPIDO");

    assert_eq!(result.matches[0].raw_start, 5);
    assert_eq!(result.matches[0].raw_end, 14);
    assert_eq!(result.matches[0].view, CandidateViewKind::Normalized);
}

#[test]
fn matches_a_separated_letter_evasion_candidate() {
    let detector = detector("idiot");
    let result = detector.check("i.d.i.o.t");

    assert!(result.is_match());
    assert_eq!(result.matches[0].view, CandidateViewKind::Evasion);
    assert_eq!(result.matches[0].raw_start, 0);
    assert_eq!(result.matches[0].raw_end, 9);
}
```

- [x] **Step 6: Run the lexical-span tests and verify the field failure**

Run: `cargo test --test detector`

Expected: compilation fails because the new match fields do not exist.

- [x] **Step 7: Match all three candidate views**

Add these fields to `LexiconMatch`.

```rust
pub view: CandidateViewKind,
pub normalized_start: usize,
pub normalized_end: usize,
pub raw_start: usize,
pub raw_end: usize,
```

Keep `matched_confusable_view` for source compatibility.

Deduplicate one entry at one original span. Prefer normalized, then confusable, then evasion evidence.

- [x] **Step 8: Run all lexical and text tests**

Run: `cargo test --test text --test detector`

Expected: all tests pass.

### Task 2: Native rule packs and policy analysis

**Files:**

- Create: `src/rule_pack.rs`
- Create: `src/policy.rs`
- Create: `tests/policy.rs`
- Modify: `src/text.rs`
- Modify: `src/detector.rs`
- Modify: `src/lib.rs`

**Interfaces:**

- Consumes: `TextDocument`, `TextSpan`, `CandidateViewKind`, and `Detection`.
- Produces: `AnalysisContext`, `ReplyTarget`, `PolicyAction`, `CategoryScores`, `RuleId`, `RuleEvidence`, and `PolicyResult`.
- Produces: `Detector::analyze(&self, text, context)`.

- [x] **Step 1: Write the failing policy contract tests**

```rust
#[test]
fn scores_a_second_person_insult() {
    let result = detector("idiot", "cds", false).analyze(
        "You are an idiot.",
        AnalysisContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 70);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(result.has_rule(RuleId::TargetedLexicalMatch));
}

#[test]
fn uses_a_reply_target_for_a_bare_insult() {
    let context = AnalysisContext {
        language: Some("EN"),
        reply_target: ReplyTarget::Person,
    };
    let result = detector("idiot", "cds", false).analyze("Idiot.", context);

    assert_eq!(result.scores.targeted_abuse, 70);
}

#[test]
fn blocks_a_direct_threat_without_a_lexical_match() {
    let result = detector("idiot", "cds", false).analyze(
        "I will kill you.",
        AnalysisContext::for_language("EN"),
    );

    assert!(!result.lexical.is_match());
    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
}

#[test]
fn keeps_a_negated_match_as_suppressed_evidence() {
    let result = detector("idiot", "cds", false).analyze(
        "You are not an idiot.",
        AnalysisContext::for_language("EN"),
    );

    assert!(result.lexical.is_match());
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn keeps_a_quoted_match_as_suppressed_evidence() {
    let result = detector("idiot", "cds", false).analyze(
        "\"You are an idiot,\" she said.",
        AnalysisContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::QuotedEvidence));
}

#[test]
fn suppresses_counterspeech_about_an_identity_slur() {
    let result = detector("vermin", "ps", true).analyze(
        "Do not call immigrants vermin.",
        AnalysisContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.identity_attack, 0);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn blocks_a_group_identity_attack() {
    let result = detector("vermin", "ps", true).analyze(
        "Immigrants are vermin.",
        AnalysisContext::for_language("EN"),
    );

    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);
}

#[test]
fn sentiment_support_never_creates_an_action() {
    let result = detector("idiot", "cds", false).analyze(
        "This service is awful!!!",
        AnalysisContext::for_language("EN"),
    );

    assert_eq!(result.scores.sentiment_support, 2);
    assert_eq!(result.action, PolicyAction::Allow);
}
```

- [x] **Step 2: Run the policy tests and verify the missing API failure**

Run: `cargo test --test policy`

Expected: compilation fails because the policy API does not exist.

- [x] **Step 3: Add the public policy types**

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReplyTarget { #[default] Unknown, Person, ProtectedGroup }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnalysisContext<'a> {
    pub language: Option<&'a str>,
    pub reply_target: ReplyTarget,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CategoryScores {
    pub profanity: u8,
    pub targeted_abuse: u8,
    pub identity_attack: u8,
    pub threat_language: u8,
    pub sentiment_support: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyAction { Allow, Review, Block }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyCategory {
    Profanity,
    TargetedAbuse,
    IdentityAttack,
    ThreatLanguage,
    SentimentSupport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleId {
    LexicalMatch,
    LexicalCollisionExcluded,
    TargetedLexicalMatch,
    ReplyTargetedLexicalMatch,
    DirectThreat,
    ThreatIntentMarker,
    IdentityGroupTarget,
    IdentityStereotypeSupport,
    NegatedEvidence,
    QuotedEvidence,
    ReportedEvidence,
    CounterspeechEvidence,
    NegativeSentiment,
    CapsSupport,
    PunctuationSupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleEvidence {
    pub rule_id: RuleId,
    pub category: PolicyCategory,
    pub points: u8,
    pub language: Option<String>,
    pub matched_text: String,
    pub candidate_view: Option<CandidateViewKind>,
    pub normalized_start: Option<usize>,
    pub normalized_end: Option<usize>,
    pub raw_start: Option<usize>,
    pub raw_end: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyResult {
    pub original_text: String,
    pub lexical: Detection,
    pub scores: CategoryScores,
    pub action: PolicyAction,
    pub evidence: Vec<RuleEvidence>,
}
```

Map every `RuleId` to the matching lowercase snake-case value for CLI output.

Add `PolicyResult::has_rule(RuleId) -> bool` and `PolicyResult::max_risk_points() -> u8`.

- [x] **Step 4: Add the eight rule packs**

Use this private type.

```rust
pub(crate) struct RulePack {
    pub language: &'static str,
    pub targets: Vec<Vec<String>>,
    pub groups: Vec<Vec<String>>,
    pub identity_links: Vec<Vec<String>>,
    pub negators: Vec<Vec<String>>,
    pub threats: Vec<Vec<String>>,
    pub intent: Vec<Vec<String>>,
    pub reports: Vec<Vec<String>>,
    pub counterspeech: Vec<Vec<String>>,
    pub positive: Vec<Vec<String>>,
    pub negative: Vec<Vec<String>>,
    pub intensifiers: Vec<Vec<String>>,
    pub diminishers: Vec<Vec<String>>,
}
```

Use EN, ES, FR, DE, IT, PT, RU, and AR fixtures in the source.

Return `None` for unknown language codes.

- [x] **Step 5: Implement clause-local policy rules**

Use four tokens for target relations. Use five tokens for threat relations.

Use three preceding tokens for negation. Use four preceding tokens for counterspeech.

Cap the combined suppressed profanity subtotal at 10 points per message.

Suppress a lexical event that overlaps a multi-token negator.

Add 28 fixed language-lemma collision exclusions for AR, DE, EN, ES, FR, IT, PT, and RU.

In explicit mode, apply only exclusions for the selected language.

In automatic mode, apply each exclusion by the matched HurtLex entry language.

Preserve raw matches and add zero-point `LexicalCollisionExcluded` evidence.

Copy candidate view and normalized offsets into every lexical-derived evidence item.

Keep rule-only candidate provenance empty.

Keep target, group, sentiment, threat, and active lexical rules independent from excluded lexical events.

Reactivate German `hund` only for the exact explicit-language phrase `du hund`.

Gate identity attacks with a direct relation, net negative lexical support, or a protected-group reply.

Keep caps and punctuation outside the identity gate.

Allow whitespace-only zero-copula identity syntax only for RU and AR.

Use TextDetox errors only as manual audit candidates. Never generate runtime exclusions from labels.

Mark each row that influences a rule as audit-only. Exclude that row from later quality metrics.

Use these named constants.

```rust
const CONSERVATIVE_PROFANITY: u8 = 30;
const INCLUSIVE_PROFANITY: u8 = 20;
const EVASION_BONUS: u8 = 5;
const SUPPRESSED_PROFANITY: u8 = 10;
const TARGETED_ABUSE: u8 = 70;
const IDENTITY_ATTACK: u8 = 85;
const THREAT_LANGUAGE: u8 = 95;
const MAX_SENTIMENT_SUPPORT: u8 = 8;
```

Treat only `ps`, `rci`, `om`, `ddf`, and `ddp` as identity candidates.

Require a group target or a protected-group reply before identity escalation.

Do not use `re` as a threat source. Do not use `svp` as a sentiment source.

- [x] **Step 6: Run the policy tests and verify they pass**

Include regression tests for combined suppression, negator overlap, collision scope, raw-match preservation, and eight-language positive controls.

Run: `cargo test --test policy`

Expected: all policy tests pass.

- [x] **Step 7: Add one threat fixture per installed language pack**

Use these literal messages.

```text
EN: I will kill you
ES: Te voy a matar
FR: Je vais te tuer
DE: Ich werde dich töten
IT: Ti ucciderò
PT: Eu vou matar você
RU: Я убью тебя
AR: سأقتلك أنت
```

Assert `threat_language >= 95` and `PolicyAction::Block` for every row.

- [x] **Step 8: Run the multilingual policy tests**

Run: `cargo test --test policy`

Expected: every language fixture passes.

### Task 3: TextDetox preparation and provenance

**Files:**

- Create: `src/textdetox.rs`
- Create: `tests/textdetox.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Consumes: `normalize_text` and `EvalRow`.
- Produces: `TextDetoxLanguage`, `TextDetoxSourceRow`, `DatasetSplit`, `PreparedTextDetox`, and provenance writers.
- Produces: `parse_textdetox_rows`, `parse_textdetox_page`, `textdetox_rows_url`, and `prepare_textdetox`.
- Produces: source, evaluation, and provenance TSV writers.

- [x] **Step 1: Write the failing language, duplicate, conflict, and split tests**

```rust
#[test]
fn keeps_hindi_and_hinglish_separate() {
    let rows = parse_textdetox_rows(Cursor::new(concat!(
        "source_id\tlanguage\ttoxic\ttext\n",
        "textdetox@rev/hi/1\thi\t0\tनमस्ते\n",
        "textdetox@rev/hin/1\thin\t1\ttu idiot hai\n",
    ))).expect("valid rows");

    assert_eq!(rows[0].language.detector_code(), "HI");
    assert_eq!(rows[1].language.detector_code(), "HINGLISH");
}

#[test]
fn deduplicates_normalized_text_and_preserves_provenance() {
    let rows = source_rows(&[
        ("b", "en", 1, "You are an IDIOT!"),
        ("a", "en", 1, "you are an idiot"),
    ]);
    let prepared = prepare_textdetox(&rows, &BTreeSet::from(["EN".to_owned()]))
        .expect("prepared rows");

    assert_eq!(prepared.summary.evaluation_rows, 1);
    assert_eq!(prepared.summary.duplicate_rows, 1);
    assert_eq!(prepared.provenance.len(), 2);
    assert_eq!(prepared.provenance[0].canonical_source_id.as_deref(), Some("a"));
}

#[test]
fn excludes_a_group_with_conflicting_labels() {
    let rows = source_rows(&[
        ("a", "en", 0, "Same text!"),
        ("b", "en", 1, "same text"),
    ]);
    let prepared = prepare_textdetox(&rows, &BTreeSet::from(["EN".to_owned()]))
        .expect("prepared rows");

    assert_eq!(prepared.summary.evaluation_rows, 0);
    assert_eq!(prepared.summary.conflicting_groups, 1);
}

#[test]
fn locks_the_fnv_split_contract() {
    assert_eq!(split_for_key("EN", "you are an idiot"), DatasetSplit::Development);
    assert_eq!(split_for_key("EN", "message 1"), DatasetSplit::Validation);
    assert_eq!(split_for_key("EN", "message 14"), DatasetSplit::Test);
}

#[test]
fn parses_a_rows_api_page_with_revision_source_ids() {
    let page = parse_textdetox_page(Cursor::new(concat!(
        "{\"rows\":[{\"row_idx\":7,\"row\":{\"text\":\"hello\",\"toxic\":0}}],",
        "\"num_rows_total\":5000,\"num_rows_per_page\":100,\"partial\":false}"
    )), "en", "abc123").expect("valid page");

    assert_eq!(page.rows[0].source_id, "textdetox@abc123/en/000007");
    assert_eq!(page.total_rows, 5000);
}
```

- [x] **Step 2: Run the TextDetox tests and verify the missing module failure**

Run: `cargo test --test textdetox`

Expected: compilation fails because the TextDetox API does not exist.

- [x] **Step 3: Add the TextDetox types and parsers**

Use these public types.

```rust
pub const TEXTDETOX_PREPARATION_VERSION: &str = "v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextDetoxLanguage {
    Amharic,
    Arabic,
    German,
    English,
    Spanish,
    French,
    Hebrew,
    Hindi,
    Hinglish,
    Italian,
    Japanese,
    Russian,
    Tatar,
    Ukrainian,
    Chinese,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatasetSplit { Development, Validation, Test }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDetoxSourceRow {
    pub source_id: String,
    pub language: TextDetoxLanguage,
    pub label: EvalLabel,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceStatus {
    Representative,
    Duplicate,
    LabelConflict,
    UnsupportedLanguage,
    EmptyText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceRow {
    pub source_id: String,
    pub source_language: String,
    pub detector_language: String,
    pub group_id: Option<String>,
    pub split: Option<DatasetSplit>,
    pub canonical_source_id: Option<String>,
    pub status: ProvenanceStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextDetoxSummary {
    pub source_rows: usize,
    pub evaluation_rows: usize,
    pub duplicate_rows: usize,
    pub conflicting_groups: usize,
    pub unsupported_rows: usize,
    pub empty_rows: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTextDetox {
    pub development: Vec<EvalRow>,
    pub validation: Vec<EvalRow>,
    pub test: Vec<EvalRow>,
    pub provenance: Vec<ProvenanceRow>,
    pub summary: TextDetoxSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTextDetoxPage {
    pub rows: Vec<TextDetoxSourceRow>,
    pub total_rows: usize,
}
```

Accept only `am ar de en es fr he hi hin it ja ru tt uk zh`.

Map `hin` to `HINGLISH`. Map every other code to its uppercase detector code.

Reject blank or repeated source IDs. Reject labels outside zero and one.

Add `serde_json = "1.0"` for rows API pages.

- [x] **Step 4: Implement exact grouped preparation**

Group on `<detector-language><NUL><normalized-text>`.

Use trimmed raw text when normalization produces no tokens.

Choose the smallest source ID as the representative.

Drop a complete group when labels conflict.

Use FNV-1a 64-bit. Use 70, 15, and 15 percent split buckets from the specification.

Write group IDs as `v1-` plus 16 lowercase hexadecimal digits.

- [x] **Step 5: Implement the four TSV writers**

Write `development.tsv`, `validation.tsv`, and `test.tsv` with this header.

```text
language<TAB>label<TAB>text
```

Write `provenance.tsv` with source ID, language codes, group ID, split, representative ID, and status.

- [x] **Step 6: Run the TextDetox tests and verify they pass**

Run: `cargo test --test textdetox`

Expected: all TextDetox tests pass.

### Task 4: Policy evaluation and CLI integration

**Files:**

- Modify: `src/workflow.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `tests/workflow.rs`
- Modify: `tests/cli.rs`

**Interfaces:**

- Consumes: `Detector::analyze`, `PolicyAction`, and prepared TextDetox rows.
- Produces: `evaluate_policy(rows, entries, minimum_action)`.
- Produces: `fetch-textdetox`, `prepare-textdetox`, policy `check`, and policy `eval` commands.

- [x] **Step 1: Write the failing policy evaluation test**

```rust
#[test]
fn policy_evaluation_detects_a_threat_without_hurtlex_support() {
    let rows = vec![EvalRow {
        language: "EN".to_owned(),
        label: EvalLabel::Toxic,
        text: "I will kill you".to_owned(),
    }];
    let entries = vec![entry("EN", "idiot")];

    let report = evaluate_policy(&rows, entries, PolicyAction::Review)
        .expect("policy evaluation");

    assert_eq!(report.overall.true_positive, 1);
}
```

- [x] **Step 2: Run the workflow test and verify the missing function failure**

Run: `cargo test --test workflow policy_evaluation_detects_a_threat_without_hurtlex_support`

Expected: compilation fails because `evaluate_policy` does not exist.

- [x] **Step 3: Add policy evaluation**

Build one detector per row language. Pass the row language through `AnalysisContext`.

Count `Review` and `Block` when the minimum action is `Review`.

Count only `Block` when the minimum action is `Block`.

Keep the legacy `evaluate` function.

- [x] **Step 4: Write the failing CLI policy-output test**

```rust
assert!(stdout.contains("prediction=toxic score=1.000 action=review"));
assert!(stdout.contains("category=targeted_abuse points=70"));
assert!(stdout.contains("rule=targeted_lexical_match"));
```

- [x] **Step 5: Run the CLI test and verify the output failure**

Run: `cargo test --test cli check_prints_the_prediction_and_match`

Expected: the command succeeds, but the policy output assertions fail.

- [x] **Step 6: Add policy fields to `check` and switch `eval` to policy evaluation**

Keep `score` as the legacy lexical score.

Print action, maximum risk points, every category value, and every rule evidence row.

Add `--reply-target unknown|person|protected-group` to `check`.

Add `--minimum-action review|block` to `eval`.

Pass no language pack when `--language auto` is active.

- [x] **Step 7: Add `prepare-textdetox`**

Accept `--input`, `--output-dir`, and `--languages`.

Default `--languages` to `EN,ES,FR,DE,IT,RU,AR`.

Read the acquisition TSV. Write the three evaluation files and the provenance file.

Print source, duplicate, conflict, excluded, and output counts.

- [x] **Step 8: Add `fetch-textdetox`**

Accept `--output`, `--languages`, and optional `--max-rows`.

Default `--languages` to `en,es,fr,de,it,ru,ar`.

Read the dataset `sha` from the Hugging Face dataset API before and after acquisition.

Fail when the two revision values differ.

Fetch at most 100 rows per rows API request.

Write the acquisition TSV with this header.

```text
source_id<TAB>language<TAB>toxic<TAB>text
```

Use `textdetox@<sha>/<language>/<row-index>` for each source ID.

- [x] **Step 9: Run workflow and CLI tests**

Run: `cargo test --test workflow --test cli`

Expected: all workflow and CLI tests pass.

### Task 5: Documentation and full verification

**Files:**

- Modify: `README.md`

**Interfaces:**

- Documents: policy output, collision exclusions, eight packs, lexical fallback, TextDetox acquisition, split controls, and known limits.

- [x] **Step 1: Document the runnable policy examples**

Document one insult, one threat, one quote, and one Spanish message.

State that translation services and neural sentiment tools are outside the runtime.

State that the score is not a probability.

- [x] **Step 2: Document the TextDetox acquisition TSV**

Use this exact header.

```text
source_id<TAB>language<TAB>toxic<TAB>text
```

State the 15 accepted source codes. State that `hin` means Hinglish.

State that exact normalized duplicates cannot cross splits.

State that paraphrases and copied templates can still cross splits.

State that binary labels cannot measure category accuracy.

- [x] **Step 3: Format the project**

Run: `cargo fmt --all -- --check`

Expected: exit code zero.

- [x] **Step 4: Run Clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Expected: exit code zero.

- [x] **Step 5: Run the full test suite**

Run: `cargo test --all-targets --all-features`

Expected: every test passes.

- [x] **Step 6: Run live CLI checks**

Run: `cargo run --quiet -- check --language EN --text "I will kill you"`

Expected: `action=block` and `threat_language=95` or more.

Run: `cargo run --quiet -- check --language ES --text "Te voy a matar"`

Expected: `action=block` and `threat_language=95` or more.

Run: `cargo run --quiet -- check --language EN --text "Do not call people idiots"`

Expected: the lexical evidence remains and the action is `allow`.

Run: `cargo run --quiet -- fetch-textdetox --languages en --max-rows 10 --output /tmp/toxcheck-textdetox.tsv`

Expected: the command writes ten source rows and one header row.
