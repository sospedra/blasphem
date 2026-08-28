//! Wiktionary lexicon harvest and offline build. Harvest takes network in and
//! writes JSON; build takes that JSON plus a human sense table and writes the
//! final lexicon TSV. No assignment happens during harvest.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use blasphem::MatchLevel;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const USER_AGENT: &str = "blasphem-lexicon-build/1.0 (https://github.com/sospedra/blasphem)";
const EN_WIKTIONARY_HOST: &str = "en.wiktionary.org";

const BASE_RETRY_DELAY: Duration = Duration::from_secs(3);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(60);
/// Wikimedia 429 blocks can persist for minutes. Keep retrying with capped
/// exponential backoff until this much total wait has been spent, then stop.
const MAX_TOTAL_RETRY_WAIT: Duration = Duration::from_secs(180);
/// Defensive bound on continuation pages. A real Wiktionary continuation
/// always terminates well before this for any single category or lemma batch.
const MAX_PAGES: usize = 500;
const LEMMA_BATCH_SIZE: usize = 12;

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

/// The "{code}:" topic categories that feed lexicon assignment. Deliberately
/// excludes "{code}:People", which runs to thousands of rows nothing reads.
const TOPIC_NAMES: &[&str] = &[
    "Ethnicity",
    "Nationalities",
    "Demonyms",
    "LGBTQ",
    "Sexual orientations",
    "Disability",
    "Male genitalia",
    "Female genitalia",
    "Genitalia",
    "Prostitution",
    "Crime",
    "Religion",
    "Occupations",
    "Military",
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
    #[error("cannot parse a sense or drop table: {0}")]
    Csv(#[from] csv::Error),
    #[error("sense table entry for {lemma} has an unknown category: {category:?}")]
    InvalidCategory { lemma: String, category: String },
    #[error("sense table entry for {lemma} has an invalid level: {level:?}")]
    InvalidLevel { lemma: String, level: String },
    #[error("sense table has two rows for {lemma:?}: lines {first_line} and {second_line}")]
    DuplicateSense {
        lemma: String,
        first_line: usize,
        second_line: usize,
    },
}

type QueryParams = BTreeMap<String, String>;

/// The default single-wiki source for a CLI harvest: en.wiktionary.org with
/// suffix-derived categories.
#[must_use]
pub fn default_wiki(language_name: &str) -> WikiSource {
    WikiSource {
        host: EN_WIKTIONARY_HOST.to_owned(),
        categories: Vec::new(),
        strong: STRONG_SUFFIXES
            .iter()
            .map(|suffix| format!("Category:{language_name} {suffix}"))
            .collect(),
    }
}

/// Harvests offensive-word categories from every configured wiki and writes
/// `{options.output}/{storage_code}.harvest.json`.
///
/// # Errors
///
/// Returns an error when a request fails after retries, a response cannot be
/// parsed, the combined harvest has zero lemmas, or the output cannot be
/// written.
pub fn harvest(options: &HarvestOptions) -> Result<HarvestReport, LexiconError> {
    let client = Client::builder().user_agent(USER_AGENT).build()?;
    let mut offence = BTreeMap::new();
    let mut topic = BTreeMap::new();
    for wiki in &options.wikis {
        let harvested = harvest_wiki(&client, wiki, options)?;
        merge_new_keys(&mut offence, harvested.offence);
        merge_new_keys(&mut topic, harvested.topic);
    }
    let lemmas = dedup_lemmas(&offence);
    if lemmas.is_empty() {
        return Err(LexiconError::Empty(options.storage_code.clone()));
    }
    let host = primary_host(options);
    let raw_lemma_categories = lemma_category_map(&client, host, &lemmas)?;
    let code = options.storage_code.to_ascii_lowercase();
    let lemma_categories =
        filter_lemma_categories(raw_lemma_categories, &options.language_name, &code);
    let harvest = Harvest {
        language_name: options.language_name.clone(),
        storage_code: options.storage_code.clone(),
        offence,
        topic,
        lemma_categories,
    };
    write_harvest(&options.output, &harvest, lemmas.len())
}

struct WikiHarvest {
    offence: BTreeMap<String, Vec<String>>,
    topic: BTreeMap<String, Vec<String>>,
}

fn harvest_wiki(
    client: &Client,
    wiki: &WikiSource,
    options: &HarvestOptions,
) -> Result<WikiHarvest, LexiconError> {
    if wiki.categories.is_empty() {
        return harvest_derived_wiki(client, wiki, options);
    }
    let offence: BTreeMap<String, Vec<String>> = wiki
        .categories
        .iter()
        .map(|title| {
            Ok((
                bare_category_name(title),
                category_members(client, &wiki.host, title)?,
            ))
        })
        .collect::<Result<_, LexiconError>>()?;
    Ok(WikiHarvest {
        offence,
        topic: BTreeMap::new(),
    })
}

fn harvest_derived_wiki(
    client: &Client,
    wiki: &WikiSource,
    options: &HarvestOptions,
) -> Result<WikiHarvest, LexiconError> {
    let categories = all_categories(client, &wiki.host, &options.language_name)?;
    let offence: BTreeMap<String, Vec<String>> = OFFENCE_SUFFIXES
        .iter()
        .filter_map(|suffix| {
            let title = format!("Category:{} {suffix}", options.language_name);
            categories
                .contains(&title)
                .then(|| ((*suffix).to_owned(), title))
        })
        .map(|(key, title)| Ok((key, category_members(client, &wiki.host, &title)?)))
        .collect::<Result<_, LexiconError>>()?;
    let code = options.storage_code.to_ascii_lowercase();
    let topic = topic_categories(client, &wiki.host, &code)?;
    Ok(WikiHarvest { offence, topic })
}

fn bare_category_name(title: &str) -> String {
    title.strip_prefix("Category:").unwrap_or(title).to_owned()
}

fn merge_new_keys(
    target: &mut BTreeMap<String, Vec<String>>,
    additions: BTreeMap<String, Vec<String>>,
) {
    for (key, value) in additions {
        target.entry(key).or_insert(value);
    }
}

fn dedup_lemmas(offence: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut lemmas: Vec<String> = offence.values().flatten().cloned().collect();
    lemmas.sort();
    lemmas.dedup();
    lemmas
}

fn primary_host(options: &HarvestOptions) -> &str {
    options
        .wikis
        .first()
        .map_or(EN_WIKTIONARY_HOST, |wiki| wiki.host.as_str())
}

fn write_harvest(
    output: &Path,
    harvest: &Harvest,
    lemma_count: usize,
) -> Result<HarvestReport, LexiconError> {
    let bytes = serde_json::to_vec_pretty(harvest)?;
    let path = output.join(format!("{}.harvest.json", harvest.storage_code));
    fs::create_dir_all(output).map_err(|source| LexiconError::Io {
        path: output.to_owned(),
        source,
    })?;
    fs::write(&path, &bytes).map_err(|source| LexiconError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(HarvestReport {
        lemmas: lemma_count,
        sha256: hex(&Sha256::digest(&bytes)),
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Every category title in namespace 14 whose name starts with `prefix`.
/// Query: list=allpages&apnamespace=14&apprefix={prefix}&aplimit=500
/// Continue key: apcontinue. "Spanish" returns 3,589 titles across eight
/// pages; "es:" returns 4,629 across ten.
fn all_categories(
    client: &Client,
    host: &str,
    prefix: &str,
) -> Result<BTreeSet<String>, LexiconError> {
    let mut titles = BTreeSet::new();
    let mut cursor = None;
    for _ in 0..MAX_PAGES {
        let response = get_json(client, host, &allpages_params(prefix, cursor.as_deref()))?;
        titles.extend(page_titles(&response, "allpages"));
        cursor = next_cursor(&response, "apcontinue");
        if cursor.is_none() {
            break;
        }
    }
    Ok(titles)
}

fn allpages_params(prefix: &str, cursor: Option<&str>) -> QueryParams {
    let mut params = QueryParams::from([
        ("action".to_owned(), "query".to_owned()),
        ("list".to_owned(), "allpages".to_owned()),
        ("apnamespace".to_owned(), "14".to_owned()),
        ("apprefix".to_owned(), prefix.to_owned()),
        ("aplimit".to_owned(), "500".to_owned()),
        ("format".to_owned(), "json".to_owned()),
    ]);
    if let Some(value) = cursor {
        params.insert("apcontinue".to_owned(), value.to_owned());
    }
    params
}

/// Main-namespace members of one category.
/// Query: list=categorymembers&cmtitle={title}&cmnamespace=0&cmlimit=500
/// Continue key: cmcontinue.
fn category_members(client: &Client, host: &str, title: &str) -> Result<Vec<String>, LexiconError> {
    let mut members = Vec::new();
    let mut cursor = None;
    for _ in 0..MAX_PAGES {
        let response = get_json(
            client,
            host,
            &category_members_params(title, cursor.as_deref()),
        )?;
        members.extend(page_titles(&response, "categorymembers"));
        cursor = next_cursor(&response, "cmcontinue");
        if cursor.is_none() {
            break;
        }
    }
    Ok(members)
}

fn category_members_params(title: &str, cursor: Option<&str>) -> QueryParams {
    let mut params = QueryParams::from([
        ("action".to_owned(), "query".to_owned()),
        ("list".to_owned(), "categorymembers".to_owned()),
        ("cmtitle".to_owned(), title.to_owned()),
        ("cmnamespace".to_owned(), "0".to_owned()),
        ("cmlimit".to_owned(), "500".to_owned()),
        ("format".to_owned(), "json".to_owned()),
    ]);
    if let Some(value) = cursor {
        params.insert("cmcontinue".to_owned(), value.to_owned());
    }
    params
}

/// Members of TOPIC_NAMES under "{code}:", keyed by bare topic name such as
/// "es:Demonyms". Fetches its own "{code}:" category listing first — Spanish
/// alone has 4,629 of them, most irrelevant — and only pulls members for a
/// name confirmed to exist there. Skip topics with zero members; a name that
/// exists but is currently empty (verified for "es:Ethnicity") still
/// resolves, it is simply absent from the result.
fn topic_categories(
    client: &Client,
    host: &str,
    code: &str,
) -> Result<BTreeMap<String, Vec<String>>, LexiconError> {
    let prefix = format!("{code}:");
    let candidates = all_categories(client, host, &prefix)?;
    let mut topics = BTreeMap::new();
    for name in TOPIC_NAMES {
        let title = format!("Category:{prefix}{name}");
        if !candidates.contains(&title) {
            continue;
        }
        let members = category_members(client, host, &title)?;
        if !members.is_empty() {
            topics.insert(bare_category_name(&title), members);
        }
    }
    Ok(topics)
}

/// Per-lemma category membership, for part of speech and topic lookup.
/// Query: prop=categories&cllimit=max&titles={twelve titles joined by |}
/// Continue key: the whole `continue` object, merged into the next query.
/// Twelve titles per request is deliberate. Larger batches exhaust the
/// category budget mid-response and silently truncate.
fn lemma_category_map(
    client: &Client,
    host: &str,
    lemmas: &[String],
) -> Result<BTreeMap<String, Vec<String>>, LexiconError> {
    let mut categories = BTreeMap::new();
    for batch in lemmas.chunks(LEMMA_BATCH_SIZE) {
        categories.extend(lemma_categories_batch(client, host, batch)?);
    }
    Ok(categories)
}

fn lemma_categories_batch(
    client: &Client,
    host: &str,
    batch: &[String],
) -> Result<BTreeMap<String, Vec<String>>, LexiconError> {
    let mut pages: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut params = lemma_categories_params(batch);
    for _ in 0..MAX_PAGES {
        let response = get_json(client, host, &params)?;
        merge_page_categories(&mut pages, &response);
        let Some(next) = next_batch_params(&params, &response) else {
            break;
        };
        params = next;
    }
    Ok(pages)
}

fn lemma_categories_params(batch: &[String]) -> QueryParams {
    QueryParams::from([
        ("action".to_owned(), "query".to_owned()),
        ("prop".to_owned(), "categories".to_owned()),
        ("cllimit".to_owned(), "max".to_owned()),
        ("titles".to_owned(), batch.join("|")),
        ("format".to_owned(), "json".to_owned()),
    ])
}

fn merge_page_categories(pages: &mut BTreeMap<String, Vec<String>>, response: &Value) {
    for (title, categories) in response_categories(response) {
        pages.entry(title).or_default().extend(categories);
    }
}

fn next_batch_params(params: &QueryParams, response: &Value) -> Option<QueryParams> {
    let continuation = response.get("continue")?.as_object()?;
    let mut merged = params.clone();
    for (key, value) in continuation {
        merged.insert(key.clone(), value.as_str()?.to_owned());
    }
    Some(merged)
}

fn response_categories(response: &Value) -> Vec<(String, Vec<String>)> {
    response
        .pointer("/query/pages")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(_, page)| {
            let title = page.get("title").and_then(Value::as_str)?.to_owned();
            let categories = page
                .get("categories")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|category| category.get("title").and_then(Value::as_str))
                .map(str::to_owned)
                .collect();
            Some((title, categories))
        })
        .collect()
}

/// Keeps only categories starting with "{language} " or "{code}:".
fn filter_lemma_categories(
    raw: BTreeMap<String, Vec<String>>,
    language: &str,
    code: &str,
) -> BTreeMap<String, Vec<String>> {
    let language_prefix = format!("{language} ");
    let code_prefix = format!("{code}:");
    raw.into_iter()
        .map(|(title, categories)| {
            let kept = categories
                .into_iter()
                .map(|category| bare_category_name(&category))
                .filter(|bare| matches_language(bare, &language_prefix, &code_prefix))
                .collect();
            (title, kept)
        })
        .collect()
}

fn matches_language(bare_category: &str, language_prefix: &str, code_prefix: &str) -> bool {
    bare_category.starts_with(language_prefix) || bare_category.starts_with(code_prefix)
}

fn page_titles(response: &Value, key: &str) -> Vec<String> {
    response
        .pointer(&format!("/query/{key}"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.get("title").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn next_cursor(response: &Value, key: &str) -> Option<String> {
    response
        .get("continue")
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn get_json(client: &Client, host: &str, params: &QueryParams) -> Result<Value, LexiconError> {
    let url = format!("https://{host}/w/api.php");
    let mut waited = Duration::ZERO;
    let mut attempt: u32 = 0;
    let error = loop {
        attempt += 1;
        let error = match fetch_json(client, &url, params) {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        let delay = retry_delay(attempt);
        if waited.saturating_add(delay) > MAX_TOTAL_RETRY_WAIT {
            break error;
        }
        wait_before_retry(attempt, delay);
        waited += delay;
    };
    Err(error)
}

fn retry_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(16);
    let scaled = BASE_RETRY_DELAY.as_secs().saturating_mul(1_u64 << exponent);
    Duration::from_secs(scaled).min(MAX_RETRY_DELAY)
}

fn wait_before_retry(attempt: u32, delay: Duration) {
    eprintln!(
        "wiktionary request retry {attempt} after {}s",
        delay.as_secs()
    );
    thread::sleep(delay);
}

fn fetch_json(client: &Client, url: &str, params: &QueryParams) -> Result<Value, LexiconError> {
    let bytes = client
        .get(url)
        .query(params)
        .send()?
        .error_for_status()?
        .bytes()?;
    Ok(serde_json::from_slice(&bytes)?)
}

// --- Offline build: harvest JSON + human sense table -> the final TSV. ---

/// Categories that mark a stereotyped identity group. Drives both
/// `stereotype` and the `identity_entries` count in `BuildReport`.
const STEREOTYPE_CATEGORIES: &[&str] = &["ps", "rci", "om", "ddf", "ddp"];

/// The full six-column schema's category codes. A sense table row naming
/// anything outside this set is a typo, not a new category.
const VALID_CATEGORIES: &[&str] = &[
    "ps", "rci", "pa", "ddf", "ddp", "dmc", "is", "or", "an", "asm", "asf", "pr", "om", "qas",
    "cds", "re", "svp",
];

/// Topic name (bare, after the "{code}:" prefix) to category code. Order is
/// priority order: a lemma in more than one of these topics takes the
/// category of whichever entry appears first.
const TOPIC_CATEGORIES: &[(&str, &str)] = &[
    ("Ethnicity", "ps"),
    ("Nationalities", "ps"),
    ("Demonyms", "rci"),
    ("LGBTQ", "om"),
    ("Sexual orientations", "om"),
    ("Disability", "ddp"),
    ("Male genitalia", "asm"),
    ("Female genitalia", "asf"),
    ("Genitalia", "asm"),
    ("Prostitution", "pr"),
    ("Crime", "re"),
    ("Religion", "svp"),
    ("Occupations", "pa"),
    ("Military", "pa"),
    ("Politics", "dmc"),
    ("Nationalism", "dmc"),
    ("Conspiracy theories", "dmc"),
    ("Animals", "an"),
    ("Plants", "or"),
];

/// Wiktionary part-of-speech category suffixes, checked in this priority
/// order because a lemma commonly carries more than one (a masculine noun
/// used adjectivally is tagged both "nouns" and "adjectives").
const NOUN_TAGS: &[&str] = &["nouns", "proper nouns", "noun forms"];
const VERB_TAGS: &[&str] = &["verbs", "verb forms"];
const ADJECTIVE_TAGS: &[&str] = &["adjectives", "adjective forms"];
const ADVERB_TAGS: &[&str] = &["adverbs", "adverb forms"];
const INTERJECTION_TAGS: &[&str] = &["interjections"];

/// Spanish reflexive verbs end "-arse"/"-erse"/"-irse"; strip the clitic
/// before checking the infinitive ending.
const REFLEXIVE_VERB_SUFFIXES: &[&str] = &["arse", "erse", "irse"];

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

/// A human-reviewed override from `{STORAGE}.senses.tsv`. `level` is `None`
/// when the column was left blank, meaning "derive it from the rules".
#[derive(Debug, Clone)]
struct SenseOverride {
    category: String,
    level: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SenseRecord {
    lemma: String,
    category: String,
    level: String,
}

/// Where a row's category and level came from. Recorded in the provenance
/// file so a thin or surprising build is auditable without re-deriving it.
struct Assignment {
    category: String,
    category_source: &'static str,
    level: String,
    level_source: &'static str,
}

/// A row plus the provenance of its category and level, carried together so
/// the TSV and the provenance JSON render from one assignment pass.
struct AssignedRow {
    row: LexiconRow,
    category_source: &'static str,
    level_source: &'static str,
}

/// Builds the final lexicon TSV for one language from its Wiktionary harvest
/// plus the human sense table and drop list already checked in next to it.
///
/// # Errors
///
/// Returns an error when the harvest cannot be read or parsed, the sense or
/// drop table is malformed, a sense entry names an unknown category or level,
/// a lemma appears twice in the sense table, or the output cannot be
/// written.
pub fn build(options: &BuildOptions) -> Result<BuildReport, LexiconError> {
    let harvest_path = options
        .harvest_root
        .join(format!("{}.harvest.json", options.storage_code));
    let harvest_bytes = fs::read(&harvest_path).map_err(|source| LexiconError::Io {
        path: harvest_path,
        source,
    })?;
    let harvest: Harvest = serde_json::from_slice(&harvest_bytes)?;
    let senses = read_senses(&sibling_output_path(options, "senses.tsv"))?;
    let drops = read_drops(&sibling_output_path(options, "drops.txt"))?;

    let offence = lower_membership(&harvest.offence);
    let lemma_categories = lower_lemma_categories(&harvest.lemma_categories);
    let code = harvest.storage_code.to_ascii_lowercase();

    let (candidates, unmatched_drops) = candidate_lemmas(&offence, &senses, &drops);
    warn_unmatched_drops(&options.storage_code, &unmatched_drops);
    let mut assigned: Vec<AssignedRow> = candidates
        .into_iter()
        .map(|lemma| {
            let categories = lemma_categories.get(&lemma).map(Vec::as_slice);
            let assignment = assign(&code, &offence, categories, &senses, &lemma);
            let pos = assign_pos(&harvest.language_name, categories, &lemma);
            let stereotype = stereotype_flag(&assignment.category).to_owned();
            AssignedRow {
                row: LexiconRow {
                    id: String::new(),
                    pos,
                    category: assignment.category,
                    stereotype,
                    lemma,
                    level: assignment.level,
                },
                category_source: assignment.category_source,
                level_source: assignment.level_source,
            }
        })
        .collect();
    assigned.sort_by(|left, right| {
        (&left.row.category, &left.row.lemma).cmp(&(&right.row.category, &right.row.lemma))
    });

    let storage_code = &options.storage_code;
    for (index, item) in assigned.iter_mut().enumerate() {
        item.row.id = format!("{storage_code}{:05}", index + 1);
    }
    let identity_entries = assigned
        .iter()
        .filter(|item| STEREOTYPE_CATEGORIES.contains(&item.row.category.as_str()))
        .count();

    fs::create_dir_all(&options.output).map_err(|source| LexiconError::Io {
        path: options.output.clone(),
        source,
    })?;
    let tsv_bytes = render_tsv(&assigned);
    write_output(&sibling_output_path(options, "tsv"), &tsv_bytes)?;
    let provenance_bytes = render_provenance(options, &harvest, &harvest_bytes, &assigned)?;
    write_output(
        &sibling_output_path(options, "provenance.json"),
        &provenance_bytes,
    )?;

    Ok(BuildReport {
        entries: assigned.len(),
        identity_entries,
        sha256: hex(&Sha256::digest(&tsv_bytes)),
    })
}

fn sibling_output_path(options: &BuildOptions, extension: &str) -> PathBuf {
    options
        .output
        .join(format!("{}.{extension}", options.storage_code))
}

fn write_output(path: &Path, bytes: &[u8]) -> Result<(), LexiconError> {
    fs::write(path, bytes).map_err(|source| LexiconError::Io {
        path: path.to_owned(),
        source,
    })
}

/// Reads the human sense table. A missing file means no overrides yet, which
/// is the normal starting state for a language that has none.
fn read_senses(path: &Path) -> Result<BTreeMap<String, SenseOverride>, LexiconError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let file = fs::File::open(path).map_err(|source| LexiconError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mut reader = csv::ReaderBuilder::new().delimiter(b'\t').from_reader(file);
    let mut senses = BTreeMap::new();
    let mut lines_by_lemma: BTreeMap<String, usize> = BTreeMap::new();
    for (index, record) in reader.deserialize::<SenseRecord>().enumerate() {
        let record = record?;
        let line = index + 2;
        let lemma = record.lemma.trim().to_lowercase();
        if let Some(&first_line) = lines_by_lemma.get(&lemma) {
            return Err(LexiconError::DuplicateSense {
                lemma,
                first_line,
                second_line: line,
            });
        }
        lines_by_lemma.insert(lemma.clone(), line);
        let category = validate_category(&lemma, record.category.trim())?;
        let level = match record.level.trim() {
            "" => None,
            level => Some(validate_level(&lemma, level)?),
        };
        senses.insert(lemma, SenseOverride { category, level });
    }
    Ok(senses)
}

fn validate_category(lemma: &str, category: &str) -> Result<String, LexiconError> {
    if VALID_CATEGORIES.contains(&category) {
        return Ok(category.to_owned());
    }
    Err(LexiconError::InvalidCategory {
        lemma: lemma.to_owned(),
        category: category.to_owned(),
    })
}

fn validate_level(lemma: &str, level: &str) -> Result<String, LexiconError> {
    let parsed: MatchLevel = level.parse().map_err(|_| LexiconError::InvalidLevel {
        lemma: lemma.to_owned(),
        level: level.to_owned(),
    })?;
    Ok(match parsed {
        MatchLevel::Conservative => "conservative",
        MatchLevel::Inclusive => "inclusive",
    }
    .to_owned())
}

/// Reads the drop list: one lemma per line, excluded from the build
/// entirely. A missing file means nothing is dropped.
fn read_drops(path: &Path) -> Result<BTreeSet<String>, LexiconError> {
    if !path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = fs::read_to_string(path).map_err(|source| LexiconError::Io {
        path: path.to_owned(),
        source,
    })?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_lowercase)
        .collect())
}

/// Lowercases every lemma in a harvest membership map (`offence` or
/// `topic`), merging sets when two titles collide after casefolding.
fn lower_membership(source: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, BTreeSet<String>> {
    source
        .iter()
        .map(|(key, lemmas)| {
            let lowered = lemmas.iter().map(|lemma| lemma.to_lowercase()).collect();
            (key.clone(), lowered)
        })
        .collect()
}

/// Lowercases `lemma_categories` keys, merging category lists when two
/// harvested titles (for example "Charo" and "charo") collide after
/// casefolding.
fn lower_lemma_categories(source: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, Vec<String>> {
    let mut lowered: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (lemma, categories) in source {
        lowered
            .entry(lemma.to_lowercase())
            .or_default()
            .extend(categories.iter().cloned());
    }
    lowered
}

/// The lemmas a build considers: every offence-derived lemma plus every
/// lemma the sense table names outright (the channel through which words
/// with no Wiktionary presence, such as a flat-list addition, enter the
/// lexicon), minus anything dropped. Also reports which drop entries named
/// a lemma that was never a candidate, so a stale or misspelled drop is
/// visible instead of silently doing nothing.
fn candidate_lemmas(
    offence: &BTreeMap<String, BTreeSet<String>>,
    senses: &BTreeMap<String, SenseOverride>,
    drops: &BTreeSet<String>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut candidates: BTreeSet<String> = offence.values().flatten().cloned().collect();
    candidates.extend(senses.keys().cloned());
    let unmatched_drops = drops.difference(&candidates).cloned().collect();
    for drop in drops {
        candidates.remove(drop);
    }
    (candidates, unmatched_drops)
}

fn warn_unmatched_drops(storage_code: &str, unmatched: &BTreeSet<String>) {
    if unmatched.is_empty() {
        return;
    }
    let lemmas = unmatched.iter().cloned().collect::<Vec<_>>().join(", ");
    eprintln!("warning: {storage_code} drops.txt names lemmas absent from the build: {lemmas}");
}

fn assign(
    code: &str,
    offence: &BTreeMap<String, BTreeSet<String>>,
    categories: Option<&[String]>,
    senses: &BTreeMap<String, SenseOverride>,
    lemma: &str,
) -> Assignment {
    let sense = senses.get(lemma);
    let (category, category_source) = match sense {
        Some(sense) => (sense.category.clone(), "sense-table"),
        None => match topic_category(code, categories) {
            Some(category) => (category.to_owned(), "topic"),
            None => (
                membership_category(offence, lemma).to_owned(),
                "membership-default",
            ),
        },
    };
    let (level, level_source) = match sense.and_then(|sense| sense.level.clone()) {
        Some(level) => (level, "sense-table"),
        None if is_strong(offence, lemma) => ("conservative".to_owned(), "strong-suffix"),
        None => ("inclusive".to_owned(), "default-inclusive"),
    };
    Assignment {
        category,
        category_source,
        level,
        level_source,
    }
}

/// Step 2 of the assignment order: a lemma already in an offence category
/// (true of every `categories` this function is called with, since that
/// list only exists for offence-derived lemmas) that also sits in one of
/// `TOPIC_CATEGORIES`'s topics takes a sense-specific code over the generic
/// one. First table entry whose topic the lemma carries wins.
fn topic_category(code: &str, categories: Option<&[String]>) -> Option<&'static str> {
    let categories = categories?;
    let prefix = format!("{code}:");
    let topics: BTreeSet<&str> = categories
        .iter()
        .filter_map(|category| category.strip_prefix(prefix.as_str()))
        .collect();
    TOPIC_CATEGORIES
        .iter()
        .find(|(topic, _)| topics.contains(topic))
        .map(|(_, category)| *category)
}

/// Step 3 of the assignment order: no sense entry, no topic hit. Checked in
/// the order given, first match wins; anything left over is `qas`.
fn membership_category(offence: &BTreeMap<String, BTreeSet<String>>, lemma: &str) -> &'static str {
    let is_in = |suffix: &str| offence.get(suffix).is_some_and(|set| set.contains(lemma));
    if is_in("ethnic slurs") {
        return "ps";
    }
    if is_in("anti-LGBTQ slurs") {
        return "om";
    }
    if is_in("derogatory terms") {
        return "cds";
    }
    "qas"
}

fn is_strong(offence: &BTreeMap<String, BTreeSet<String>>, lemma: &str) -> bool {
    STRONG_SUFFIXES
        .iter()
        .any(|suffix| offence.get(*suffix).is_some_and(|set| set.contains(lemma)))
}

fn stereotype_flag(category: &str) -> &'static str {
    if STEREOTYPE_CATEGORIES.contains(&category) {
        "yes"
    } else {
        "no"
    }
}

/// Derives `pos` from the Wiktionary part-of-speech categories, falling back
/// to morphology when the lemma carries none (always true for a lemma that
/// entered only through the sense table).
fn assign_pos(language_name: &str, categories: Option<&[String]>, lemma: &str) -> String {
    wiktionary_pos(language_name, categories)
        .unwrap_or_else(|| morphological_pos(lemma))
        .to_owned()
}

fn wiktionary_pos(language_name: &str, categories: Option<&[String]>) -> Option<&'static str> {
    let categories = categories?;
    let prefix = format!("{language_name} ");
    let tags: BTreeSet<&str> = categories
        .iter()
        .filter_map(|category| category.strip_prefix(prefix.as_str()))
        .collect();
    let has_any = |tag_set: &[&str]| tag_set.iter().any(|tag| tags.contains(tag));
    if has_any(NOUN_TAGS) {
        return Some("n");
    }
    if has_any(VERB_TAGS) {
        return Some("v");
    }
    if has_any(ADJECTIVE_TAGS) {
        return Some("a");
    }
    if has_any(ADVERB_TAGS) {
        return Some("r");
    }
    if has_any(INTERJECTION_TAGS) {
        return Some("i");
    }
    None
}

fn morphological_pos(lemma: &str) -> &'static str {
    if lemma.contains(' ') {
        return "p";
    }
    let is_reflexive = REFLEXIVE_VERB_SUFFIXES
        .iter()
        .any(|suffix| lemma.ends_with(suffix));
    let stem = if is_reflexive {
        lemma.strip_suffix("se").unwrap_or(lemma)
    } else {
        lemma
    };
    if stem.ends_with("ar") || stem.ends_with("er") || stem.ends_with("ir") {
        return "v";
    }
    if lemma.ends_with("mente") {
        return "r";
    }
    "n"
}

fn render_tsv(assigned: &[AssignedRow]) -> Vec<u8> {
    let header = "id\tpos\tcategory\tstereotype\tlemma\tlevel".to_owned();
    let lines = assigned.iter().map(|item| {
        let row = &item.row;
        [
            row.id.as_str(),
            row.pos.as_str(),
            row.category.as_str(),
            row.stereotype.as_str(),
            row.lemma.as_str(),
            row.level.as_str(),
        ]
        .join("\t")
    });
    let mut body = std::iter::once(header)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n");
    body.push('\n');
    body.into_bytes()
}

#[derive(Debug, Serialize)]
struct Provenance {
    storage_code: String,
    language_name: String,
    harvest_sha256: String,
    entries: usize,
    entries_detail: Vec<ProvenanceEntry>,
}

#[derive(Debug, Serialize)]
struct ProvenanceEntry {
    id: String,
    lemma: String,
    category: String,
    category_source: &'static str,
    level: String,
    level_source: &'static str,
}

fn render_provenance(
    options: &BuildOptions,
    harvest: &Harvest,
    harvest_bytes: &[u8],
    assigned: &[AssignedRow],
) -> Result<Vec<u8>, LexiconError> {
    let entries_detail = assigned
        .iter()
        .map(|item| ProvenanceEntry {
            id: item.row.id.clone(),
            lemma: item.row.lemma.clone(),
            category: item.row.category.clone(),
            category_source: item.category_source,
            level: item.row.level.clone(),
            level_source: item.level_source,
        })
        .collect();
    let provenance = Provenance {
        storage_code: options.storage_code.clone(),
        language_name: harvest.language_name.clone(),
        harvest_sha256: hex(&Sha256::digest(harvest_bytes)),
        entries: assigned.len(),
        entries_detail,
    };
    Ok(serde_json::to_vec_pretty(&provenance)?)
}
