use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use blasphem::Language;
use serde::Serialize;
use thiserror::Error;

use crate::{
    atomic_publish::{AtomicPublishError, atomic_publish_replacing},
    calibration::GateResult,
    evidence::{Sha256Digest, sha256_digest},
    model_manifest::{ModelManifest, ModelSetError, parse_model_manifest},
    reproduce::{
        CORPUS_ROOT, ProgramCall, ReproduceError, ReproduceOptions, cargo_program,
        generate_artifacts, model_manifest_path, model_root, read_language_artifact_lock,
        run_program, words,
    },
};

/// The step that publishes reviewed artifacts over the committed ones.
pub const PUBLICATION_STEP: &str = "publish-reviewed-artifacts";
/// The step that writes generated evidence reports outside Git.
pub const EVIDENCE_STEP: &str = "write-evidence-reports";

const MODEL_ROOT: &str = "resources/models";
const MODEL_MANIFEST: &str = "resources/metadata/model-manifest.json";
const LANGUAGE_ARTIFACT_LOCK: &str = "resources/metadata/language-artifact.json";
const LEXICON_ROOT: &str = "resources/lexicon";
const BEHAVIOR_ROOT: &str = "crates/blasphem/tests/fixtures/behavior";
const REPORT_ROOT: &str = "reports";
/// One evidence report and the subcommand that writes it.
struct EvidenceReport {
    file_name: &'static str,
    subcommand: &'static str,
    prepared: bool,
    fixtures: bool,
}

const EVIDENCE_REPORTS: [EvidenceReport; 3] = [
    EvidenceReport {
        file_name: "multilingual-validation.json",
        subcommand: "evaluate",
        prepared: true,
        fixtures: false,
    },
    EvidenceReport {
        file_name: "multilingual-behavior.json",
        subcommand: "behavior",
        prepared: true,
        fixtures: true,
    },
    EvidenceReport {
        file_name: "multilingual-cli-smoke.json",
        subcommand: "cli-smoke",
        prepared: false,
        fixtures: false,
    },
];

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// The inputs of one regeneration run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegenerateOptions {
    /// The checked-out repository that receives the reviewed artifacts.
    pub project_root: PathBuf,
    /// The directory that receives every generated intermediate.
    pub work_root: PathBuf,
}

/// One committed file the run considered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedFile {
    pub relative_path: String,
    pub changed: bool,
    pub sha256: Sha256Digest,
}

/// The outcome of one regeneration run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegenerateReport {
    pub files: Vec<PublishedFile>,
}

impl RegenerateReport {
    #[must_use]
    pub fn changed(&self) -> usize {
        self.files.iter().filter(|file| file.changed).count()
    }
}

/// The failure of one regeneration run.
#[derive(Debug, Error)]
pub enum RegenerateError {
    #[error(transparent)]
    Generate(#[from] ReproduceError),
    #[error("cannot read {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot publish {}: {source}", path.display())]
    Publish {
        path: PathBuf,
        #[source]
        source: AtomicPublishError,
    },
    #[error("cannot serialize {relative}: {source}")]
    Serialize {
        relative: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot parse {}: {source}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot read the compiled model manifest: {0}")]
    CompiledManifest(#[from] ModelSetError),
    #[error("{} fails a validation gate: {gates:?}", language.code())]
    ValidationGate {
        language: Language,
        gates: GateResult,
    },
    #[error("{} reports no validation gates", .0.code())]
    MissingValidationGate(Language),
}

/// Regenerates committed models and locks, plus untracked evidence reports.
///
/// # Errors
///
/// Returns an error for a failing generation step, a failing validation gate, or a failed write.
pub fn regenerate(options: &RegenerateOptions) -> Result<RegenerateReport, RegenerateError> {
    let reproduce = ReproduceOptions {
        project_root: options.project_root.clone(),
        work_root: options.work_root.clone(),
        skip_browser: true,
    };
    generate_artifacts(&reproduce)?;
    let manifest = read_compiled_manifest(&reproduce)?;
    check_validation_gates(&manifest)?;

    let mut files = publish_model_set(options, &reproduce, &manifest)?;
    files.push(publish_language_artifact(options, &reproduce)?);
    files.push(publish_language_lock(options, &reproduce)?);
    files.extend(publish_evidence_reports(options, &reproduce)?);
    Ok(RegenerateReport { files })
}

fn read_compiled_manifest(reproduce: &ReproduceOptions) -> Result<ModelManifest, RegenerateError> {
    let bytes = read_file(&model_manifest_path(reproduce))?;
    Ok(parse_model_manifest(bytes.as_slice())?)
}

fn check_validation_gates(manifest: &ModelManifest) -> Result<(), RegenerateError> {
    for entry in &manifest.entries {
        let gates = entry
            .validation_gates
            .ok_or(RegenerateError::MissingValidationGate(entry.language))?;
        if !gates.passed() {
            return Err(RegenerateError::ValidationGate {
                language: entry.language,
                gates,
            });
        }
    }
    Ok(())
}

fn publish_model_set(
    options: &RegenerateOptions,
    reproduce: &ReproduceOptions,
    manifest: &ModelManifest,
) -> Result<Vec<PublishedFile>, RegenerateError> {
    let compiled = model_root(reproduce);
    let mut files = Vec::with_capacity(manifest.entries.len() + 1);
    for entry in &manifest.entries {
        let relative = format!("{MODEL_ROOT}/{}", entry.artifact_relative_path);
        let bytes = read_file(&compiled.join(&entry.artifact_relative_path))?;
        files.push(publish_bytes(options, &relative, &bytes)?);
    }
    let manifest_bytes = read_file(&model_manifest_path(reproduce))?;
    files.push(publish_bytes(options, MODEL_MANIFEST, &manifest_bytes)?);
    Ok(files)
}

fn publish_language_artifact(
    options: &RegenerateOptions,
    reproduce: &ReproduceOptions,
) -> Result<PublishedFile, RegenerateError> {
    let lock = read_language_artifact_lock(&options.project_root)?;
    let bytes = read_file(&reproduce.work_root.join("language.bin"))?;
    let relative = lock.artifact_relative_path.clone();
    publish_bytes(options, &relative, &bytes)
}

fn publish_language_lock(
    options: &RegenerateOptions,
    reproduce: &ReproduceOptions,
) -> Result<PublishedFile, RegenerateError> {
    let mut lock = read_language_artifact_lock(&options.project_root)?;
    let bytes = read_file(&reproduce.work_root.join("language.bin"))?;
    lock.artifact_bytes = bytes.len();
    lock.artifact_sha256 = sha256_digest(&bytes);
    publish_json(options, LANGUAGE_ARTIFACT_LOCK, &lock)
}

fn publish_evidence_reports(
    options: &RegenerateOptions,
    reproduce: &ReproduceOptions,
) -> Result<Vec<PublishedFile>, RegenerateError> {
    let cargo = cargo_program();
    let staged_root = reproduce.work_root.join(REPORT_ROOT);
    let mut files = Vec::with_capacity(EVIDENCE_REPORTS.len());
    for report in &EVIDENCE_REPORTS {
        let staged = staged_root.join(report.file_name);
        run_program(
            EVIDENCE_STEP,
            &ProgramCall {
                program: &cargo,
                arguments: evidence_arguments(reproduce, report, &staged),
                directory: &options.project_root,
            },
        )?;
        let bytes = read_file(&staged)?;
        let relative = format!("{REPORT_ROOT}/{}", report.file_name);
        files.push(publish_bytes(options, &relative, &bytes)?);
    }
    Ok(files)
}

fn evidence_arguments(
    reproduce: &ReproduceOptions,
    report: &EvidenceReport,
    staged: &Path,
) -> Vec<std::ffi::OsString> {
    let mut arguments = words(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "blasphem-train",
        "--",
        report.subcommand,
    ]);
    if report.subcommand == "evaluate" {
        arguments.extend(words(&["--split", "validation"]));
    }
    if report.fixtures {
        arguments.extend(words(&["--fixture-root", BEHAVIOR_ROOT]));
    }
    if report.prepared {
        arguments.push("--corpus-root".into());
        arguments.push(reproduce.project_root.join(CORPUS_ROOT).into());
    }
    arguments.extend(words(&[
        "--model-manifest",
        MODEL_MANIFEST,
        "--lexicon-root",
        LEXICON_ROOT,
        "--output",
    ]));
    arguments.push(staged.into());
    arguments
}

fn publish_json<T: Serialize>(
    options: &RegenerateOptions,
    relative: &'static str,
    value: &T,
) -> Result<PublishedFile, RegenerateError> {
    let destination = options.project_root.join(relative);
    if json_value_is_current(&destination, value)? {
        return Ok(PublishedFile {
            relative_path: relative.to_owned(),
            changed: false,
            sha256: file_digest(&destination)?,
        });
    }
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|source| RegenerateError::Serialize { relative, source })?;
    bytes.push(b'\n');
    publish_bytes(options, relative, &bytes)
}

fn json_value_is_current<T: Serialize>(path: &Path, value: &T) -> Result<bool, RegenerateError> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(false);
    };
    let Ok(committed) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Ok(false);
    };
    let generated = serde_json::to_value(value).map_err(|source| RegenerateError::Parse {
        path: path.to_owned(),
        source,
    })?;
    Ok(committed == generated)
}

fn publish_bytes(
    options: &RegenerateOptions,
    relative: &str,
    bytes: &[u8],
) -> Result<PublishedFile, RegenerateError> {
    let destination = options.project_root.join(relative);
    let sha256 = sha256_digest(bytes);
    if fs::read(&destination).is_ok_and(|current| current == bytes) {
        return Ok(PublishedFile {
            relative_path: relative.to_owned(),
            changed: false,
            sha256,
        });
    }
    let staged = stage_file(&destination, bytes)?;
    atomic_publish_replacing(&staged, &destination).map_err(|source| RegenerateError::Publish {
        path: destination,
        source,
    })?;
    Ok(PublishedFile {
        relative_path: relative.to_owned(),
        changed: true,
        sha256,
    })
}

fn stage_file(destination: &Path, bytes: &[u8]) -> Result<PathBuf, RegenerateError> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| RegenerateError::Write {
        path: parent.to_owned(),
        source,
    })?;
    let name = destination
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let staged = parent.join(format!(
        ".{name}.regenerate-{}-{sequence}",
        std::process::id()
    ));
    fs::write(&staged, bytes).map_err(|source| RegenerateError::Write {
        path: staged.clone(),
        source,
    })?;
    Ok(staged)
}

fn read_file(path: &Path) -> Result<Vec<u8>, RegenerateError> {
    fs::read(path).map_err(|source| RegenerateError::Read {
        path: path.to_owned(),
        source,
    })
}

fn file_digest(path: &Path) -> Result<Sha256Digest, RegenerateError> {
    Ok(sha256_digest(&read_file(path)?))
}
