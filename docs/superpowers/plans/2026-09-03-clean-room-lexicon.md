# Clean-Room Lexicon Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the HurtLex rule channel in all 15 languages with independently sourced lexica, so the shipped package carries no NonCommercial term.

**Architecture:** Each language gets a lexicon built from four permissive sources, in priority order: its own native Wiktionary, the textdetox multilingual toxic lexicon, English Wiktionary, and the flat lists LDNOOBW and washyourmouth. The repo corpus supplies a fifth stream of mined candidates. A committed sense table assigns the 17 categories per lemma. A new `blasphem-train lexicon` subcommand harvests and merges deterministically. Each language passes only when the recompiled model matches or beats its current validation F1.

HurtLex is a reference throughout. Open it to check what a word means. Do not copy a category assignment from it.

**Tech Stack:** Rust 2024 (`blasphem-train`), `reqwest` blocking, `csv`, `sha2`, `serde_json`. No new dependencies.

**Spec:** None written. The method is proven end to end on Spanish. The reviewed output is `data/clean-room-v1/ES.tsv` and the measured result is in Task 0 below. Treat the ES artifacts as the executable spec.

## Global Constraints

- Output schema is the existing six columns, tab separated, with a header row: `id`, `pos`, `category`, `stereotype`, `lemma`, `level`.
- `id` is the storage code followed by five digits, zero padded, for example `ES00001`. `src/lexicon.rs:81` reads the language from the leading alphabetic characters. Any other prefix is rejected at parse time.
- `pos` is one of `n`, `a`, `v`, `r`, `i`, `p`, `x`. No consumer reads it. Fill it, do not agonise over it.
- `category` is one of the 17 codes: `ps`, `rci`, `pa`, `ddf`, `ddp`, `dmc`, `is`, `or`, `an`, `asm`, `asf`, `pr`, `om`, `qas`, `cds`, `re`, `svp`.
- `stereotype` is `yes` or `no`. Set `yes` when the category is `ps`, `rci`, `om`, `ddf`, or `ddp`.
- `level` is `conservative` or `inclusive`. Use `inclusive` when the lemma has a common neutral sense.
- `src/policy.rs:552` reads `ps`, `rci`, `om`, `ddf`, `ddp` to classify identity attacks. Those five carry the product behaviour. Do not leave them empty.
- Never read `data/raw-v1/hurtlex/` to decide a lemma or a category. Consulting it to check a fact is allowed. Copying an assignment is not.
- Storage code is not always the language code. `Language::Ms` has code `MS` and storage code `ID` (`src/language.rs:113` and `:131`). Files are named by storage code.
- Every source must be recorded in `resources/datasets/source-lock-v1.json` with its URL, revision, SHA-256, and licence before it is used.
- Sizes in reports are megabytes with two decimals.

---

## Task 0: Baseline and gate table

The gate for every language is its current validation F1. Beat it or match it.

| Language | Storage | Current F1 | HurtLex rows | Difficulty |
|---|---|---:|---:|---|
| TR | TR | 0.0516 | 2,349 | easiest |
| ZH | ZH | 0.0674 | 4,251 | easiest |
| JA | JA | 0.3730 | 8,428 | easy |
| IT | IT | 0.3769 | 6,940 | easy |
| ES | ES | 0.4228 → **0.5476** | 5,006 | **done** |
| PT | PT | 0.4895 | 3,901 | moderate |
| AR | AR | 0.5343 | 3,220 | moderate |
| DE | DE | 0.5543 | 5,039 | moderate |
| KO | KO | 0.5738 | 2,267 | moderate |
| MS | ID | 0.5783 | 3,586 | moderate |
| VI | VI | 0.5854 | 2,031 | moderate |
| HI | HI | 0.7471 | 2,209 | hard |
| EN | EN | 0.7496 | 8,228 | hard |
| RU | RU | 0.8604 | 4,679 | hardest |
| FR | FR | 0.9108 | 5,024 | hardest |

Numbers come from `validation_metrics.f1` in `resources/models/multilingual-v2/manifest.json`.

**The gate does not measure categories.** `src/detector.rs:261-267` scores a match from `entry.level` and the view kind. Category never reaches the nudge decision; it feeds `src/policy.rs:552`, which classifies the kind of event afterwards. So F1 is a lemma-coverage test. Identity categories are a separate, unmeasured requirement that the gate will happily pass without.

Treat them as two deliverables per language. Clear the gate with coverage. Fill `ps`, `rci`, `om`, `ddf`, `ddp` because the product needs them, and verify that work by reading the file, not by reading F1.

Spanish is finished. 1,493 entries against 5,006 HurtLex rows, and it moved F1 from 0.4228 to 0.5476 with 41 more true positives for one extra false positive.

Work the table top down. TR and ZH are near zero, so almost anything beats them and they prove the pipeline cheaply.

Difficulty here is the gate height, not the odds of clearing it. Read it together with the source matrix at the end. FR and RU sit at the top of the gate column but have the deepest supply in the set, 9,770 native Wiktionary entries and 141,000 textdetox entries. VI and MS sit mid-table on the gate and have the thinnest supply. Those two are the real risk.

- [ ] **Step 1: Record the gate table**

```bash
python3 -c "
import json
d=json.load(open('resources/models/multilingual-v2/manifest.json'))
for e in sorted(d['entries'], key=lambda e: e['validation_metrics']['f1']):
    print(e['language'], round(e['validation_metrics']['f1'],4))
" > /tmp/gate-table.txt
cat /tmp/gate-table.txt
```

Expected: 15 lines, ES at 0.4228, FR highest at 0.9108.

- [ ] **Step 2: Commit the plan**

```bash
git add docs/superpowers/plans/2026-09-03-clean-room-lexicon.md
git commit -m "Plan the clean-room lexicon rebuild"
```

---

## Task 1: Add the `lexicon-harvest` subcommand

Harvest is a network step. Isolate it so every later step is offline and reproducible.

**Files:**
- Create: `crates/blasphem-train/src/lexicon.rs`
- Modify: `crates/blasphem-train/src/lib.rs` (add `pub mod lexicon;`)
- Modify: `crates/blasphem-train/src/main.rs:55-67` (add the subcommand)

**Interfaces:**
- Produces: `harvest(options: &HarvestOptions) -> Result<HarvestReport, LexiconError>`, writing `{output}/{STORAGE}.harvest.json`.
- Consumes: nothing from earlier tasks.

- [ ] **Step 1: Define the harvest module**

```rust
//! Wiktionary lexicon harvest. Network in, JSON out. No assignment happens here.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const USER_AGENT: &str = "blasphem-lexicon-build/1.0 (https://github.com/sospedra/blasphem)";

/// The English Wiktionary category suffixes that mark a lemma as offensive.
const OFFENCE_SUFFIXES: &[&str] = &[
    "ethnic slurs",
    "anti-LGBTQ slurs",
    "religious slurs",
    "vulgarities",
    "swear words",
    "derogatory terms",
    "offensive terms",
    "dysphemisms",
];

/// The suffixes whose members are offensive in every sense.
const STRONG_SUFFIXES: &[&str] = &[
    "ethnic slurs",
    "anti-LGBTQ slurs",
    "religious slurs",
    "vulgarities",
    "swear words",
];

/// One wiki to harvest. The native wiki names its categories in its own
/// language, so the titles are passed in whole rather than built from a
/// suffix list. See the source matrix for the verified titles per language.
#[derive(Debug, Clone)]
pub struct WikiSource {
    /// Host, for example "fr.wiktionary.org" or "en.wiktionary.org".
    pub host: String,
    /// Full category titles to pull. Empty means derive them from
    /// OFFENCE_SUFFIXES, which only works on en.wiktionary.org.
    pub categories: Vec<String>,
    /// Titles that mark an unambiguous slur, a subset of `categories`.
    pub strong: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HarvestOptions {
    /// English name of the language, used only for en.wiktionary.org prefixes.
    pub language_name: String,
    pub storage_code: String,
    /// Native wiki first, en.wiktionary.org second. Order is priority order.
    pub wikis: Vec<WikiSource>,
    pub output: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Harvest {
    pub language_name: String,
    pub storage_code: String,
    pub offence: BTreeMap<String, Vec<String>>,
    pub topic: BTreeMap<String, Vec<String>>,
    pub lemma_categories: BTreeMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct HarvestReport {
    pub lemmas: usize,
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    #[error("wiktionary request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("cannot read or write {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse the wiktionary response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("the harvest for {0} is empty")]
    Empty(String),
}

pub fn harvest(options: &HarvestOptions) -> Result<HarvestReport, LexiconError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()?;
    let categories = all_categories(&client, &options.language_name)?;
    let mut offence = BTreeMap::new();
    for suffix in OFFENCE_SUFFIXES {
        let title = format!("Category:{} {suffix}", options.language_name);
        if !categories.contains(&title) {
            continue;
        }
        offence.insert((*suffix).to_owned(), category_members(&client, &title)?);
    }
    let mut lemmas: Vec<String> = offence.values().flatten().cloned().collect();
    lemmas.sort();
    lemmas.dedup();
    if lemmas.is_empty() {
        return Err(LexiconError::Empty(options.storage_code.clone()));
    }
    let topic = topic_categories(&client, &options.language_name, &categories)?;
    let lemma_categories = lemma_category_map(&client, &lemmas)?;
    let harvest = Harvest {
        language_name: options.language_name.clone(),
        storage_code: options.storage_code.clone(),
        offence,
        topic,
        lemma_categories,
    };
    let bytes = serde_json::to_vec_pretty(&harvest)?;
    let path = options
        .output
        .join(format!("{}.harvest.json", options.storage_code));
    fs::create_dir_all(&options.output).map_err(|source| LexiconError::Io {
        path: options.output.clone(),
        source,
    })?;
    fs::write(&path, &bytes).map_err(|source| LexiconError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(HarvestReport {
        lemmas: lemmas.len(),
        sha256: hex(&Sha256::digest(&bytes)),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
```

Four helpers remain. Each is a paged GET that follows the `continue` token until the response omits it, then returns the accumulated titles.

```rust
/// Every category title in namespace 14 whose name starts with the language.
/// Query: list=allpages&apnamespace=14&apprefix={language}&aplimit=500
/// Continue key: apcontinue. Spanish returns 3,589 titles across eight pages.
fn all_categories(client: &Client, language: &str) -> Result<BTreeSet<String>, LexiconError>;

/// Main-namespace members of one category.
/// Query: list=categorymembers&cmtitle={title}&cmnamespace=0&cmlimit=500
/// Continue key: cmcontinue.
fn category_members(client: &Client, title: &str) -> Result<Vec<String>, LexiconError>;

/// Members of the topic categories, keyed by bare topic name such as "es:Demonyms".
/// Selects from `categories` every title matching "Category:{code}:*", then calls
/// category_members on each. Skip topics with zero members.
fn topic_categories(
    client: &Client,
    language: &str,
    categories: &BTreeSet<String>,
) -> Result<BTreeMap<String, Vec<String>>, LexiconError>;

/// Per-lemma category membership, for part of speech and topic lookup.
/// Query: prop=categories&cllimit=max&titles={twelve titles joined by |}
/// Continue key: the whole `continue` object, merged into the next query.
/// Keep only categories starting with "{language} " or "{code}:".
/// Twelve titles per request is deliberate. Larger batches exhaust the
/// category budget mid-response and silently truncate.
fn lemma_category_map(
    client: &Client,
    lemmas: &[String],
) -> Result<BTreeMap<String, Vec<String>>, LexiconError>;
```

Rate limiting is real and it bites. Wikimedia returns HTTP 429 under parallel load and the block persists for minutes after the burst stops. Issue every request serially, retry up to four times with a three second delay, and treat an empty body as a retryable failure rather than an empty result. A silent empty result is how a harvest ends up with zero lemmas and a passing exit code.

- [ ] **Step 2: Wire the subcommand**

```rust
#[derive(Debug, Args)]
struct LexiconHarvestArgs {
    #[arg(long)]
    language_name: String,
    #[arg(long)]
    storage_code: String,
    #[arg(long)]
    output: PathBuf,
}
```

Add `LexiconHarvest(LexiconHarvestArgs)` to the `Command` enum at `crates/blasphem-train/src/main.rs:55-67`. Print `status=harvested language={code} lemmas={n} sha256={sha}` on success, matching the existing one-line status convention used by `compile` and `evaluate`.

- [ ] **Step 3: Run it against Spanish and compare**

```bash
mkdir -p /tmp/harvest
cargo run --release -p blasphem-train -- lexicon-harvest \
  --language-name Spanish --storage-code ES --output /tmp/harvest
```

Expected: `status=harvested language=ES lemmas=1360 sha256=...`. The lemma count must land within five percent of 1,360. A large miss means the category list or the paging is wrong.

- [ ] **Step 4: Commit**

```bash
git add crates/blasphem-train/src/lexicon.rs crates/blasphem-train/src/lib.rs crates/blasphem-train/src/main.rs
git commit -m "Add the wiktionary lexicon harvest command"
```

---

## Task 2: Add the `lexicon-build` subcommand

Build is offline. Same inputs always produce the same bytes.

**Files:**
- Modify: `crates/blasphem-train/src/lexicon.rs`
- Modify: `crates/blasphem-train/src/main.rs`

**Interfaces:**
- Consumes: `{harvest}/{STORAGE}.harvest.json` from Task 1, deserialised into the `Harvest` struct defined in Task 1.
- Produces: `build(options: &BuildOptions) -> Result<BuildReport, LexiconError>`, writing `data/clean-room-v1/{STORAGE}.tsv` and `data/clean-room-v1/{STORAGE}.provenance.json`.

```rust
#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub harvest_root: PathBuf,
    pub storage_code: String,
    pub output: PathBuf,
}

#[derive(Debug)]
pub struct BuildReport {
    pub entries: usize,
    pub identity_entries: usize,
    pub sha256: String,
}

/// One emitted row. Column order matches the six-column schema.
#[derive(Debug, Clone, Serialize)]
pub struct LexiconRow {
    pub id: String,
    pub pos: String,
    pub category: String,
    pub stereotype: String,
    pub lemma: String,
    pub level: String,
}
```

`identity_entries` counts rows whose category is `ps`, `rci`, `om`, `ddf`, or `ddp`. Print it in the status line so a thin build is visible without opening the file.

- [ ] **Step 1: Define the sense table format**

Create `data/clean-room-v1/{STORAGE}.senses.tsv`, two columns, tab separated, with a header:

```
lemma	category
sudaca	ps
bollera	om
cegatón	ddf
gilipollas	ddp
```

This file is the human contribution. It is committed, reviewed, and diffable. A lemma absent from the table falls through to the topic rules, then to the default.

- [ ] **Step 2: Implement the assignment order**

Apply in this order and stop at the first match:

1. The sense table, if the lemma appears in it.
2. Topic intersection. A lemma in an offence category and in `{code}:Ethnicity` or `{code}:Nationalities` is `ps`. In `{code}:Demonyms` it is `rci`. In `{code}:LGBTQ` or `{code}:Sexual orientations` it is `om`. In `{code}:Disability` it is `ddp`. In `{code}:Male genitalia` it is `asm`, `{code}:Female genitalia` is `asf`, plain `{code}:Genitalia` is `asm`. `{code}:Prostitution` is `pr`. `{code}:Crime` is `re`. `{code}:Religion` is `svp`. `{code}:Occupations` and `{code}:Military` are `pa`. Politics, nationalism, and conspiracy topics are `dmc`. Animal topics are `an`. Plant topics are `or`.
3. Membership alone. `ethnic slurs` is `ps`. `anti-LGBTQ slurs` is `om`. `derogatory terms` is `cds`. Everything else is `qas`.

Set `level` to `conservative` when the lemma is in a strong suffix category or came from LDNOOBW or washyourmouth, `inclusive` otherwise. Set `stereotype` to `yes` when the category is one of the five identity codes.

Derive `pos` from the Wiktionary part-of-speech categories, falling back to morphology: a space gives `p`, a verb ending gives `v`, an adverb ending gives `r`, otherwise `n`.

Emit rows sorted by `(category, lemma)` and number the ids from one in that order.

- [ ] **Step 3: Rebuild Spanish and diff against the reviewed file**

```bash
cp /tmp/es-senses.tsv data/clean-room-v1/ES.senses.tsv
cargo run --release -p blasphem-train -- lexicon-build \
  --harvest /tmp/harvest --storage-code ES --output data/clean-room-v1
git diff --stat data/clean-room-v1/ES.tsv
```

Expected: no diff. The tool must reproduce the reviewed Spanish file exactly. If it does not, the assignment order is wrong. Fix the tool, not the file.

The 479 Spanish sense assignments already exist. Extract them from the current file before this step:

```bash
python3 -c "
import csv
rows=csv.DictReader(open('data/clean-room-v1/ES.tsv'),delimiter='\t')
out=[(r['lemma'],r['category']) for r in rows if r['category'] not in ('qas','cds')]
with open('/tmp/es-senses.tsv','w') as f:
    f.write('lemma\tcategory\n')
    for l,c in sorted(out): f.write(f'{l}\t{c}\n')
print('sense rows:',len(out))
"
```

- [ ] **Step 4: Commit**

```bash
git add crates/blasphem-train/src/lexicon.rs crates/blasphem-train/src/main.rs data/clean-room-v1/ES.senses.tsv
git commit -m "Add the offline lexicon build command"
```

---

## Task 3: Per-language build, one task per language

Repeat this task fourteen times, in the Task 0 difficulty order: TR, ZH, JA, IT, PT, AR, DE, KO, MS, VI, HI, EN, RU, FR.

Substitute `{NAME}` (the English Wiktionary language name), `{CODE}` (the storage code), and `{GATE}` (the current F1).

Language names for the API: Turkish, Chinese, Japanese, Italian, Portuguese, Arabic, German, Korean, Indonesian, Vietnamese, Hindi, English, Russian, French. Note that Indonesian harvests into storage code `ID`.

**Files:**
- Create: `data/clean-room-v1/{CODE}.senses.tsv`
- Create: `data/clean-room-v1/{CODE}.tsv`
- Create: `data/clean-room-v1/{CODE}.provenance.json`

- [ ] **Step 1: Harvest**

```bash
cargo run --release -p blasphem-train -- lexicon-harvest \
  --language-name {NAME} --storage-code {CODE} --output /tmp/harvest
```

Expected: a non-zero lemma count. Arabic and Hindi will be small. Arabic returned 61 lemmas across `derogatory terms` and `ethnic slurs` in the survey. If the count is under 200, note it and continue. The other two sources carry the rest.

- [ ] **Step 2: Pull the textdetox lexicon**

Covers RU, ZH, EN, FR, IT, ES, AR, DE, JA, HI. Skip for TR, PT, KO, VI, MS.

```bash
python3 -c "
import urllib.request,json,io
code={'MS':None,'ZH':'zh','JA':'ja','AR':'ar','RU':'ru','DE':'de','FR':'fr',
      'IT':'it','HI':'hi','EN':'en','ES':'es'}.get('{CODE}')
if not code: raise SystemExit('no textdetox split for {CODE}')
u=f'https://huggingface.co/api/datasets/textdetox/multilingual_toxic_lexicon/parquet/{code}/train/0.parquet'
urllib.request.urlretrieve(u,'/tmp/harvest/{CODE}.textdetox.parquet')
print('downloaded')
"
python3 -c "
import pyarrow.parquet as pq
t=pq.read_table('/tmp/harvest/{CODE}.textdetox.parquet')
col=t.column_names[0]
rows=[str(v) for v in t.column(col).to_pylist() if str(v).strip()]
open('/tmp/harvest/{CODE}.textdetox.txt','w').write('\n'.join(rows))
print('{CODE} textdetox entries:',len(rows))
"
```

Expected counts: RU 141,000, ZH 3,840, EN 3,390, FR 1,290, IT 815, ES 430, AR 430, JA 328, DE 247, HI 133. A wildly different number means the split name changed.

Record the parquet SHA-256 in the source lock. The licence is OpenRAIL++, which is use-restricted rather than OSI. `corpus/*.tsv` already comes from the same project.

- [ ] **Step 3: Merge the flat sources**

```bash
curl -sL -o /tmp/harvest/{CODE}.ldnoobw.txt \
  "https://raw.githubusercontent.com/LDNOOBW/List-of-Dirty-Naughty-Obscene-and-Otherwise-Bad-Words/master/$(echo {CODE} | tr A-Z a-z)"
curl -sL "https://raw.githubusercontent.com/thisandagain/washyourmouthoutwithsoap/develop/data/build.json" \
  | python3 -c "
import sys,json
d=json.load(sys.stdin); c='$(echo {CODE} | tr A-Z a-z)'
open('/tmp/harvest/{CODE}.wymo.txt','w').write('\n'.join(d.get(c,[])))
print(c,len(d.get(c,[])))
"
```

LDNOOBW has no `id` and no `vi`. washyourmouth has no `ar` and no `zh`. Expect one of the two files to be empty for MS, VI, AR, and ZH. That is known, not a failure.

- [ ] **Step 4: Mine the corpus for candidates**

Two tokenisers. Whitespace languages split on word boundaries. ZH and JA emit character n-grams instead, because the runtime never tokenises them either. `src/rules/packs/cjk.rs:33` sets `RuleMatchProfile::CompactClauses` and its phrase sets are raw substrings such as `畜生` and `去死`. Chargrams are what the matcher consumes, so mine chargrams.

```bash
python3 -c "
import csv,re,unicodedata
from collections import Counter
CJK = '{CODE}' in ('ZH','JA')
def units(s):
    s=unicodedata.normalize('NFC',s)
    if not CJK: return set(re.findall(r'\w{3,}', s.lower(), re.UNICODE))
    t=re.sub(r'[\s\W_]+','',s)
    return {t[i:i+n] for n in (2,3,4) for i in range(len(t)-n+1)}
tox,cln=Counter(),Counter()
for r in csv.DictReader(open('corpus/{CODE}.tsv'),delimiter='\t'):
    (tox if r['label']=='toxic' else cln).update(units(r['text']))
floor = 6 if CJK else 4
out=[((t+1)/(cln[w]+1),t,cln[w],w) for w,t in tox.items() if t>=floor]
out=[o for o in out if o[0]>=6]; out.sort(reverse=True)
open('/tmp/harvest/{CODE}.corpus.txt','w').write('\n'.join(w for _,_,_,w in out))
print('candidates:',len(out))
"
```

Spanish produced 314 candidates with real signal (`mierda`, `subnormales`, `zorra`) and real noise (`crees`, `aqui`, `cara`). Expect the CJK runs to be noisier still, because overlapping n-grams surface substrings of a single real term. Keep the longest span when two candidates nest.

Treat the file as a review queue, not as input. Only promote a candidate into the sense table after reading it.

Now run the second miner. The ratio miner above cannot find identity slurs, because a nationality or a religion appears constantly in neutral text and its ratio collapses below the threshold. `cina` scores 2.5 overall and `onta` scores 2.3. Both are live slurs.

Recover them with a syntactic frame. Both Vietnamese and Indonesian mark person-reference with a fixed head word, so score the noun by how it behaves after that head rather than by how it behaves everywhere.

```bash
python3 -c "
import csv,re,unicodedata
from collections import Counter
HEADS={'VI':['thằng','bọn','lũ','đám','đồ','tụi'],
       'ID':['dasar','si','orang','kaum','bangsa'],
       'ES':['puto','puta','maldito','pinche'],
       'EN':['fucking','damn','dirty','filthy']}.get('{CODE}',[])
if not HEADS: raise SystemExit('no head list for {CODE}')
heads=set(HEADS)
ft,fc=Counter(),Counter()
for r in csv.DictReader(open('corpus/{CODE}.tsv'),delimiter='\t'):
    toks=re.findall(r'\w+',unicodedata.normalize('NFC',r['text']).lower(),re.UNICODE)
    sink = ft if r['label']=='toxic' else fc
    for a,b in zip(toks,toks[1:]):
        if a in heads and len(b)>1: sink[b]+=1
out=[(ft[w]/(fc[w]+1),ft[w],fc[w],w) for w in ft if ft[w]>=3]
out.sort(reverse=True)
open('/tmp/harvest/{CODE}.frames.txt','w').write('\n'.join(w for _,_,_,w in out))
print('frame candidates:',len(out))
for r,t,c,w in out[:20]: print(f'  {w:18} tox={t:<4} clean={c:<4} ratio={r:.1f}')
"
```

Rank by frame ratio, never by raw frame count. Ranking by count fills the Vietnamese output with `này`, `nào`, and `gì`.

Drop ambiguous heads. Vietnamese `con` and `cái` classify objects as well as people, so they pull in demonstratives. `thằng`, `bọn`, `lũ`, `đám`, and `đồ` are unambiguous.

Measured on the current corpora, the frame miner surfaced `bani` at 17 toxic frames and 0 clean, `cina` at 12 and 3, and `onta` at 12 and 0 for Indonesian, and `bóng` at 26 and 0 for Vietnamese. The ratio miner rejected every one of them. These are `ps`, `svp`, and `om` entries, which is the exact category the gap sits in.

The frame miner will not find `ddf` or `ddp`. Disability slurs take no distinctive frame in either language.

**A frame ratio is not a dominant-sense verdict, and the two must not be conflated.** The frame miner measures how a word behaves *after an identity head word*. It says nothing about how that word behaves everywhere else. Malay found `onta` at 12 toxic frames to 0 clean and promoted it to `conservative` on the strength of that, having concluded it had no competing everyday sense — but `unta` is the standard Indonesian and Malay word for camel, and a word-boundary count over the whole corpus gives 152 toxic against 67 clean, where the clean hits are ordinary camel and Hajj references: `naik onta`, `burung onta` meaning ostrich, `Nabi Muhammad`.

Both readings were correct about different things. A frame candidate still has to clear Step 5b's two questions on its own, over the full corpus, before it earns `conservative`.

- [ ] **Step 5: Write the sense table**

Build `data/clean-room-v1/{CODE}.senses.tsv` by reading the harvested lemmas and assigning each one a category from its meaning. This is the slow step and it does not automate.

Priority order when time is limited. The five identity codes carry the product behaviour through `src/policy.rs:552`, so assign `ps`, `rci`, `om`, `ddf`, and `ddp` first. Everything else can sit in `cds` or `qas` without breaking the policy path.

Spanish landed 280 identity entries out of 1,493. Aim for a similar share.

You may open `data/raw-v1/hurtlex/{CODE}/1.2/hurtlex_{CODE}.tsv` to check what a word means. Do not copy its category for a lemma. The Spanish check measured 27.5 percent category agreement on the overlap, which is what independent assignment looks like.

- [ ] **Step 5b: Enumerate every conservative identity row**

Mandatory. Not a spot-check, not a sample. Take every row whose category is `ps`, `rci`, `om`, `ddf`, or `ddp` and whose level is `conservative`, and for each one ask a single question: does the bare lemma have a dominant everyday sense?

```bash
awk -F'\t' 'NR>1 && $6=="conservative" && ($3=="ps"||$3=="rci"||$3=="om"||$3=="ddf"||$3=="ddp") {print $3"\t"$5}' \
  data/clean-room-v1/{CODE}.tsv | sort | tee /tmp/{CODE}-identity-audit.txt | wc -l
```

Where the answer is yes, keep the unambiguous compound and drop the bare form. `神经病` not `神经`. `绿茶婊` not `綠茶`. `香蕉人` not `香蕉`. Never downgrade the bare form to `inclusive` instead — that leaves an ordinary word in a slur lexicon scoring 0.6 on every neutral use of it.

**Verify both questions against evidence, never from memory.** Grep the corpus for each lemma and read the hits. Italian ran the dominant-sense test over `orecchione` from recall, brainstormed one alternate sense, picked the wrong one, and shipped the ordinary word for mumps at `om` conservative — while only corpus-checking rows it already doubted, which is precisely the set that does not need checking. The same session then justified keeping `mongolo` on "2 corpus hits" when there is 1; the second was `mongolfiera`, a hot air balloon, matched as a substring. A lexicon built to catch substring collisions should not gather its own evidence with one.

Ask a second question of every row, because the first one alone is not enough: **is the dominant sense itself mild?** The first question catches a slur that collides with an unrelated word. The second catches a word that is genuinely about the target group but is ordinary, affectionate, or merely blunt rather than abusive. Japanese passed the first test and still shipped `じじい` and `ばばあ` at `ddf` conservative — near-universal, frequently affectionate address terms for old people, which HurtLex itself does not carry. A word can be correctly categorised and still not belong at `conservative`.

This step exists because recall-based scanning provably does not catch this class. Chinese ran a fix round aimed squarely at it and three instances survived; the enumeration then found sixteen more, including `阿里巴巴`, the Alibaba Group, and `輪子`, the word for wheel, both sitting at `ps` conservative with `stereotype=yes`.

Report the count audited and the count changed. A pass that reports zero changes on a language with more than a hundred identity rows has almost certainly not been run.

**Sweep inflected and script variants of every entry.** The runtime does no stemming — `src/text.rs` performs NFC normalisation and confusable folding only — so `macaco` and `macaca` are independent lemmas that must each be present and must agree. Every language has a shape of this problem and each needs a mechanical sweep, not a manual one:

- Gendered Romance and Slavic languages: masculine, feminine, and plural forms. Portuguese evaluated `macaco` and never considered `macaca`, which is 87.5% toxic in its own corpus and is one of the most recognised anti-Black slurs in the language.
- Chinese: simplified against traditional. Solved with OpenCC, which found 55 pairs and 12 disagreements.
- Japanese: kanji, hiragana, katakana. Solved with pykakasi grouping by reading, 128 groups.

Report the method, its coverage, and the number of groups and disagreements found. A sweep that finds nothing on a gendered language has not been run.

**Reconcile every removal before you report.** Diff the lemma set, not the row count, and make the arithmetic close:

```bash
python3 -c "
import csv,subprocess
def lemmas(ref,code):
    out=subprocess.run(['git','show',f'{ref}:data/clean-room-v1/{code}.tsv'],
                       capture_output=True,text=True).stdout.splitlines()
    return {r['lemma'] for r in csv.DictReader(out,delimiter='\t')}
before,after=lemmas('BASEREF','{CODE}'),lemmas('HEAD','{CODE}')
print('removed:',sorted(before-after))
print('added  :',len(after-before))
"
```

Every lemma in `removed` gets a named reason in the report. This is not bureaucracy: Chinese removed `太平公主` and Japanese removed `パンパン` with no mention in either report, and both were found only because a reviewer diffed the lemma sets after noticing the arithmetic was off by one. An undisclosed removal is the one change nobody can audit — it looks identical whether it deleted a neutral word correctly or a real slur by accident.

- [ ] **Step 6: Build**

```bash
cargo run --release -p blasphem-train -- lexicon-build \
  --harvest /tmp/harvest --storage-code {CODE} --output data/clean-room-v1
head -3 data/clean-room-v1/{CODE}.tsv
```

Expected: a header row and ids of the form `{CODE}00001`.

- [ ] **Step 7: Validate the schema before wasting a compile**

```bash
python3 -c "
import csv
rows=list(csv.DictReader(open('data/clean-room-v1/{CODE}.tsv'),delimiter='\t'))
ok=all(r['level'] in ('conservative','inclusive') and r['stereotype'] in ('yes','no')
       and r['lemma'].strip() and r['id'].startswith('{CODE}') for r in rows)
ids=[r['id'] for r in rows]
print('rows',len(rows),'schema_ok',ok,'dupes',len(ids)-len(set(ids)))
"
```

Expected: `schema_ok True dupes 0`.

- [ ] **Step 8: Measure the rule channel alone**

```bash
mkdir -p /tmp/lexcheck-{CODE}
cp data/clean-room-v1/{CODE}.tsv /tmp/lexcheck-{CODE}/hurtlex_{CODE}.tsv
python3 -c "
import csv
rows=[r for r in csv.DictReader(open('corpus/{CODE}.tsv'),delimiter='\t') if r['split']=='test']
with open('/tmp/lexcheck-{CODE}/corpus.tsv','w',newline='') as f:
    w=csv.writer(f,delimiter='\t',lineterminator='\n'); w.writerow(['language','label','text'])
    for r in rows: w.writerow(['{CODE}',r['label'],r['text'].replace('\t',' ')])
"
cargo run --release -p blasphem-train -- eval \
  --input /tmp/lexcheck-{CODE}/corpus.tsv --data-dir /tmp/lexcheck-{CODE}
```

This is a fast signal before the expensive compile. Compare it against the same command pointed at a directory holding the HurtLex file. If the new lexicon loses badly here, go back to Step 5.

- [ ] **Step 9: Commit the language**

```bash
git add data/clean-room-v1/{CODE}.tsv data/clean-room-v1/{CODE}.senses.tsv data/clean-room-v1/{CODE}.provenance.json
git commit -m "Build the clean-room {CODE} lexicon"
```

---

## Task 3b: Retrofit the variant sweep onto languages built before it existed

Spanish, Turkish, Chinese, Japanese and Italian were built before the inflected-variant sweep became a mandatory step. Chinese and Japanese are covered — they ran OpenCC and pykakasi respectively. Turkish is agglutinative and the sweep does not apply in the same shape. Spanish and Italian are gendered Romance languages that never had one.

The sweep is not merely a correctness fix. Portuguese gained 155 entries from it, taking the file from 388 to 543 and identity coverage from 97 to 207. That is a 40% gain on a language whose sources were otherwise exhausted.

Crude exposure, counting masculine `-o` lemmas whose `-a` twin is absent: Spanish 419 of 1,494, Italian 96 of 437, Portuguese 57 of 543 after its sweep. The Spanish and Italian figures overcount, because many `-o` words are not gendered person-nouns — `culo` has no `cula`. Portuguese generated 374 candidates and accepted 155, rejecting 219, so expect a similar acceptance rate.

**Files:** `data/clean-room-v1/ES.tsv`, `ES.senses.tsv`, `IT.tsv`, `IT.senses.tsv`, and both provenance files.

- [ ] **Step 1: Read the Portuguese method**

`.superpowers/sdd/2026-09-03-clean-room-lexicon/task-3-PT-report.md` documents the working method. Reuse it rather than reinventing it. Two of its rejections show the required discrimination: it declined to add `feminista`, the neutral word, and it caught `moura` colliding with a footballer's surname.

- [ ] **Step 2: Sweep Spanish, then Italian**

Generate masculine, feminine and plural variants for every entry. Corpus-check each candidate with word-boundary greps. Reject any variant with a competing everyday sense — the same test as Step 5b, and for the same reason.

**Inflect dropped lemmas too, not only kept ones.** Portuguese's sweep re-inflected its kept entries and would therefore never have found `macaca`, the very word that prompted building it: `macaco` was dropped for having competing senses, so its feminine was never generated. A form can be a live slur while its opposite gender is genuinely ambiguous. Run the same generation over `{CODE}.drops.txt` and corpus-check the results. Portuguese's excluded identity terms — `judeu`, `cigano`, `preto`, `negro`, `polaco` — were spot-checked afterwards and none turned up a live miss, but that was luck rather than method.

**Verify sibling inheritance mechanically after generating.** Every variant must agree with its source on category, level and stereotype. Portuguese's sweep broke this on 4 of 155 additions — `fodas` landing at inclusive against `foda` at conservative, and `infernos` at `svp` against `inferno` at `cds`, a category disagreement rather than a level one. Assert the property in the sweep rather than reading for it.

- [ ] **Step 3: Rebuild and verify**

```bash
cargo run --release -p blasphem-train -- lexicon-build --harvest /tmp/harvest --storage-code ES --output data/clean-room-v1
cargo run --release -p blasphem-train -- lexicon-build --harvest /tmp/harvest --storage-code IT --output data/clean-room-v1
```

Report candidates generated, accepted, and rejected per language, and the before-and-after entry and identity counts.

- [ ] **Step 4: Commit**

```bash
git add data/clean-room-v1/ES.tsv data/clean-room-v1/ES.senses.tsv data/clean-room-v1/IT.tsv data/clean-room-v1/IT.senses.tsv data/clean-room-v1/ES.provenance.json data/clean-room-v1/IT.provenance.json
git commit -m "Sweep inflected variants into the Spanish and Italian lexica"
```

---

## Task 4: Repoint the runtime at the new lexica

Do this once, after every language that will pass has passed. The four provenance gates all fire together, so a partial swap does not build.

**Files:**
- Modify: `src/embedded.rs:13-27`
- Modify: `src/registry.rs` (fifteen `hurtlex_sha256` digests)
- Modify: `crates/blasphem-train/src/model_manifest.rs:26`
- Modify: `resources/datasets/source-lock-v1.json`
- Modify: `turbo.json:17`

- [ ] **Step 1: Repoint the embedded bytes**

Change every `include_bytes!` in `src/embedded.rs` from `../data/raw-v1/hurtlex/EN/1.2/hurtlex_EN.tsv` to `../data/clean-room-v1/EN.tsv`, and the same for the other fourteen storage codes. Remember `Language::Ms` reads `ID.tsv`.

- [ ] **Step 2: Recompute the fifteen digests**

```bash
for f in data/clean-room-v1/*.tsv; do
  case "$f" in *senses*) continue;; esac
  echo "$(basename "$f" .tsv) $(shasum -a 256 "$f" | cut -d' ' -f1)"
done
```

Paste each digest into the matching `RegistryEntry::new` call in `src/registry.rs`. The ES entry is at `src/registry.rs:230`. Update `SPANISH_HURTLEX_SHA256` at `crates/blasphem-train/src/model_manifest.rs:26` to the new ES digest.

- [ ] **Step 3: Update the source lock**

Replace the HurtLex entries in `resources/datasets/source-lock-v1.json` with one entry per new source: English Wiktionary (CC BY-SA 4.0), LDNOOBW (CC BY 4.0), washyourmouth (MIT). Record the harvest SHA-256 from Task 1 as the revision anchor for Wiktionary, which has no commit hash.

- [ ] **Step 4: Update the turbo input path**

Change `$TURBO_ROOT$/data/raw-v1/hurtlex/**` at `turbo.json:17` to `$TURBO_ROOT$/data/clean-room-v1/**`.

- [ ] **Step 5: Build**

```bash
cargo build --release --locked 2>&1 | tail -5
```

Expected: `Finished`. A digest mismatch here means Step 2 missed an entry.

- [ ] **Step 6: Commit**

```bash
git add src/embedded.rs src/registry.rs crates/blasphem-train/src/model_manifest.rs resources/datasets/source-lock-v1.json turbo.json
git commit -m "Point the runtime at the clean-room lexica"
```

---

## Task 5: Recompile and gate

**Files:**
- Modify: `resources/models/multilingual-v2/*.bin`
- Modify: `resources/models/multilingual-v2/manifest.json`
- Modify: `reports/multilingual-validation.json`

- [ ] **Step 1: Compile the model set**

```bash
rm -rf /tmp/models-clean
cargo run --release --locked -p blasphem-train -- compile \
  --corpus-root corpus \
  --source-lock resources/datasets/source-lock-v1.json \
  --hurtlex-root data/clean-room-v1 \
  --output /tmp/models-clean
```

`compile` expects the nested layout `{CODE}/1.2/hurtlex_{CODE}.tsv`. Either lay a shadow root out that way or add a flat-layout flag to `CompileArgs` at `crates/blasphem-train/src/main.rs:119`. Prefer the flag. The nested `1.2` path is a HurtLex version number and means nothing here.

- [ ] **Step 2: Read the gate**

```bash
python3 -c "
import json
old=json.load(open('resources/models/multilingual-v2/manifest.json'))
new=json.load(open('/tmp/models-clean/manifest.json'))
o={e['language']:e['validation_metrics']['f1'] for e in old['entries']}
n={e['language']:e['validation_metrics']['f1'] for e in new['entries']}
fails=[]
for l in o:
    d=n[l]-o[l]
    mark='PASS' if d>=-0.001 else 'FAIL'
    if mark=='FAIL': fails.append(l)
    print(f'{l:4} {o[l]:.4f} -> {n[l]:.4f} {d:+.4f} {mark}')
print('failing:',fails or 'none')
"
```

Expected on the Spanish precedent: `ES 0.4228 -> 0.5476 +0.1247 PASS`.

- [ ] **Step 3: Handle failures**

A language that fails the gate has three outs, in order of preference. Expand its sense table and rebuild. Accept a documented regression if the absolute F1 stays above the product threshold. Keep HurtLex for that language and ship the package as split-licence until it is fixed.

Do not silently lower the gate. Record every accepted regression in the report from Task 6.

- [ ] **Step 4: Promote the artifacts**

```bash
cp /tmp/models-clean/*.bin /tmp/models-clean/manifest.json resources/models/multilingual-v2/
```

Then update the fifteen `artifact_sha256` digests in `src/registry.rs` from the new manifest, and rebuild.

- [ ] **Step 5: Regenerate the evidence**

```bash
cargo run --release --locked -p blasphem-train -- evaluate \
  --split validation --corpus-root corpus \
  --model-manifest resources/models/multilingual-v2/manifest.json \
  --hurtlex-root data/clean-room-v1 \
  --output reports/multilingual-validation.json
cargo test --workspace --locked 2>&1 | tail -20
```

Expected: `status=calibration_evidence languages=15` and a green suite.

- [ ] **Step 6: Commit**

```bash
git add resources/models/multilingual-v2 src/registry.rs reports/multilingual-validation.json
git commit -m "Recompile the models from the clean-room lexica"
```

---

## Task 6: Retire HurtLex and restate the licence

Only after Task 5 is green for every language.

**Files:**
- Delete: `data/raw-v1/hurtlex/` (15 files)
- Delete: `data/hurtlex/` (15 files, already dead per `docs/dead-code-audit.md:168`)
- Modify: `NOTICE:18-29`
- Modify: `README.md:268`, `:290`, `:307`, `:318`, `:368`
- Modify: `CONTRIBUTING.md:96`, `:105`
- Create: `docs/clean-room-lexicon-report.md`

- [ ] **Step 1: Rewrite the NOTICE block**

Delete the `## hurtlex` section at `NOTICE:18-29`. Add one section per new source. Wiktionary is CC BY-SA 4.0, so the share-alike statement stays, worded against Wiktionary rather than HurtLex. LDNOOBW is CC BY 4.0, attribution only. washyourmouth is MIT.

State plainly that the NonCommercial term is gone and that share-alike remains.

- [ ] **Step 2: Write the report**

Create `docs/clean-room-lexicon-report.md` with the before and after F1 per language, the entry count per language, the source split per language, and every accepted regression. Report sizes in megabytes with two decimals.

- [ ] **Step 3: Delete the lexica**

```bash
git rm -r data/raw-v1/hurtlex data/hurtlex
grep -rn "hurtlex" --include="*.rs" --include="*.toml" --include="*.json" src crates resources turbo.json | grep -v clean-room
```

Expected from the grep: only the `hurtlex_sha256` field name and the `parse_hurtlex` function name, which are internal identifiers. Rename them to `lexicon_sha256` and `parse_lexicon` in a follow-up commit if you want the name gone too. That rename touches `src/lexicon.rs:64`, `src/lib.rs:37`, `src/workflow.rs:71`, `src/rules/channel.rs:115`, and `crates/blasphem-bench/examples/profile_dense.rs:28`.

- [ ] **Step 4: Full verification**

```bash
cargo test --workspace --locked 2>&1 | tail -20
cargo run --release --locked -p blasphem-train -- corpus-verify \
  --corpus-root corpus \
  --evaluation-lock resources/datasets/evaluation-lock-v1.json 2>&1 | tail -3
pnpm -w turbo run build test 2>&1 | tail -20
```

Expected: all green. `corpus-verify` must still pass because the corpus never changed.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "Retire the HurtLex lexica and restate the licence"
```

---

## Source matrix

Every count below is measured, not estimated. Native Wiktionary category sizes were read from `prop=categoryinfo` on each wiki. textdetox counts come from the dataset card.

| Lang | Native Wiktionary | textdetox | en.wiktionary | Flat lists | Gate |
|---|---:|---:|---:|---:|---:|
| RU | 461 | **141,000** | — | 290 | 0.8604 |
| FR | **9,770** | 1,290 | — | 217 | 0.9108 |
| EN | — | 3,390 | **8,835** | 550 | 0.7496 |
| ZH | not found | **3,840** | present | 319 | 0.0674 |
| JA | **873** | 328 | 269 | 302 | 0.3730 |
| IT | 319 | **815** | — | 299 | 0.3769 |
| DE | not found | 247 | **1,223** | 199 | 0.5543 |
| TR | **797** | — | — | 274 | 0.0516 |
| PT | **416** | — | — | 208 | 0.4895 |
| AR | not found | **430** | 61 | 37 | 0.5343 |
| KO | **248** | — | present | 191 | 0.5738 |
| VI | 31 | — | **265** | 115 | 0.5854 |
| HI | — | 133 | **137** | 256 | 0.7471 |
| MS | 205 | — | **231** | 115 | 0.5783 |
| ES | 668 | 430 | — | 193 | **done at 1,493** |

Native category names, verified:

- FR: `Catégorie:Termes péjoratifs en français` 3955, `Termes argotiques en français` 3895, `Termes vulgaires en français` 1212, `Insultes en français` 708
- JA: `カテゴリ:日本語 俗語` 853, `日本語 卑語` 20
- TR: `Kategori:Türkçe argo` 749, `Türkçe kaba konuşma` 48
- RU: `Категория:Бранные выражения/ru` 442, `Оскорбления/ru` 12, `Ругательства/ru` 6
- PT: `Categoria:Pejorativo (Português)` 416
- IT: `Categoria:Parole volgari-IT` 314, `Termini dispregiativi` 5
- KO: `분류:한국어 속어` 248
- MS: `Kategori:id:Istilah kasar` 103, `Turunan kata kasar` 11, `Turunan kata umpat` 4
- VI: `Thể loại:Từ thô tục tiếng Việt` 22, `Từ xúc phạm tiếng Việt` 8

DE, AR, and ZH returned nothing under the obvious names. Their native wikis use a different scheme. Discover it with `list=search&srnamespace=14` before falling back. English Wiktionary already covers DE at 1,223 and textdetox covers AR at 430 and ZH at 3,840, so none of the three is blocked.

MS harvests under two Wiktionary language names, because Malay and Indonesian are one language for this system's purposes and both feed `hurtlex_ID.tsv`. On en.wiktionary: `Malay derogatory terms` 67, `Malay vulgarities` 48, `Malay offensive terms` 22, `Malay ethnic slurs` 10, `Malay dysphemisms` 3, `Malay anti-LGBTQ slurs` 1, plus the Indonesian set at 80. On ms.wiktionary: `Kategori:Kata kasar bahasa Melayu` 65 and `Kata kasar bahasa Indonesia` 22, plus id.wiktionary at 118.

Match Malay titles exactly, never by prefix. `apprefix=Malay` also returns `Malayalam derogatory terms`, `Malayalam offensive terms`, and `Malayalam vulgarities`. Malayalam is a different language and pulling it in would poison the file.

## Extra corpora for VI and MS

These two have the thinnest lexicon supply and the largest available corpora. Mine them for candidates. Do not add them to `corpus/`, or the validation split stops being the thing the gate table measured.

| Dataset | Licence | Rows | Language |
|---|---|---:|---|
| `zerostratos/vietnamese_toxic_core` | Apache-2.0 | 48,553 | VI |
| `uitnlp/vihsd` | MIT | 10K–100K | VI |
| `haipradana/indonesian-twitter-hate-speech-cleaned` | Apache-2.0 | 18,148 | MS |

`vietnamese_toxic_core` has `text` and a binary `label`. The Indonesian set has `text` and a `neutral`/hate label. `uitnlp/vihsd` is UIT-ViHSD from the original authors.

Rejected for licensing: `tarudesu/ViCTSD`, `SEACrowd/id_abusive`, `SEACrowd/local_id_abusive`, `SEACrowd/id_abusive_news_comment`, `manueltonneau/indonesian-hate-speech-superset`, `naot97/vietnamese-toxic-data`. All carry no licence tag, which is all rights reserved and worse than HurtLex.

## Risks

Raw count is not the predictor. Spanish beat HurtLex with 1,493 entries against 5,006 rows. Treat the matrix as supply, not as outcome.

VI and MS have the thinnest lexicon supply, but both now have a route. MS gained 151 Malay entries on en.wiktionary and 87 on ms.wiktionary once Malay and Indonesian are treated as one language, reaching roughly 900 lemmas before dedup against HurtLex ID's 2,387 unique. VI reaches roughly 700 against HurtLex VI's 1,510. Spanish cleared its gate at 1,493 against 3,354, a comparable ratio, so run the compile before assuming either fails.

Their real deficit is identity categories, and the gate will not reveal it. HurtLex holds 320 identity lemmas for VI and 524 for ID. Wiktionary topic categories are nearly empty for both: `vi:Ethnicity` 6, `vi:Demonyms` 14, `vi:LGBTQ` 6, no `vi:Disability`; `id:Ethnicity` 4, `id:Demonyms` 2, `id:LGBTQ` 10, `id:Disability` 1. The Spanish intersection rule yields almost nothing here. The frame miner in Task 3 Step 4 is the substitute.

A HurtLex lookup adds little for these two. Measured against the current candidate pools, only 40 of 412 VI lemmas and 84 of 455 MS lemmas appear in HurtLex at all, and of those only 12 and 27 carry an identity category. Roughly half the VI hits are translation artefacts: HurtLex tags `mông` (buttocks), `đít` (arse), and `lừa` (donkey) as `om`. The Indonesian hits are cleaner. Run the lookup, it is nearly free, but grow the pool first because the overlap scales with the pool and not with HurtLex.

`ddf` and `ddp` have no found source in either language. Neither Wiktionary nor the frame miner reaches them. Expect to hand-write those two categories or ship them short.

FR and RU are no longer the risk. FR has the deepest native supply in the set and RU has 141,000 textdetox entries. Their high gates now look reachable.

textdetox is OpenRAIL++, which is a use-restricted licence rather than an OSI one. `corpus/*.tsv` already comes from that project, so this adds no new obligation, but record it in the NOTICE as its own class rather than folding it in with the CC sources.

Share-alike survives this work. Wiktionary is CC BY-SA 4.0. Anyone shipping a derived lexicon still carries BY-SA. The win is that NonCommercial is gone, so commercial use of the package becomes lawful.
