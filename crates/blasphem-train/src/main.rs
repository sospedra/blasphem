use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use blasphem::{
    ConfusionMatrix, LevelSelection, Metrics, PolicyAction, evaluate_policy, load_lexica,
};
use blasphem_train::acquisition::{
    MAX_SOURCE_DOWNLOAD_BYTES, current_unix_seconds, extract_archive_member, freeze_observation,
    source_record_from_request_with_download, validate_catalog,
    validate_observation_matches_catalog, validate_source_download,
    validate_source_lock_for_acquisition, validate_textdetox_download_identity,
    write_acquired_sources, write_frozen_source_lock, write_source_observation,
};
use blasphem_train::compiler::{BatchCompileOptions, compile_model_set};
use blasphem_train::corpus::verify_corpus;
use blasphem_train::evaluation_lock::parse_evaluation_lock;
use blasphem_train::evidence::write_canonical_json;
use blasphem_train::lexicon::{BuildOptions, HarvestOptions, build, default_wiki, harvest};
use blasphem_train::preparation::{PrepareCorpusOptions, prepare_corpus};
use blasphem_train::regenerate::{RegenerateOptions, regenerate};
use blasphem_train::reproduce::{ReproduceOptions, reproduce};
use blasphem_train::source_manifest::{
    FrozenSource, SOURCE_OBSERVATION_SCHEMA_VERSION, SourceObservation, parse_frozen_source_lock,
    parse_source_catalog, parse_source_observation,
};
use blasphem_train::verification::{evaluate_behavior, evaluate_cli_smoke, evaluate_validation};
use blasphem_train::{ReqwestTextDetoxClient, TextDetoxHttpClient, parse_eval_rows};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;

const DEFAULT_LANGUAGES: &str = "EN,ES,FR,DE,IT,PT,RU,AR";
const ALL_LANGUAGES: &[&str] = &[
    "AF", "AR", "BG", "BN", "CA", "CS", "CY", "DA", "DE", "EL", "EN", "EO", "ES", "ET", "EU", "FA",
    "FI", "FR", "GA", "GL", "HE", "HI", "HR", "HU", "ID", "IS", "IT", "JA", "KO", "LT", "LV", "MK",
    "MS", "MT", "NL", "NO", "PL", "PT", "RO", "RU", "SIMPLE", "SK", "SL", "SQ", "SR", "SV", "SW",
    "TH", "TL", "TR", "UK", "VI", "ZH",
];

#[derive(Debug, Parser)]
#[command(
    name = "blasphem-train",
    about = "Offline multilingual dataset pipeline"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Observe(ObserveArgs),
    FreezeSources(FreezeSourcesArgs),
    Acquire(AcquireArgs),
    Prepare(PrepareArgs),
    CorpusVerify(CorpusVerifyArgs),
    Setup(SetupArgs),
    Compile(CompileArgs),
    Evaluate(EvaluateArgs),
    Behavior(BehaviorArgs),
    CliSmoke(CliSmokeArgs),
    Eval(EvalArgs),
    Reproduce(ReproduceArgs),
    Regenerate(RegenerateArgs),
    LexiconHarvest(LexiconHarvestArgs),
    LexiconBuild(LexiconBuildArgs),
}

#[derive(Debug, Args)]
struct ObserveArgs {
    #[arg(long)]
    source_catalog: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct FreezeSourcesArgs {
    #[arg(long)]
    observation: PathBuf,
    #[arg(long)]
    reviewed: bool,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct AcquireArgs {
    #[arg(long)]
    source_lock: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct PrepareArgs {
    #[arg(long)]
    source_lock: PathBuf,
    #[arg(long)]
    raw_root: PathBuf,
    #[arg(long)]
    audit_exclusions: Option<PathBuf>,
    #[arg(long)]
    evaluation_lock: Option<PathBuf>,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct CorpusVerifyArgs {
    #[arg(long)]
    corpus_root: PathBuf,
    #[arg(long)]
    evaluation_lock: PathBuf,
}

#[derive(Debug, Args)]
struct CompileArgs {
    #[arg(long)]
    corpus_root: PathBuf,
    #[arg(long)]
    source_lock: PathBuf,
    #[arg(long)]
    hurtlex_root: PathBuf,
    #[arg(long, default_value = "tests/fixtures/behavior")]
    behavior_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct EvaluateArgs {
    #[arg(long, value_enum)]
    split: EvaluationSplitArg,
    #[arg(long)]
    corpus_root: PathBuf,
    #[arg(long)]
    model_manifest: PathBuf,
    #[arg(long)]
    hurtlex_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum EvaluationSplitArg {
    Validation,
}

#[derive(Debug, Args)]
struct BehaviorArgs {
    #[arg(long)]
    fixture_root: PathBuf,
    #[arg(long)]
    corpus_root: PathBuf,
    #[arg(long)]
    model_manifest: PathBuf,
    #[arg(long)]
    hurtlex_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct CliSmokeArgs {
    #[arg(long)]
    model_manifest: PathBuf,
    #[arg(long)]
    hurtlex_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct SetupArgs {
    #[arg(long, default_value = DEFAULT_LANGUAGES)]
    languages: String,
    #[arg(long, default_value = "data/hurtlex")]
    data_dir: PathBuf,
}

#[derive(Debug, Args)]
struct EvalArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long, default_value = "data/hurtlex")]
    data_dir: PathBuf,
    #[arg(long, value_enum, default_value_t = MinimumActionArg::Review)]
    minimum_action: MinimumActionArg,
}

#[derive(Debug, Args)]
struct ReproduceArgs {
    /// The directory that holds generated data. Defaults to a temporary directory.
    #[arg(long)]
    work_root: Option<PathBuf>,
    /// Skips the npm and browser checks.
    #[arg(long)]
    skip_browser: bool,
}

#[derive(Debug, Args)]
struct RegenerateArgs {
    /// The directory that holds generated data. Defaults to a temporary directory.
    #[arg(long)]
    work_root: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct LexiconHarvestArgs {
    #[arg(long)]
    language_name: String,
    #[arg(long)]
    storage_code: String,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct LexiconBuildArgs {
    #[arg(long)]
    harvest: PathBuf,
    #[arg(long)]
    storage_code: String,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MinimumActionArg {
    Review,
    Block,
}

impl From<MinimumActionArg> for PolicyAction {
    fn from(value: MinimumActionArg) -> Self {
        match value {
            MinimumActionArg::Review => Self::Review,
            MinimumActionArg::Block => Self::Block,
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Observe(arguments) => observe_sources(&arguments),
        Command::FreezeSources(arguments) => freeze_sources_command(&arguments),
        Command::Acquire(arguments) => acquire_sources_command(&arguments),
        Command::Prepare(arguments) => prepare_sources_command(&arguments),
        Command::CorpusVerify(arguments) => corpus_verify_command(&arguments),
        Command::Setup(arguments) => setup(&arguments),
        Command::Compile(arguments) => compile_models(&arguments),
        Command::Evaluate(arguments) => evaluate_evidence(&arguments),
        Command::Behavior(arguments) => behavior_evidence(&arguments),
        Command::CliSmoke(arguments) => cli_smoke_evidence(&arguments),
        Command::Eval(arguments) => eval(&arguments),
        Command::Reproduce(arguments) => reproduce_repository(&arguments),
        Command::Regenerate(arguments) => regenerate_repository(&arguments),
        Command::LexiconHarvest(arguments) => lexicon_harvest_command(&arguments),
        Command::LexiconBuild(arguments) => lexicon_build_command(&arguments),
    }
}

fn reproduce_repository(arguments: &ReproduceArgs) -> Result<()> {
    let project_root = std::env::current_dir().context("cannot read the current directory")?;
    if let Some(work_root) = arguments.work_root.clone() {
        return report_reproduction(project_root, work_root, arguments.skip_browser);
    }
    let temporary = tempfile::tempdir().context("cannot create a reproduction work directory")?;
    report_reproduction(
        project_root,
        temporary.path().to_owned(),
        arguments.skip_browser,
    )
}

fn report_reproduction(
    project_root: PathBuf,
    work_root: PathBuf,
    skip_browser: bool,
) -> Result<()> {
    let report = reproduce(&ReproduceOptions {
        project_root,
        work_root,
        skip_browser,
    })?;
    println!("status=reproduced steps={}", report.steps.len());
    Ok(())
}

fn regenerate_repository(arguments: &RegenerateArgs) -> Result<()> {
    let project_root = std::env::current_dir().context("cannot read the current directory")?;
    if let Some(work_root) = arguments.work_root.clone() {
        return report_regeneration(project_root, work_root);
    }
    let temporary = tempfile::tempdir().context("cannot create a regeneration work directory")?;
    report_regeneration(project_root, temporary.path().to_owned())
}

fn report_regeneration(project_root: PathBuf, work_root: PathBuf) -> Result<()> {
    let report = regenerate(&RegenerateOptions {
        project_root,
        work_root,
    })?;
    for file in report.files.iter().filter(|file| file.changed) {
        println!(
            "status=rewrote path={} sha256={}",
            one_line(&file.relative_path),
            file.sha256
        );
    }
    println!(
        "status=regenerated files={} changed={}",
        report.files.len(),
        report.changed()
    );
    Ok(())
}

fn lexicon_harvest_command(arguments: &LexiconHarvestArgs) -> Result<()> {
    let options = HarvestOptions {
        language_name: arguments.language_name.clone(),
        storage_code: arguments.storage_code.clone(),
        wikis: vec![default_wiki(&arguments.language_name)],
        output: arguments.output.clone(),
    };
    let report = harvest(&options).context("cannot harvest the wiktionary lexicon")?;
    println!(
        "status=harvested language={} lemmas={} sha256={}",
        arguments.storage_code, report.lemmas, report.sha256
    );
    Ok(())
}

fn lexicon_build_command(arguments: &LexiconBuildArgs) -> Result<()> {
    let options = BuildOptions {
        harvest_root: arguments.harvest.clone(),
        storage_code: arguments.storage_code.clone(),
        output: arguments.output.clone(),
    };
    let report = build(&options).context("cannot build the offline lexicon")?;
    println!(
        "status=built language={} entries={} identity_entries={} sha256={}",
        arguments.storage_code, report.entries, report.identity_entries, report.sha256
    );
    Ok(())
}

fn compile_models(arguments: &CompileArgs) -> Result<()> {
    let manifest = compile_model_set(&BatchCompileOptions {
        corpus_root: arguments.corpus_root.clone(),
        source_lock: arguments.source_lock.clone(),
        hurtlex_root: arguments.hurtlex_root.clone(),
        behavior_root: Some(arguments.behavior_root.clone()),
        output: arguments.output.clone(),
    })
    .context("cannot compile the multilingual model set")?;
    println!(
        "status=compiled languages={} output={}",
        manifest.entries.len(),
        one_line(&arguments.output.to_string_lossy())
    );
    Ok(())
}

fn evaluate_evidence(arguments: &EvaluateArgs) -> Result<()> {
    match arguments.split {
        EvaluationSplitArg::Validation => {
            let evidence = evaluate_validation(
                &arguments.corpus_root,
                &arguments.model_manifest,
                &arguments.hurtlex_root,
            )
            .context("cannot create validation calibration evidence")?;
            write_canonical_json(&arguments.output, &evidence)
                .context("cannot write validation calibration evidence")?;
            println!(
                "status=calibration_evidence languages={} output={}",
                evidence.languages.len(),
                one_line(&arguments.output.to_string_lossy()),
            );
            Ok(())
        }
    }
}

fn behavior_evidence(arguments: &BehaviorArgs) -> Result<()> {
    let evidence = evaluate_behavior(
        &arguments.fixture_root,
        &arguments.corpus_root,
        &arguments.model_manifest,
        &arguments.hurtlex_root,
    )
    .context("cannot create behavior contract evidence")?;
    let failures = evidence
        .languages
        .values()
        .flat_map(|language| &language.cases)
        .filter(|case| !case.passed)
        .map(|case| case.case_id.as_str())
        .collect::<Vec<_>>();
    if !failures.is_empty() {
        bail!(
            "behavior contract failed {} cases: {}",
            failures.len(),
            failures.join(",")
        );
    }
    write_canonical_json(&arguments.output, &evidence)
        .context("cannot write behavior contract evidence")?;
    println!(
        "status=behavior_contract_evidence cases=360 output={}",
        one_line(&arguments.output.to_string_lossy()),
    );
    Ok(())
}

fn cli_smoke_evidence(arguments: &CliSmokeArgs) -> Result<()> {
    let evidence = evaluate_cli_smoke(&arguments.model_manifest, &arguments.hurtlex_root)
        .context("cannot create native CLI smoke evidence")?;
    let failures = evidence
        .languages
        .values()
        .flat_map(|language| &language.cases)
        .filter(|case| !case.passed)
        .map(|case| case.case_id.as_str())
        .collect::<Vec<_>>();
    if !failures.is_empty() {
        bail!(
            "native CLI smoke failed {} cases: {}",
            failures.len(),
            failures.join(",")
        );
    }
    write_canonical_json(&arguments.output, &evidence)
        .context("cannot write native CLI smoke evidence")?;
    println!(
        "status=native_cli_smoke_evidence cases=60 output={}",
        one_line(&arguments.output.to_string_lossy()),
    );
    Ok(())
}

fn observe_sources(arguments: &ObserveArgs) -> Result<()> {
    if arguments.output.exists() {
        bail!(
            "source output already exists: {}",
            arguments.output.display()
        );
    }
    let input = File::open(&arguments.source_catalog)
        .with_context(|| format!("cannot read {}", arguments.source_catalog.display()))?;
    let catalog = parse_source_catalog(input)?;
    validate_catalog(&catalog)?;
    let client = Client::builder()
        .user_agent("blasphem-train/0.1")
        .build()
        .context("cannot build the HTTP client")?;
    let acquired_at = current_unix_seconds()?;
    let mut records = Vec::with_capacity(catalog.sources.len());
    for request in &catalog.sources {
        let source = observe_source(&client, request)?;
        let record = source_record_from_request_with_download(
            request,
            request.requested_url.clone(),
            source.revision,
            source
                .downloaded_bytes
                .as_deref()
                .unwrap_or(&source.canonical_bytes),
            &source.canonical_bytes,
            acquired_at,
        )?;
        records.push(record);
    }
    let observation = SourceObservation {
        schema_version: SOURCE_OBSERVATION_SCHEMA_VERSION.to_owned(),
        sources: records,
    };
    write_source_observation(&arguments.output, &observation)?;
    println!(
        "status=observed sources={} output={}",
        observation.sources.len(),
        one_line(&arguments.output.to_string_lossy())
    );
    Ok(())
}

fn corpus_verify_command(arguments: &CorpusVerifyArgs) -> Result<()> {
    let evaluation = parse_evaluation_lock(
        File::open(&arguments.evaluation_lock)
            .with_context(|| format!("cannot read {}", arguments.evaluation_lock.display()))?,
    )?;
    let report = verify_corpus(&arguments.corpus_root, &evaluation)?;
    println!(
        "status=verified languages={} rows={}",
        report.languages, report.rows
    );
    Ok(())
}

fn freeze_sources_command(arguments: &FreezeSourcesArgs) -> Result<()> {
    if !arguments.reviewed {
        bail!("freeze-sources requires --reviewed after human source and license review");
    }
    let input = File::open(&arguments.observation)
        .with_context(|| format!("cannot read {}", arguments.observation.display()))?;
    let observation = parse_source_observation(input)?;
    let catalog = parse_source_catalog(
        File::open("resources/datasets/source-catalog-v1.json")
            .context("cannot read resources/datasets/source-catalog-v1.json")?,
    )?;
    validate_observation_matches_catalog(&observation, &catalog)?;
    let source_lock = freeze_observation(observation)?;
    write_frozen_source_lock(&arguments.output, &source_lock)?;
    println!(
        "status=frozen sources={} output={}",
        source_lock.sources.len(),
        one_line(&arguments.output.to_string_lossy())
    );
    Ok(())
}

fn acquire_sources_command(arguments: &AcquireArgs) -> Result<()> {
    if arguments.output.exists() {
        bail!(
            "source output already exists: {}",
            arguments.output.display()
        );
    }
    let input = File::open(&arguments.source_lock)
        .with_context(|| format!("cannot read {}", arguments.source_lock.display()))?;
    let source_lock = parse_frozen_source_lock(input)?;
    validate_source_lock_for_acquisition(&source_lock)?;
    let client = Client::builder()
        .user_agent("blasphem-train/0.1")
        .build()
        .context("cannot build the HTTP client")?;
    let mut files = Vec::with_capacity(source_lock.sources.len());
    for source in &source_lock.sources {
        let bytes = acquire_frozen_source(&client, source)?;
        files.push((source.source_file_id.clone(), bytes));
    }
    let observation = write_acquired_sources(
        &arguments.output,
        &source_lock,
        files,
        current_unix_seconds()?,
    )?;
    println!(
        "status=acquired sources={} output={}",
        observation.sources.len(),
        one_line(&arguments.output.to_string_lossy())
    );
    Ok(())
}

fn prepare_sources_command(arguments: &PrepareArgs) -> Result<()> {
    let publication = prepare_corpus(&PrepareCorpusOptions {
        source_lock: arguments.source_lock.clone(),
        raw_root: arguments.raw_root.clone(),
        audit_exclusions: arguments.audit_exclusions.clone(),
        evaluation_lock: arguments.evaluation_lock.clone(),
        output: arguments.output.clone(),
    })?;
    println!(
        "status=prepared source_rows={} excluded={} audit_only={} output={}",
        publication.manifest.source_rows,
        publication
            .manifest
            .inclusion_status_counts
            .get("excluded")
            .copied()
            .unwrap_or(0),
        publication
            .manifest
            .exclusion_reason_counts
            .get("audit_only")
            .copied()
            .unwrap_or(0),
        one_line(&arguments.output.to_string_lossy())
    );
    Ok(())
}

struct CanonicalSource {
    canonical_bytes: Vec<u8>,
    downloaded_bytes: Option<Vec<u8>>,
    revision: Option<String>,
}

trait TextDetoxDownloadBoundary {
    fn download(&mut self, url: &str) -> Result<Vec<u8>>;
}

struct ReqwestTextDetoxDownload {
    client: ReqwestTextDetoxClient,
}

impl ReqwestTextDetoxDownload {
    fn new(client: &Client) -> Self {
        Self {
            client: ReqwestTextDetoxClient::new(client.clone()),
        }
    }
}

impl TextDetoxDownloadBoundary for ReqwestTextDetoxDownload {
    fn download(&mut self, url: &str) -> Result<Vec<u8>> {
        Ok(self.client.get(url)?.body)
    }
}

fn observe_source(
    client: &Client,
    request: &blasphem_train::source_manifest::SourceRequest,
) -> Result<CanonicalSource> {
    if request.dataset == blasphem_train::datasets::DatasetId::TextDetox {
        return observe_textdetox_source(request, &mut ReqwestTextDetoxDownload::new(client));
    }
    let bytes = download_bytes(client, &request.requested_url)?;
    let revision = match &request.revision_url {
        Some(url) => Some(read_revision_document(&download_bytes(client, url)?)?),
        None => request.requested_revision.clone(),
    };
    if let Some(requested) = &request.requested_revision
        && revision.as_deref() != Some(requested)
    {
        bail!("source revision does not match the requested revision");
    }
    Ok(CanonicalSource {
        canonical_bytes: bytes,
        downloaded_bytes: None,
        revision,
    })
}

fn observe_textdetox_source(
    request: &blasphem_train::source_manifest::SourceRequest,
    downloader: &mut impl TextDetoxDownloadBoundary,
) -> Result<CanonicalSource> {
    let source_code = request
        .source_file_id
        .strip_prefix("textdetox-")
        .ok_or_else(|| anyhow::anyhow!("invalid TextDetox source identifier"))?;
    let revision = validate_textdetox_download_identity(
        &request.source_file_id,
        request.requested_revision.as_deref(),
        &request.requested_url,
    )?;
    let parquet_bytes = downloader.download(&request.requested_url)?;
    let rows = blasphem_train::parse_textdetox_parquet(&parquet_bytes, source_code, revision)?;
    let mut canonical_bytes = Vec::new();
    blasphem_train::datasets::textdetox::write_textdetox_source_tsv(&mut canonical_bytes, &rows)?;
    Ok(CanonicalSource {
        canonical_bytes,
        downloaded_bytes: Some(parquet_bytes),
        revision: Some(revision.to_owned()),
    })
}

fn acquire_frozen_source(client: &Client, source: &FrozenSource) -> Result<Vec<u8>> {
    let bytes = if source.dataset == blasphem_train::datasets::DatasetId::TextDetox {
        acquire_textdetox_source(source, &mut ReqwestTextDetoxDownload::new(client))?
    } else {
        download_bytes(client, &source.immutable_source_url)?
    };
    Ok(match source.archive_member.as_deref() {
        Some(member) => extract_archive_member(&bytes, member)?,
        None => bytes,
    })
}

fn acquire_textdetox_source(
    source: &FrozenSource,
    downloader: &mut impl TextDetoxDownloadBoundary,
) -> Result<Vec<u8>> {
    let source_code = source
        .source_file_id
        .strip_prefix("textdetox-")
        .ok_or_else(|| anyhow::anyhow!("invalid TextDetox source identifier"))?;
    let revision = validate_textdetox_download_identity(
        &source.source_file_id,
        source.revision.as_deref(),
        &source.immutable_source_url,
    )?;
    let parquet_bytes = downloader.download(&source.immutable_source_url)?;
    validate_source_download(source, &parquet_bytes)?;
    let rows = blasphem_train::parse_textdetox_parquet(&parquet_bytes, source_code, revision)?;
    let mut output = Vec::new();
    blasphem_train::datasets::textdetox::write_textdetox_source_tsv(&mut output, &rows)?;
    Ok(output)
}

fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("cannot download {url}"))?
        .error_for_status()
        .with_context(|| format!("source returned an error for {url}"))?;
    let mut bytes = Vec::new();
    response
        .take(MAX_SOURCE_DOWNLOAD_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("cannot read source response from {url}"))?;
    if bytes.len() > MAX_SOURCE_DOWNLOAD_BYTES {
        bail!("source response exceeds {MAX_SOURCE_DOWNLOAD_BYTES} bytes");
    }
    Ok(bytes)
}

fn read_revision_document(bytes: &[u8]) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct RevisionDocument {
        sha: String,
    }
    let revision = serde_json::from_slice::<RevisionDocument>(bytes)?
        .sha
        .trim()
        .to_owned();
    if revision.is_empty() {
        bail!("source revision is blank");
    }
    Ok(revision)
}

fn setup(arguments: &SetupArgs) -> Result<()> {
    let languages = parse_languages(&arguments.languages)?;
    fs::create_dir_all(&arguments.data_dir).with_context(|| {
        format!(
            "cannot create data directory {}",
            arguments.data_dir.display()
        )
    })?;
    let client = Client::builder()
        .user_agent("blasphem-experimental/0.1")
        .build()
        .context("cannot build the HTTP client")?;

    for language in languages {
        let path = arguments.data_dir.join(format!("hurtlex_{language}.tsv"));
        if path.exists() {
            println!(
                "status=existing language={language} path={}",
                one_line(&path.to_string_lossy())
            );
            continue;
        }
        let url = hurtlex_url(&language);
        let bytes = client
            .get(&url)
            .send()
            .with_context(|| format!("cannot download {url}"))?
            .error_for_status()
            .with_context(|| format!("HurtLex returned an error for {url}"))?
            .bytes()
            .with_context(|| format!("cannot read the response from {url}"))?;
        fs::write(&path, &bytes).with_context(|| format!("cannot write {}", path.display()))?;
        println!(
            "status=downloaded language={language} path={}",
            one_line(&path.to_string_lossy())
        );
    }
    Ok(())
}

fn one_line(value: &str) -> String {
    value.chars().flat_map(char::escape_debug).collect()
}

fn eval(arguments: &EvalArgs) -> Result<()> {
    let input = File::open(&arguments.input)
        .with_context(|| format!("cannot read {}", arguments.input.display()))?;
    let rows = parse_eval_rows(input)?;
    let languages: Vec<String> = rows
        .iter()
        .map(|row| row.language.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let entries = load_lexica(
        &arguments.data_dir,
        &languages,
        LevelSelection::Conservative,
    )?;
    let report = evaluate_policy(&rows, entries, arguments.minimum_action.into())?;

    print_matrix("overall", report.overall);
    for (language, matrix) in report.by_language {
        print_matrix(&language, matrix);
    }
    Ok(())
}

fn parse_languages(value: &str) -> Result<Vec<String>> {
    if value.trim().eq_ignore_ascii_case("all") {
        return Ok(ALL_LANGUAGES
            .iter()
            .map(|language| (*language).to_owned())
            .collect());
    }
    let languages: BTreeSet<String> = value
        .split(',')
        .map(str::trim)
        .filter(|language| !language.is_empty())
        .map(str::to_ascii_uppercase)
        .collect();
    if languages.is_empty()
        || languages.iter().any(|language| {
            !language
                .chars()
                .all(|character| character.is_ascii_alphabetic())
        })
    {
        bail!("languages must be comma-separated alphabetic codes");
    }
    Ok(languages.into_iter().collect())
}

fn hurtlex_url(language: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/valeriobasile/hurtlex/refs/heads/master/lexica/{language}/1.2/hurtlex_{language}.tsv"
    )
}

fn print_matrix(scope: &str, matrix: ConfusionMatrix) {
    let n = matrix
        .true_positive
        .saturating_add(matrix.true_negative)
        .saturating_add(matrix.false_positive)
        .saturating_add(matrix.false_negative);
    let Metrics {
        accuracy,
        precision,
        recall,
        specificity,
        f1,
    } = matrix.metrics();
    println!(
        "scope={} n={n} tp={} tn={} fp={} fn={} accuracy={} precision={} recall={} specificity={} f1={}",
        one_line(scope),
        matrix.true_positive,
        matrix.true_negative,
        matrix.false_positive,
        matrix.false_negative,
        display_metric(accuracy),
        display_metric(precision),
        display_metric(recall),
        display_metric(specificity),
        display_metric(f1),
    );
}

fn display_metric(value: Option<f64>) -> String {
    value.map_or_else(|| "N/A".to_owned(), |value| format!("{value:.3}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use blasphem::Language;
    use blasphem_train::{
        acquisition::{frozen_source_from_record, source_record_from_request_with_download},
        datasets::{DatasetId, LineageStatus},
        source_manifest::SourceRequest,
        source_role::SourceRole,
    };
    use parquet::{
        data_type::{ByteArray, ByteArrayType, Int64Type},
        file::writer::SerializedFileWriter,
        schema::parser::parse_message_type,
    };

    use super::{TextDetoxDownloadBoundary, acquire_textdetox_source, observe_textdetox_source};

    #[test]
    fn observe_and_acquire_each_download_one_parquet_file_per_textdetox_source() {
        let revision = blasphem_train::TEXTDETOX_REVISION;
        let url = format!(
            "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/{revision}/data/en-00000-of-00001.parquet"
        );
        let request = SourceRequest {
            dataset: DatasetId::TextDetox,
            detector_language: Language::En,
            source_role: SourceRole::Baseline,
            source_file_id: "textdetox-en".to_owned(),
            requested_url: url.clone(),
            revision_url: None,
            requested_revision: Some(revision.to_owned()),
            archive_member: None,
            file_path: "textdetox/en.tsv".to_owned(),
            license_id: "CC-BY-4.0".to_owned(),
            license_url: "https://example.test/license".to_owned(),
            license_year: 2024,
            citation: "Fixture citation".to_owned(),
            upstream_lineage: vec!["https://example.test/source".to_owned()],
            lineage_status: LineageStatus::Resolved,
        };
        let parquet = parquet_fixture();
        let mut observe_download = CountingDownload::new(parquet.clone());

        let observed = observe_textdetox_source(&request, &mut observe_download)
            .expect("observe TextDetox source");

        assert_eq!(observe_download.urls, [url.as_str()]);
        let record = source_record_from_request_with_download(
            &request,
            url.clone(),
            observed.revision.clone(),
            observed
                .downloaded_bytes
                .as_deref()
                .expect("download bytes"),
            &observed.canonical_bytes,
            1,
        )
        .expect("source record");
        let source = frozen_source_from_record(&record);
        let mut acquire_download = CountingDownload::new(parquet);

        let canonical = acquire_textdetox_source(&source, &mut acquire_download)
            .expect("acquire TextDetox source");

        assert_eq!(acquire_download.urls, [url.as_str()]);
        assert_eq!(canonical, observed.canonical_bytes);
    }

    struct CountingDownload {
        bytes: Vec<u8>,
        urls: Vec<String>,
    }

    impl CountingDownload {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                bytes,
                urls: Vec::new(),
            }
        }
    }

    impl TextDetoxDownloadBoundary for CountingDownload {
        fn download(&mut self, url: &str) -> anyhow::Result<Vec<u8>> {
            self.urls.push(url.to_owned());
            Ok(self.bytes.clone())
        }
    }

    fn parquet_fixture() -> Vec<u8> {
        let schema = Arc::new(
            parse_message_type(concat!(
                "message schema {",
                " REQUIRED BYTE_ARRAY text (STRING);",
                " REQUIRED INT64 toxic;",
                " }"
            ))
            .expect("schema"),
        );
        let mut bytes = Vec::new();
        let mut writer =
            SerializedFileWriter::new(&mut bytes, schema, Default::default()).expect("writer");
        let mut row_group = writer.next_row_group().expect("row group");
        let mut text_writer = row_group
            .next_column()
            .expect("text column")
            .expect("text column exists");
        text_writer
            .typed::<ByteArrayType>()
            .write_batch(&[ByteArray::from("exact text")], None, None)
            .expect("write text");
        text_writer.close().expect("close text");
        let mut label_writer = row_group
            .next_column()
            .expect("label column")
            .expect("label column exists");
        label_writer
            .typed::<Int64Type>()
            .write_batch(&[0], None, None)
            .expect("write label");
        label_writer.close().expect("close label");
        row_group.close().expect("close row group");
        writer.close().expect("close file");
        bytes
    }
}
