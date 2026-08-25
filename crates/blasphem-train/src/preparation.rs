use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use blasphem::Language;

use crate::{
    datasets::{
        DatasetAdapter, DatasetId, ImportedRow, PreparationPolicy, SourceInput, SourceSplit,
        SplitPolicy, germ_eval_2018::GermEval2018Adapter, ibrohim_budi::IbrohimBudiAdapter,
        kmhas::KMHasAdapter, offenseval_tr::OffensEvalTrAdapter, prepare_language,
        textdetox::TextDetoxAdapter, told_br::ToldBrAdapter, vihos::ViHosAdapter,
    },
    evaluation_lock::{parse_evaluation_lock, verify_sealed_partitions},
    publication::{PreparedPublication, publish_prepared},
    source_manifest::{FrozenSource, parse_frozen_source_lock, parse_source_observation},
};

/// The inputs of one offline corpus preparation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareCorpusOptions {
    pub source_lock: PathBuf,
    pub raw_root: PathBuf,
    pub audit_exclusions: Option<PathBuf>,
    pub evaluation_lock: Option<PathBuf>,
    pub output: PathBuf,
}

/// Imports every frozen source, splits it, and publishes one prepared corpus.
///
/// # Errors
///
/// Returns an error when a source is unreadable, when a language is missing, or when the
/// published output changes a sealed evaluation partition.
pub fn prepare_corpus(options: &PrepareCorpusOptions) -> Result<PreparedPublication> {
    let lock_input = File::open(&options.source_lock)
        .with_context(|| format!("cannot read {}", options.source_lock.display()))?;
    let source_lock = parse_frozen_source_lock(lock_input)?;
    let observation_input = File::open(options.raw_root.join("source-observation-v1.json"))
        .with_context(|| {
            format!(
                "cannot read {}/source-observation-v1.json",
                options.raw_root.display()
            )
        })?;
    let observation = parse_source_observation(observation_input)?;
    crate::acquisition::validate_observation_matches_lock(&observation, &source_lock)?;
    let audit_only = match &options.audit_exclusions {
        Some(path) => read_audit_exclusions(path)?,
        None => BTreeMap::new(),
    };
    let prepared = prepare_every_language(options, &source_lock.sources, audit_only)?;
    let publication = publish_prepared(&options.output, &prepared, &observation)?;
    if let Some(lock_path) = options.evaluation_lock.as_ref() {
        enforce_evaluation_lock(&options.output, lock_path)?;
    }
    Ok(publication)
}

fn prepare_every_language(
    options: &PrepareCorpusOptions,
    sources: &[FrozenSource],
    mut audit_only: BTreeMap<Language, BTreeSet<String>>,
) -> Result<Vec<crate::datasets::PreparedLanguage>> {
    let source_roles = sources
        .iter()
        .map(|source| (source.source_file_id.clone(), source.source_role))
        .collect::<BTreeMap<_, _>>();
    let mut by_language = BTreeMap::new();
    for row in import_all_rows(&options.raw_root, sources)? {
        let language = row
            .detector_language
            .ok_or_else(|| anyhow::anyhow!("imported row has no detector language"))?;
        by_language
            .entry(language)
            .or_insert_with(Vec::new)
            .push(row);
    }
    let mut prepared = Vec::with_capacity(Language::ALL.len());
    for language in Language::ALL {
        let rows = by_language
            .remove(&language)
            .ok_or_else(|| anyhow::anyhow!("prepared input misses language {}", language.code()))?;
        let policy = PreparationPolicy {
            language,
            split_policy: split_policy(language),
            split_version: "fnv1a-uppercase-v1",
            normalization_version: "runtime-normalize-v2",
            audit_only_source_ids: audit_only.remove(&language).unwrap_or_default(),
            source_roles: source_roles.clone(),
        };
        prepared.push(prepare_language(rows, &policy)?);
    }
    if let Some((language, _)) = audit_only.into_iter().next() {
        bail!(
            "audit exclusion has no imported language: {}",
            language.code()
        );
    }
    Ok(prepared)
}

const fn split_policy(language: Language) -> SplitPolicy {
    match language {
        Language::Tr => SplitPolicy::TurkishOfficialTest,
        Language::Vi | Language::Ko => SplitPolicy::PreserveOfficial,
        _ => SplitPolicy::Hash70_15_15,
    }
}

fn enforce_evaluation_lock(prepared_root: &Path, lock_path: &Path) -> Result<()> {
    let file =
        File::open(lock_path).with_context(|| format!("cannot open {}", lock_path.display()))?;
    let lock = parse_evaluation_lock(file).context("cannot parse the evaluation lock")?;
    verify_sealed_partitions(prepared_root, &lock)
        .context("the prepared output changes a sealed evaluation partition")?;
    Ok(())
}

/// Reads the audit exclusion table that marks rule-influencing rows as audit-only.
///
/// # Errors
///
/// Returns an error when the header, a field count, a language, or a repeated identifier is
/// invalid.
pub fn read_audit_exclusions(path: &Path) -> Result<BTreeMap<Language, BTreeSet<String>>> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_path(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let header = reader.headers()?.clone();
    if header
        .iter()
        .ne(["detector_language", "source_id", "reason"])
    {
        bail!("audit exclusion header must be detector_language\\tsource_id\\treason");
    }
    let mut source_ids = BTreeSet::new();
    let mut output = BTreeMap::<Language, BTreeSet<String>>::new();
    for record in reader.records() {
        let record = record?;
        if record.len() != 3 {
            bail!("audit exclusion row must have three fields");
        }
        let language = Language::from_str(record.get(0).unwrap_or_default())
            .map_err(|_| anyhow::anyhow!("audit exclusion has an unknown language"))?;
        let source_id = record.get(1).unwrap_or_default().trim();
        let reason = record.get(2).unwrap_or_default();
        if source_id.is_empty() || reason.trim().is_empty() {
            bail!("audit exclusion source identifier and reason must be nonblank");
        }
        if !source_ids.insert(source_id.to_owned()) {
            bail!("audit exclusion repeats source identifier: {source_id}");
        }
        output
            .entry(language)
            .or_default()
            .insert(source_id.to_owned());
    }
    Ok(output)
}

fn import_all_rows(raw_root: &Path, sources: &[FrozenSource]) -> Result<Vec<ImportedRow>> {
    let mut output = Vec::new();
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::TextDetox,
        &TextDetoxAdapter,
    )?);
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::IbrohimBudi,
        &IbrohimBudiAdapter,
    )?);
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::ToldBr,
        &ToldBrAdapter,
    )?);
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::OffensEvalTr,
        &OffensEvalTrAdapter,
    )?);
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::ViHos,
        &ViHosAdapter,
    )?);
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::KMHas,
        &KMHasAdapter,
    )?);
    output.extend(import_inputs(
        raw_root,
        sources,
        DatasetId::GermEval2018,
        &GermEval2018Adapter,
    )?);
    Ok(output)
}

fn import_inputs(
    raw_root: &Path,
    sources: &[FrozenSource],
    dataset: DatasetId,
    adapter: &impl DatasetAdapter,
) -> Result<Vec<ImportedRow>> {
    let selected = sources
        .iter()
        .filter(|source| source.dataset == dataset)
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    let mut files = selected
        .iter()
        .map(|source| File::open(raw_root.join(&source.file_path)))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut inputs = selected
        .iter()
        .zip(files.iter_mut())
        .map(|(source, reader)| SourceInput {
            source_file_id: source.source_file_id.as_str(),
            source_split: source_split(source.source_file_id.as_str()),
            reader,
        })
        .collect::<Vec<_>>();
    Ok(adapter.import(&mut inputs)?)
}

fn source_split(source_file_id: &str) -> SourceSplit {
    match source_file_id {
        "offenseval-tr-training" | "vihos-train" | "kmhas-train" => SourceSplit::Train,
        "vihos-development" => SourceSplit::Development,
        "kmhas-validation" => SourceSplit::Validation,
        "offenseval-tr-test" | "offenseval-tr-test-labels" | "vihos-test" | "kmhas-test" => {
            SourceSplit::Test
        }
        _ => SourceSplit::Unsplit,
    }
}
