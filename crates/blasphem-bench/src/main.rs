use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use blasphem_bench::{
    AccuracyConfig, AccuracyRun, AutoValidationConfig, collect_size_evidence, print_comparison,
    run_accuracy, run_auto_validation, run_benchmark,
};
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    name = "blasphem-bench",
    about = "Experimental blasphem runtime evidence"
)]
struct Cli {
    #[command(subcommand)]
    command: EvidenceCommand,
}

#[derive(Debug, Subcommand)]
enum EvidenceCommand {
    /// Retrain, rebuild, and judge the shipped binary over the corpus test split.
    Accuracy {
        /// Repository root. Defaults to the current directory.
        #[arg(long)]
        project_root: Option<PathBuf>,
        /// Measure this binary as is, skipping retrain and rebuild.
        #[arg(long)]
        binary: Option<PathBuf>,
        /// The commit the measured assets come from.
        #[arg(long, default_value = "HEAD")]
        commit: String,
        /// Run name. Defaults to the short commit.
        #[arg(long)]
        label: Option<String>,
        /// Defaults to reports/benchmarks/<label>.json.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Defaults to crates/blasphem-bench/baseline.json.
        #[arg(long)]
        baseline: Option<PathBuf>,
        /// Validation report to read. Defaults to reports/multilingual-validation.json.
        #[arg(long)]
        validation_report: Option<PathBuf>,
    },
    Auto {
        #[arg(long)]
        texts: PathBuf,
        #[arg(long)]
        labels: PathBuf,
        #[arg(long)]
        fixtures: PathBuf,
        #[arg(long)]
        lexicon_root: PathBuf,
        #[arg(long)]
        model_manifest: PathBuf,
        #[arg(long)]
        native_binary: PathBuf,
        #[arg(long)]
        language_model_artifact: PathBuf,
        #[arg(long)]
        browser_report: PathBuf,
        #[arg(long)]
        c_parity_fixture: PathBuf,
        #[arg(long)]
        project_root: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        computer: String,
        #[arg(long)]
        target_triple: String,
    },
    Benchmark {
        #[arg(long)]
        fixtures: PathBuf,
        #[arg(long)]
        model_manifest: PathBuf,
        #[arg(long)]
        lexicon_root: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        computer: String,
        #[arg(long)]
        target_triple: String,
    },
    Size {
        #[arg(long)]
        binary: PathBuf,
        #[arg(long)]
        model_manifest: PathBuf,
        #[arg(long)]
        lexicon_root: PathBuf,
        #[arg(long)]
        target_triple: String,
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        EvidenceCommand::Accuracy {
            project_root,
            binary,
            commit,
            label,
            output,
            baseline,
            validation_report,
        } => {
            let project_root = match project_root {
                Some(root) => root,
                None => std::env::current_dir().context("cannot read the current directory")?,
            };
            let run = run_accuracy(&AccuracyConfig {
                project_root: project_root.clone(),
                binary,
                commit,
                label,
                validation_report,
            })?;
            let output = output.unwrap_or_else(|| {
                project_root
                    .join("reports/benchmarks")
                    .join(format!("{}.json", run.label))
            });
            write_canonical(&output, &run)?;
            println!(
                "status=measured languages={} retrained={} validation_pooled_recall={:?} test_pooled_recall={:?} output={}",
                run.validation.languages.len(),
                run.retrained,
                run.validation.pooled.metrics.recall,
                run.test.pooled.metrics.recall,
                output.display(),
            );
            let baseline = baseline
                .unwrap_or_else(|| project_root.join("crates/blasphem-bench/baseline.json"));
            if baseline.exists() && baseline.canonicalize().ok() != output.canonicalize().ok() {
                let bytes = fs::read(&baseline)
                    .with_context(|| format!("cannot read baseline {}", baseline.display()))?;
                let baseline: AccuracyRun = serde_json::from_slice(&bytes)
                    .with_context(|| format!("cannot parse baseline {}", baseline.display()))?;
                print_comparison(&baseline, &run);
            }
        }
        EvidenceCommand::Auto {
            texts,
            labels,
            fixtures,
            lexicon_root,
            model_manifest,
            native_binary,
            language_model_artifact,
            browser_report,
            c_parity_fixture,
            project_root,
            output,
            computer,
            target_triple,
        } => {
            let evidence = run_auto_validation(&AutoValidationConfig {
                texts,
                labels,
                fixtures,
                lexicon_root,
                model_manifest,
                native_binary,
                language_model_artifact,
                browser_report,
                c_parity_fixture,
                project_root,
                computer,
                rust_version: rust_version()?,
                target_triple,
            })?;
            write_canonical(&output, &evidence)?;
            println!(
                "status=measured rows={} correct={} supported_unknown={} supported_misrouted={} unsupported_unknown={} unsupported_routed={}",
                evidence.corpus.rows,
                evidence.supported.correct,
                evidence.supported.unknown,
                evidence.supported.misrouted,
                evidence.unsupported.rejected_as_unknown,
                evidence.unsupported.falsely_routed,
            );
        }
        EvidenceCommand::Benchmark {
            fixtures,
            model_manifest,
            lexicon_root,
            output,
            computer,
            target_triple,
        } => {
            let rust_version = rust_version()?;
            let evidence = run_benchmark(
                &fixtures,
                &model_manifest,
                &lexicon_root,
                &computer,
                &target_triple,
                &rust_version,
            )?;
            write_canonical(&output, &evidence)?;
            println!(
                "status=measured fixtures={} latency_gates_passed={} peak_rss_bytes={}",
                evidence.fixtures.len(),
                evidence.all_latency_gates_passed,
                evidence.peak_rss_bytes,
            );
        }
        EvidenceCommand::Size {
            binary,
            model_manifest,
            lexicon_root,
            target_triple,
            output,
        } => {
            let evidence =
                collect_size_evidence(&binary, &model_manifest, &lexicon_root, &target_triple)?;
            write_canonical(&output, &evidence)?;
            println!(
                "status=measured binary_bytes={} artifacts={} lexicon={}",
                evidence.binary.bytes,
                evidence.artifacts.len(),
                evidence.lexicon.len(),
            );
        }
    }
    Ok(())
}

fn rust_version() -> Result<String> {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .context("cannot run rustc --version")?;
    if !output.status.success() {
        bail!("rustc --version failed with {}", output.status);
    }
    String::from_utf8(output.stdout)
        .context("rustc --version returned invalid UTF-8")
        .map(|version| version.trim().to_owned())
}

fn write_canonical(path: &PathBuf, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create output directory {}", parent.display()))?;
    }
    let bytes = serde_jcs::to_vec(value).context("cannot serialize canonical evidence")?;
    fs::write(path, bytes).with_context(|| format!("cannot write {}", path.display()))
}
