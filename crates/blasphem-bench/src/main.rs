use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use blasphem_bench::{
    AutoValidationConfig, collect_size_evidence, run_auto_validation, run_benchmark,
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
    Auto {
        #[arg(long)]
        texts: PathBuf,
        #[arg(long)]
        labels: PathBuf,
        #[arg(long)]
        fixtures: PathBuf,
        #[arg(long)]
        hurtlex_root: PathBuf,
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
        hurtlex_root: PathBuf,
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
        hurtlex_root: PathBuf,
        #[arg(long)]
        target_triple: String,
        #[arg(long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        EvidenceCommand::Auto {
            texts,
            labels,
            fixtures,
            hurtlex_root,
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
                hurtlex_root,
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
            hurtlex_root,
            output,
            computer,
            target_triple,
        } => {
            let rust_version = rust_version()?;
            let evidence = run_benchmark(
                &fixtures,
                &model_manifest,
                &hurtlex_root,
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
            hurtlex_root,
            target_triple,
            output,
        } => {
            let evidence =
                collect_size_evidence(&binary, &model_manifest, &hurtlex_root, &target_triple)?;
            write_canonical(&output, &evidence)?;
            println!(
                "status=measured binary_bytes={} artifacts={} hurtlex={}",
                evidence.binary.bytes,
                evidence.artifacts.len(),
                evidence.hurtlex.len(),
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
