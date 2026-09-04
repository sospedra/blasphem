use std::{fs::File, io::Read, path::PathBuf};

use anyhow::{Context, Result, bail};
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
use blasphem_train::lexicon::{
    BuildOptions, HarvestOptions, WikiSource, build, default_wiki, harvest,
};
use blasphem_train::locales_table::{TableFormat, write_locales_table};
use blasphem_train::pack::{PackOptions, write_packs};
use blasphem_train::preparation::{PrepareCorpusOptions, prepare_corpus};
use blasphem_train::regenerate::{RegenerateOptions, regenerate};
use blasphem_train::reproduce::{ReproduceOptions, reproduce};
use blasphem_train::source_manifest::{
    FrozenSource, SOURCE_OBSERVATION_SCHEMA_VERSION, SourceObservation, parse_frozen_source_lock,
    parse_source_catalog, parse_source_observation,
};
use blasphem_train::verification::{evaluate_behavior, evaluate_cli_smoke, evaluate_validation};
use blasphem_train::versions::{check_versions, sync_versions, workspace_version};
use blasphem_train::{ReqwestTextDetoxClient, TextDetoxHttpClient};
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::blocking::Client;

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
    Compile(CompileArgs),
    /// Audits Spanish validation and optionally compares the fixed learner grid.
    EsRecall(EsRecallArgs),
    Evaluate(EvaluateArgs),
    Behavior(BehaviorArgs),
    CliSmoke(CliSmokeArgs),
    Reproduce(ReproduceArgs),
    Regenerate(RegenerateArgs),
    LexiconHarvest(LexiconHarvestArgs),
    LexiconBuild(LexiconBuildArgs),
    Pack(PackArgs),
    LocalesTable(LocalesTableArgs),
    SyncVersions(SyncVersionsArgs),
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
    /// Directory containing `{STORAGE_CODE}.tsv` lexicon files.
    #[arg(long)]
    lexicon_root: PathBuf,
    #[arg(long, default_value = "crates/blasphem/tests/fixtures/behavior")]
    behavior_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct EsRecallArgs {
    /// A new directory for artifacts and validation evidence. Never reads test predictions.
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    sweep: bool,
    /// Audits this artifact instead of the committed Spanish artifact.
    #[arg(long)]
    artifact: Option<PathBuf>,
    /// The logistic learner's development document-frequency floor.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    minimum_document_frequency: u32,
    /// Compares two fixed NB-logistic settings from Wang and Manning (2012).
    #[arg(long, conflicts_with = "sweep")]
    nb_logistic: bool,
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
    lexicon_root: PathBuf,
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
    lexicon_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct CliSmokeArgs {
    #[arg(long)]
    model_manifest: PathBuf,
    #[arg(long)]
    lexicon_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
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
    /// Native wiki host, for example "tr.wiktionary.org". Some languages
    /// carry no meaningful offence categories on en.wiktionary.org and must
    /// harvest from their own wiki instead. Harvested first; the
    /// en.wiktionary.org derivation still runs as a secondary source.
    #[arg(long)]
    native_host: Option<String>,
    /// Full category titles on the native wiki, for example
    /// "Kategori:Türkçe argo". Repeat the flag for more than one category.
    /// Ignored when `native_host` is not set.
    #[arg(long = "native-category")]
    native_categories: Vec<String>,
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

#[derive(Debug, Args)]
struct PackArgs {
    #[arg(long)]
    model_manifest: PathBuf,
    #[arg(long)]
    model_root: PathBuf,
    #[arg(long)]
    language_model: PathBuf,
    #[arg(long)]
    lexicon_root: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

#[derive(Debug, Args)]
struct SyncVersionsArgs {
    /// Fail when a manifest disagrees instead of rewriting it.
    #[arg(long)]
    check: bool,
}

#[derive(Debug, Args)]
struct LocalesTableArgs {
    #[arg(long)]
    output: PathBuf,
    /// ts, go, python, swift, or kotlin
    #[arg(long, default_value = "ts")]
    format: String,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Observe(arguments) => observe_sources(&arguments),
        Command::FreezeSources(arguments) => freeze_sources_command(&arguments),
        Command::Acquire(arguments) => acquire_sources_command(&arguments),
        Command::Prepare(arguments) => prepare_sources_command(&arguments),
        Command::CorpusVerify(arguments) => corpus_verify_command(&arguments),
        Command::Compile(arguments) => compile_models(&arguments),
        Command::EsRecall(arguments) => spanish_recall::run(&arguments),
        Command::Evaluate(arguments) => evaluate_evidence(&arguments),
        Command::Behavior(arguments) => behavior_evidence(&arguments),
        Command::CliSmoke(arguments) => cli_smoke_evidence(&arguments),
        Command::Reproduce(arguments) => reproduce_repository(&arguments),
        Command::Regenerate(arguments) => regenerate_repository(&arguments),
        Command::LexiconHarvest(arguments) => lexicon_harvest_command(&arguments),
        Command::LexiconBuild(arguments) => lexicon_build_command(&arguments),
        Command::Pack(arguments) => pack_command(&arguments),
        Command::LocalesTable(arguments) => locales_table_command(&arguments),
        Command::SyncVersions(arguments) => sync_versions_command(&arguments),
    }
}

mod spanish_recall {
    use std::{fs, fs::File, path::Path, time::Instant};

    use anyhow::{Context, Result, ensure};
    use blasphem::{
        ConfusionMatrix, EvalLabel, Judge, Language, LexiconMatch, NudgeDetector, NudgeResult,
        PackInput, PackSource, ReplyTarget, RuleChannel, RuleOutcome, SparseModel, encode_pack,
        lexicon_marked_text,
    };
    use blasphem_train::{
        BehaviorRow,
        calibration::gates_for_language,
        compiler::{CompileRequest, Learner, LogisticOptions, compile_language_with_learner},
        corpus::load_corpus_language,
        datasets::PreparedRow,
        load_panel,
        model_manifest::rule_pack_version,
        source_manifest::parse_frozen_source_lock,
    };
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    use super::EsRecallArgs;

    const BASELINE_ARTIFACT: &str = "resources/models/multilingual-v2/es-sparse-v2.bin";
    const MODEL_MANIFEST: &str = "resources/models/multilingual-v2/manifest.json";
    const SOURCE_LOCK: &str = "crates/blasphem-train/metadata/source-lock-v1.json";

    pub(super) fn run(arguments: &EsRecallArgs) -> Result<()> {
        fs::create_dir(&arguments.output).context("the evidence directory must be new")?;
        println!(
            "ES rule_sha256={:x}",
            Sha256::digest(blasphem::canonical_rule_identity(Language::Es))
        );
        let experiment = Experiment::load(&arguments.output)?;
        experiment.write_input_hashes()?;
        fs::write(
            arguments.output.join("settings.json"),
            serde_json::to_vec_pretty(&json!({
                "sweep": arguments.sweep,
                "minimum_document_frequency": arguments.minimum_document_frequency,
                "nb_logistic": arguments.nb_logistic,
                "nb_cost": 1.0,
                "nb_interpolations": [1.0, 0.25],
                "nb_minimum_document_frequency": 2,
            }))?,
        )?;
        let artifact_path = arguments
            .artifact
            .as_deref()
            .unwrap_or(Path::new(BASELINE_ARTIFACT));
        let baseline = fs::read(artifact_path)?;
        let mut summaries = vec![experiment.evaluate("baseline", &baseline)?];
        if arguments.sweep {
            for (name, learner) in learners(arguments.minimum_document_frequency) {
                summaries.push(experiment.train_candidate(&name, learner)?);
            }
        }
        if arguments.nb_logistic {
            for interpolation in [1.0, 0.25] {
                let learner = Learner::NaiveBayesLogistic {
                    cost: 1.0,
                    interpolation,
                };
                summaries.push(
                    experiment
                        .train_candidate(&format!("nb-logistic-beta-{interpolation}"), learner)?,
                );
            }
        }
        fs::write(
            arguments.output.join("summary.json"),
            serde_json::to_vec_pretty(&summaries)?,
        )?;
        Ok(())
    }

    struct Experiment<'a> {
        output: &'a Path,
        request: CompileRequest,
        lexicon: Vec<u8>,
        panel: Vec<BehaviorRow>,
        rule_pack_version: u16,
    }

    impl<'a> Experiment<'a> {
        fn load(output: &'a Path) -> Result<Self> {
            let lock = parse_frozen_source_lock(File::open(SOURCE_LOCK)?)?;
            let input = load_corpus_language(Path::new("corpus"), Language::Es, &lock)?;
            let lexicon = fs::read("lexicon/ES.tsv")?;
            let panel = load_panel(
                Path::new("crates/blasphem/tests/fixtures/behavior"),
                Language::Es,
            )?;
            let rule_pack_version = rule_pack_version(Language::Es);
            let request = CompileRequest {
                language: Language::Es,
                development: input.development,
                validation: input.validation,
                rule_channel: RuleChannel::from_lexicon_bytes(Language::Es, Some(&lexicon))?,
                clean_controls: panel
                    .iter()
                    .filter(|row| !row.expected_nudge)
                    .map(|row| row.text.clone())
                    .collect(),
            };
            Ok(Self {
                output,
                request,
                lexicon,
                panel,
                rule_pack_version,
            })
        }

        fn write_input_hashes(&self) -> Result<()> {
            let mut hashes = serde_json::Map::new();
            for path in [
                "corpus/ES.tsv",
                "lexicon/ES.tsv",
                "crates/blasphem-train/metadata/evaluation-lock-v1.json",
                SOURCE_LOCK,
                MODEL_MANIFEST,
                BASELINE_ARTIFACT,
                "crates/blasphem/tests/fixtures/behavior/es.tsv",
                "crates/blasphem/tests/fixtures/spanish-audit.tsv",
                "crates/blasphem-train/src/compiler.rs",
                "crates/blasphem-train/src/calibration.rs",
                "crates/blasphem-train/src/main.rs",
                "crates/blasphem-train/src/corpus.rs",
                "crates/blasphem-train/src/model_manifest.rs",
                "crates/blasphem/src/features.rs",
                "crates/blasphem/src/detector.rs",
                "crates/blasphem/src/rules/channel.rs",
                "crates/blasphem/src/policy.rs",
                "crates/blasphem/src/rule_pack.rs",
                "crates/blasphem/src/runtime.rs",
                "crates/blasphem/src/judge.rs",
                "crates/blasphem/src/sparse.rs",
                "crates/blasphem/src/registry.rs",
                "crates/blasphem/src/embedded.rs",
                "Cargo.lock",
                "rust-toolchain.toml",
                "resources/models/multilingual-v2/es-sparse-v2.bin",
            ] {
                hashes.insert(
                    path.to_owned(),
                    json!(format!("{:x}", Sha256::digest(fs::read(path)?))),
                );
            }
            fs::write(
                self.output.join("input-hashes.json"),
                serde_json::to_vec_pretty(&hashes)?,
            )?;
            Ok(())
        }

        fn evaluate(&self, name: &str, artifact: &[u8]) -> Result<Value> {
            Candidate::new(self, artifact)?.write_evidence(name, artifact)
        }

        fn train_candidate(&self, name: &str, learner: Learner) -> Result<Value> {
            let started = Instant::now();
            let compiled = match compile_language_with_learner(&self.request, learner) {
                Ok(compiled) => compiled,
                Err(error) => {
                    eprintln!("{name}: {error}");
                    return Ok(json!({"name": name, "error": error.to_string()}));
                }
            };
            let training_ms = started.elapsed().as_secs_f64() * 1000.0;
            let mut summary = self.evaluate(name, &compiled.artifact)?;
            ensure!(
                summary["matrix"] == serde_json::to_value(compiled.calibration.matrix)?,
                "public verdict matrix differs from calibration for {name}"
            );
            summary["training_ms"] = json!(training_ms);
            Ok(summary)
        }
    }

    fn learners(minimum_document_frequency: u32) -> impl Iterator<Item = (String, Learner)> {
        let logistic = [false, true].into_iter().flat_map(move |class_weighted| {
            [0.05, 0.15, 0.5, 1.0, 2.0].into_iter().map(move |cost| {
                let weighting = if class_weighted {
                    "balanced"
                } else {
                    "unweighted"
                };
                (
                    format!("logistic-{weighting}-{cost}"),
                    Learner::Logistic(LogisticOptions {
                        cost,
                        class_weighted,
                        minimum_document_frequency,
                    }),
                )
            })
        });
        std::iter::once(("log-odds".to_owned(), Learner::LogOdds)).chain(logistic)
    }

    struct Candidate<'a> {
        experiment: &'a Experiment<'a>,
        model: SparseModel,
        judge: Judge,
        detector: NudgeDetector,
    }

    impl<'a> Candidate<'a> {
        fn new(experiment: &'a Experiment<'a>, artifact: &[u8]) -> Result<Self> {
            let model = SparseModel::from_bytes(artifact)?;
            let pack = encode_pack(&PackInput {
                language: Language::Es,
                rule_pack_version: experiment.rule_pack_version,
                artifact,
                lexicon: &experiment.lexicon,
            });
            let source = PackSource {
                language: Language::Es,
                pack: &pack,
                pack_sha256: None,
                detect: None,
                detect_sha256: None,
            };
            let judge = Judge::from_packs(&[source], false, false)?;
            let detector = NudgeDetector::from_pack(Language::Es, &pack)?;
            Ok(Self {
                experiment,
                model,
                judge,
                detector,
            })
        }

        fn inspect(&self, text: &str) -> Result<Observation> {
            let nudge = self.detector.check(text, ReplyTarget::Unknown);
            let rules = self
                .experiment
                .request
                .rule_channel
                .analyze(text, ReplyTarget::Unknown);
            let lexicon = self
                .experiment
                .request
                .rule_channel
                .lexicon()
                .context("Spanish lexicon missing")?;
            let hits = lexicon.check(text).matches;
            let marked = lexicon_marked_text(text, &hits);
            Ok(Observation {
                nudge,
                rules,
                hits,
                raw: self.model.raw_score(&marked),
                model_score: self.model.score(&marked),
            })
        }

        fn validation(&self) -> Result<(ConfusionMatrix, Vec<Value>)> {
            let mut matrix = ConfusionMatrix::default();
            let mut rows = Vec::with_capacity(self.experiment.request.validation.len());
            for row in &self.experiment.request.validation {
                let observation = self.inspect(&row.text)?;
                let predicted = !self.judge.judge(&row.text).safe;
                ensure!(
                    predicted == observation.nudge.should_nudge,
                    "judge and detector disagree"
                );
                ensure!(
                    predicted == observation.calibrated_flag(self.model.raw_boundary()),
                    "public verdict and calibration disagree"
                );
                observation.count(&mut matrix, row.label);
                rows.push(observation.validation_row(row));
            }
            Ok((matrix, rows))
        }

        fn controls(&self) -> Result<(i32, usize, Vec<Value>)> {
            let mut floor = i32::MIN;
            let mut failures = 0;
            let mut rows = Vec::with_capacity(self.experiment.panel.len());
            for row in &self.experiment.panel {
                let observation = self.inspect(&row.text)?;
                if !row.expected_nudge && !observation.rules.suppresses_sparse_channel() {
                    floor = floor.max(observation.raw.saturating_add(1));
                }
                failures += usize::from(row.expected_nudge != observation.nudge.should_nudge);
                rows.push(observation.control_row(row));
            }
            Ok((floor, failures, rows))
        }

        fn timings(&self) -> Vec<f64> {
            let mut durations = Vec::with_capacity(7);
            for _ in 0..7 {
                let started = Instant::now();
                for row in &self.experiment.request.validation {
                    std::hint::black_box(self.judge.judge(std::hint::black_box(&row.text)));
                }
                durations.push(
                    started.elapsed().as_secs_f64() * 1_000_000.0
                        / self.experiment.request.validation.len() as f64,
                );
            }
            durations.sort_by(f64::total_cmp);
            durations
        }

        fn write_evidence(&self, name: &str, artifact: &[u8]) -> Result<Value> {
            let (matrix, rows) = self.validation()?;
            let (floor, behavior_failures, controls) = self.controls()?;
            let frozen_score = self
                .detector
                .check("No te voy a matar", ReplyTarget::Unknown)
                .score;
            let durations = self.timings();
            let summary = json!({
                "name": name, "matrix": matrix, "boundary": self.model.raw_boundary(),
                "scale": self.model.score_scale(), "clean_control_floor": floor,
                "behavior_failures": behavior_failures, "frozen_negation_score": frozen_score,
                "artifact_bytes": artifact.len(), "artifact_sha256": format!("{:x}", Sha256::digest(artifact)),
                "gates": gates_for_language(Language::Es, matrix),
                "judge_median_us": durations[3], "judge_us_runs": durations,
            });
            let output = self.experiment.output;
            fs::write(output.join(format!("{name}.bin")), artifact)?;
            fs::write(
                output.join(format!("{name}-validation.json")),
                serde_json::to_vec_pretty(&rows)?,
            )?;
            fs::write(
                output.join(format!("{name}-controls.json")),
                serde_json::to_vec_pretty(&controls)?,
            )?;
            println!("{}", serde_json::to_string(&summary)?);
            Ok(summary)
        }
    }

    struct Observation {
        nudge: NudgeResult,
        rules: RuleOutcome,
        hits: Vec<LexiconMatch>,
        raw: i32,
        model_score: u8,
    }

    impl Observation {
        fn calibrated_flag(&self, boundary: i32) -> bool {
            self.rules.should_nudge
                || (!self.rules.suppresses_sparse_channel() && self.raw >= boundary)
        }

        fn count(&self, matrix: &mut ConfusionMatrix, label: EvalLabel) {
            match (label, self.nudge.should_nudge) {
                (EvalLabel::Toxic, true) => matrix.true_positive += 1,
                (EvalLabel::Toxic, false) => matrix.false_negative += 1,
                (EvalLabel::Clean, true) => matrix.false_positive += 1,
                (EvalLabel::Clean, false) => matrix.true_negative += 1,
            }
        }

        fn validation_row(&self, row: &PreparedRow) -> Value {
            json!({
                "id": row.source_id, "label": if row.label == EvalLabel::Toxic { "toxic" } else { "clean" },
                "text": row.text, "characters": row.text.chars().count(), "flag": self.nudge.should_nudge,
                "score": self.nudge.score, "model_score": self.model_score, "raw": self.raw,
                "rule_score": self.rules.score, "rule_flag": self.rules.should_nudge,
                "suppressed": self.rules.suppresses_sparse_channel(),
                "rule_evidence": format!("{:?}", self.rules.evidence),
                "lexicon_hits": self.hits.iter().map(|hit| hit.entry.lemma.as_str()).collect::<Vec<_>>(),
            })
        }

        fn control_row(&self, row: &BehaviorRow) -> Value {
            json!({"text": row.text, "expected": row.expected_nudge,
                "flag": self.nudge.should_nudge, "score": self.nudge.score,
                "raw": self.raw, "suppressed": self.rules.suppresses_sparse_channel()})
        }
    }
}

fn sync_versions_command(arguments: &SyncVersionsArgs) -> Result<()> {
    let root = std::env::current_dir().context("cannot read the current directory")?;
    let version = workspace_version(&root)?;
    let report = if arguments.check {
        check_versions(&root)?
    } else {
        sync_versions(&root)?
    };
    println!(
        "status={} version={version} checked={} changed={}",
        if arguments.check { "checked" } else { "synced" },
        report.checked,
        report.changed
    );
    Ok(())
}

fn pack_command(arguments: &PackArgs) -> Result<()> {
    let options = PackOptions {
        model_manifest: arguments.model_manifest.clone(),
        model_root: arguments.model_root.clone(),
        language_model: arguments.language_model.clone(),
        lexicon_root: arguments.lexicon_root.clone(),
        output: arguments.output.clone(),
    };
    let report = write_packs(&options).context("cannot write the packs")?;
    println!(
        "status=packed locales={} files={} bytes={} output={}",
        report.locales,
        report.files,
        report.bytes,
        arguments.output.display()
    );
    Ok(())
}

fn locales_table_command(arguments: &LocalesTableArgs) -> Result<()> {
    let format = TableFormat::parse(&arguments.format).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown --format {:?}; use ts, go, python, swift, or kotlin",
            arguments.format
        )
    })?;
    write_locales_table(&arguments.output, format)
        .with_context(|| format!("cannot write {}", arguments.output.display()))?;
    println!("status=written path={}", arguments.output.display());
    Ok(())
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
    let mut wikis = Vec::new();
    if let Some(host) = arguments.native_host.clone() {
        wikis.push(WikiSource {
            host,
            categories: arguments.native_categories.clone(),
            strong: Vec::new(),
        });
    }
    wikis.push(default_wiki(&arguments.language_name));
    let options = HarvestOptions {
        language_name: arguments.language_name.clone(),
        storage_code: arguments.storage_code.clone(),
        wikis,
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
        lexicon_root: arguments.lexicon_root.clone(),
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
                &arguments.lexicon_root,
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
        &arguments.lexicon_root,
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
    let evidence = evaluate_cli_smoke(&arguments.model_manifest, &arguments.lexicon_root)
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
        File::open("crates/blasphem-train/metadata/source-catalog-v1.json")
            .context("cannot read crates/blasphem-train/metadata/source-catalog-v1.json")?,
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

fn one_line(value: &str) -> String {
    value.chars().flat_map(char::escape_debug).collect()
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
