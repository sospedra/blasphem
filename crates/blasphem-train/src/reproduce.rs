use std::{
    ffi::OsString,
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    compiler::{BatchCompileOptions, compile_model_set},
    corpus::verify_corpus,
    datasets::DatasetId,
    evaluation_lock::parse_evaluation_lock,
    evidence::{Sha256Digest, sha256_digest},
    model_manifest::parse_model_manifest,
    source_manifest::parse_frozen_source_lock,
};

/// The eight ordered reproduction steps.
pub const STEP_NAMES: [&str; 8] = [
    "verify-corpus",
    "verify-sealed-partitions",
    "compile-model-artifacts",
    "rebuild-language-artifact",
    "compare-model-manifest",
    "build-native-binary",
    "build-wasm-modules",
    "run-checks",
];

/// Marks every child process of a reproduction run, so a nested run does not recurse.
pub const REENTRY_GUARD_VARIABLE: &str = "BLASPHEM_REPRODUCE_ACTIVE";

/// The committed corpus: one hand-editable TSV file per language.
pub const CORPUS_ROOT: &str = "corpus";

const SOURCE_LOCK: &str = "resources/datasets/source-lock-v1.json";
const EVALUATION_LOCK: &str = "resources/datasets/evaluation-lock-v1.json";
const HURTLEX_ROOT: &str = "data/clean-room-v1";
const BEHAVIOR_ROOT: &str = "tests/fixtures/behavior";
const LANGUAGE_ARTIFACT_LOCK: &str = "resources/models/language-artifact-v1.json";
const LANGUAGE_ARTIFACT_SCHEMA_VERSION: &str = "language-artifact-v1";
const MODEL_MANIFEST: &str = "resources/models/multilingual-v2/manifest.json";
const VENDOR_ROOT: &str = "crates/blasphem-language/vendor";
const WASM_MODULE: &str = "target/wasm32-unknown-unknown/release/blasphem_wasm.wasm";

const RUST_CHECKS: [&[&str]; 3] = [
    &["test", "--workspace", "--locked"],
    &[
        "clippy",
        "--workspace",
        "--all-targets",
        "--locked",
        "--",
        "-D",
        "warnings",
    ],
    &["fmt", "--all", "--check"],
];

const JAVASCRIPT_CHECKS: [&[&str]; 4] = [
    &["install", "--frozen-lockfile"],
    &["--filter", "blasphem", "run", "build"],
    &["--filter", "blasphem", "run", "pack:check"],
    &["--filter", "blasphem", "run", "test:node"],
];

const BROWSER_SMOKE: &[&str] = &["--filter", "blasphem", "run", "test:browser"];

const WASM_VARIANTS: [(&str, Option<&str>); 2] = [
    ("default", None),
    ("explicit-only", Some("--no-default-features")),
];

/// The inputs of one reproduction run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproduceOptions {
    /// The checked-out repository. The run never writes tracked content into it.
    pub project_root: PathBuf,
    /// The directory that receives every generated artifact.
    pub work_root: PathBuf,
    /// Skips the JavaScript package checks and the browser smoke.
    pub skip_browser: bool,
}

/// One completed reproduction step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepOutcome {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// The completed steps of one reproduction run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReproduceReport {
    pub steps: Vec<StepOutcome>,
}

/// The failure of one named reproduction step.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{step}: {message}")]
pub struct ReproduceError {
    pub step: &'static str,
    pub message: String,
}

type StepResult = Result<String, ReproduceError>;
type Step = fn(&ReproduceOptions) -> StepResult;

const STEPS: [Step; 8] = [
    verify_corpus_step,
    verify_prepared_partitions,
    compile_model_artifacts,
    rebuild_language_artifact,
    compare_model_manifest,
    build_native_binary,
    build_wasm_modules,
    run_checks,
];

/// The leading steps that write generated artifacts into the work root.
pub const GENERATION_STEPS: usize = 4;

/// Runs the eight reproduction steps in order and stops at the first failure.
///
/// # Errors
///
/// Returns the first failing step, naming the file or artifact that failed.
pub fn reproduce(options: &ReproduceOptions) -> Result<ReproduceReport, ReproduceError> {
    run_steps(options, STEPS.len())
}

/// Runs the four generating steps and stops before the comparison steps.
///
/// # Errors
///
/// Returns the first failing step, naming the file or artifact that failed.
pub fn generate_artifacts(options: &ReproduceOptions) -> Result<ReproduceReport, ReproduceError> {
    run_steps(options, GENERATION_STEPS)
}

fn run_steps(options: &ReproduceOptions, count: usize) -> Result<ReproduceReport, ReproduceError> {
    let mut outcomes = Vec::with_capacity(count);
    for (name, step) in STEP_NAMES.iter().zip(STEPS).take(count) {
        let detail = step(options)?;
        outcomes.push(StepOutcome {
            name: (*name).to_owned(),
            passed: true,
            detail,
        });
    }
    Ok(ReproduceReport { steps: outcomes })
}

fn verify_corpus_step(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[0];
    let lock =
        parse_frozen_source_lock(open_project_file(STEP, &options.project_root, SOURCE_LOCK)?)
            .map_err(|error| failure(STEP, error.to_string()))?;
    let evaluation = parse_evaluation_lock(open_project_file(
        STEP,
        &options.project_root,
        EVALUATION_LOCK,
    )?)
    .map_err(|error| failure(STEP, error.to_string()))?;
    let report = verify_corpus(&options.project_root.join(CORPUS_ROOT), &evaluation)
        .map_err(|error| failure(STEP, error.to_string()))?;
    verify_hurtlex_inputs(STEP, options, &lock)?;
    Ok(format!(
        "verified {} corpus rows across {} languages",
        report.rows, report.languages
    ))
}

/// The clean-room lexicon files back the corpus directly, so their digests stay pinned.
fn verify_hurtlex_inputs(
    step: &'static str,
    options: &ReproduceOptions,
    lock: &crate::source_manifest::FrozenSourceLock,
) -> Result<(), ReproduceError> {
    let hurtlex_root = options.project_root.join(HURTLEX_ROOT);
    for source in lock
        .sources
        .iter()
        .filter(|source| source.dataset == DatasetId::HurtLex)
    {
        let relative = source
            .file_path
            .strip_prefix("hurtlex/")
            .unwrap_or(source.file_path.as_str());
        let actual = file_digest(step, &hurtlex_root.join(relative))?;
        if actual != source.file_sha256 {
            return Err(failure(
                step,
                format!(
                    "{HURTLEX_ROOT}/{relative} changed: expected {}, got {actual}",
                    source.file_sha256
                ),
            ));
        }
    }
    Ok(())
}

/// The seal now covers rows in the committed corpus, not whole prepared files.
fn verify_sealed_partitions_in_corpus(
    options: &ReproduceOptions,
    lock: &crate::evaluation_lock::EvaluationLock,
) -> Result<(), crate::corpus::CorpusError> {
    verify_corpus(&options.project_root.join(CORPUS_ROOT), lock).map(|_| ())
}

fn verify_prepared_partitions(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[1];
    let file = open_project_file(STEP, &options.project_root, EVALUATION_LOCK)?;
    let lock = parse_evaluation_lock(file).map_err(|error| failure(STEP, error.to_string()))?;
    verify_sealed_partitions_in_corpus(options, &lock)
        .map_err(|error| failure(STEP, error.to_string()))?;
    Ok(format!(
        "verified {} sealed language partitions",
        lock.languages.len()
    ))
}

fn compile_model_artifacts(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[2];
    let manifest = compile_model_set(&BatchCompileOptions {
        corpus_root: options.project_root.join(CORPUS_ROOT),
        source_lock: options.project_root.join(SOURCE_LOCK),
        hurtlex_root: options.project_root.join(HURTLEX_ROOT),
        behavior_root: Some(options.project_root.join(BEHAVIOR_ROOT)),
        output: model_root(options),
    })
    .map_err(|error| failure(STEP, error.to_string()))?;
    Ok(format!(
        "compiled {} model artifacts",
        manifest.entries.len()
    ))
}

fn rebuild_language_artifact(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[3];
    let lock = read_language_artifact_lock(&options.project_root)?;
    let vendor = options
        .project_root
        .join(VENDOR_ROOT)
        .join(&lock.source_commit);
    verify_vendored_headers(&vendor, &lock)?;
    let rebuilt = options.work_root.join("language.bin");
    let cargo = cargo_program();
    let mut arguments = words(&[
        "run",
        "--release",
        "--locked",
        "-p",
        "blasphem-language",
        "--bin",
        "blasphem-language-model",
        "--",
    ]);
    arguments.push(vendor.into());
    arguments.push(rebuilt.clone().into());
    run_program(
        STEP,
        &ProgramCall {
            program: &cargo,
            arguments,
            directory: &options.project_root,
        },
    )?;
    compare_language_artifact(&rebuilt, &lock)?;
    compare_language_artifact(
        &options.project_root.join(&lock.artifact_relative_path),
        &lock,
    )?;
    Ok(format!(
        "rebuilt {} bytes matching {}",
        lock.artifact_bytes, lock.artifact_sha256
    ))
}

fn compare_model_manifest(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[4];
    let file = open_project_file(STEP, &options.project_root, MODEL_MANIFEST)?;
    let manifest = parse_model_manifest(file).map_err(|error| failure(STEP, error.to_string()))?;
    let models = model_root(options);
    let mut mismatches = Vec::new();
    for entry in &manifest.entries {
        let path = models.join(&entry.artifact_relative_path);
        let Ok(actual) = file_digest(STEP, &path) else {
            mismatches.push(format!(
                "{} ({}) is unreadable at {}",
                entry.language.code(),
                entry.artifact_relative_path,
                path.display()
            ));
            continue;
        };
        if actual != entry.artifact_sha256 {
            mismatches.push(format!(
                "{} ({}) expected {}, got {actual}",
                entry.language.code(),
                entry.artifact_relative_path,
                entry.artifact_sha256
            ));
        }
    }
    if !mismatches.is_empty() {
        return Err(failure(
            STEP,
            format!(
                "{} of {} model artifacts do not match {MODEL_MANIFEST}: {}",
                mismatches.len(),
                manifest.entries.len(),
                mismatches.join("; ")
            ),
        ));
    }
    Ok(format!(
        "matched {} model artifact digests",
        manifest.entries.len()
    ))
}

fn build_native_binary(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[5];
    let cargo = cargo_program();
    run_program(
        STEP,
        &ProgramCall {
            program: &cargo,
            arguments: words(&["build", "--release", "--locked", "--bin", "blasphem"]),
            directory: &options.project_root,
        },
    )?;
    Ok("built the release blasphem binary".to_owned())
}

fn build_wasm_modules(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[6];
    let cargo = cargo_program();
    for (variant, feature_flag) in WASM_VARIANTS {
        let mut arguments = words(&[
            "build",
            "--release",
            "--locked",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            "blasphem-wasm",
        ]);
        arguments.extend(feature_flag.map(OsString::from));
        run_program(
            STEP,
            &ProgramCall {
                program: &cargo,
                arguments,
                directory: &options.project_root,
            },
        )?;
        generate_web_bindings(options, variant)?;
    }
    Ok("built the default and explicit-only browser modules".to_owned())
}

fn generate_web_bindings(options: &ReproduceOptions, variant: &str) -> Result<(), ReproduceError> {
    const STEP: &str = STEP_NAMES[6];
    let mut arguments = Vec::with_capacity(7);
    arguments.push(OsString::from(options.project_root.join(WASM_MODULE)));
    arguments.extend(words(&[
        "--target",
        "web",
        "--out-name",
        "blasphem",
        "--out-dir",
    ]));
    arguments.push(options.work_root.join("wasm").join(variant).into());
    run_program(
        STEP,
        &ProgramCall {
            program: "wasm-bindgen",
            arguments,
            directory: &options.project_root,
        },
    )
}

fn run_checks(options: &ReproduceOptions) -> StepResult {
    const STEP: &str = STEP_NAMES[7];
    let cargo = cargo_program();
    for check in RUST_CHECKS {
        run_program(
            STEP,
            &ProgramCall {
                program: &cargo,
                arguments: words(check),
                directory: &options.project_root,
            },
        )?;
    }
    if options.skip_browser {
        return Ok("ran the Rust checks and skipped the JavaScript checks".to_owned());
    }
    for check in JAVASCRIPT_CHECKS.into_iter().chain([BROWSER_SMOKE]) {
        run_program(
            STEP,
            &ProgramCall {
                program: "pnpm",
                arguments: words(check),
                directory: &options.project_root,
            },
        )?;
    }
    Ok("ran the Rust checks, the JavaScript checks, and the browser smoke".to_owned())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageArtifactLock {
    pub schema_version: String,
    pub artifact_relative_path: String,
    pub artifact_bytes: usize,
    pub artifact_sha256: Sha256Digest,
    pub source_commit: String,
    pub source_headers: Vec<LanguageArtifactHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageArtifactHeader {
    pub file_name: String,
    pub sha256: Sha256Digest,
}

/// Reads the pinned language-artifact lock from one checked-out repository.
///
/// # Errors
///
/// Returns an error for an unreadable, malformed, or wrongly versioned lock.
pub fn read_language_artifact_lock(
    project_root: &Path,
) -> Result<LanguageArtifactLock, ReproduceError> {
    const STEP: &str = STEP_NAMES[3];
    let file = open_project_file(STEP, project_root, LANGUAGE_ARTIFACT_LOCK)?;
    let lock: LanguageArtifactLock = serde_json::from_reader(file).map_err(|error| {
        failure(
            STEP,
            format!("cannot parse {LANGUAGE_ARTIFACT_LOCK}: {error}"),
        )
    })?;
    if lock.schema_version != LANGUAGE_ARTIFACT_SCHEMA_VERSION {
        return Err(failure(
            STEP,
            format!(
                "{LANGUAGE_ARTIFACT_LOCK} has schema version {}, expected {LANGUAGE_ARTIFACT_SCHEMA_VERSION}",
                lock.schema_version
            ),
        ));
    }
    Ok(lock)
}

fn verify_vendored_headers(
    vendor: &Path,
    lock: &LanguageArtifactLock,
) -> Result<(), ReproduceError> {
    const STEP: &str = STEP_NAMES[3];
    for header in &lock.source_headers {
        let path = vendor.join(&header.file_name);
        let actual = file_digest(STEP, &path)?;
        if actual != header.sha256 {
            return Err(failure(
                STEP,
                format!(
                    "{} changed: expected {}, got {actual}",
                    path.display(),
                    header.sha256
                ),
            ));
        }
    }
    Ok(())
}

fn compare_language_artifact(
    path: &Path,
    lock: &LanguageArtifactLock,
) -> Result<(), ReproduceError> {
    const STEP: &str = STEP_NAMES[3];
    let bytes = fs::read(path)
        .map_err(|error| failure(STEP, format!("cannot read {}: {error}", path.display())))?;
    if bytes.len() != lock.artifact_bytes {
        return Err(failure(
            STEP,
            format!(
                "{} holds {} bytes, expected {}",
                path.display(),
                bytes.len(),
                lock.artifact_bytes
            ),
        ));
    }
    let actual = sha256_digest(&bytes);
    if actual != lock.artifact_sha256 {
        return Err(failure(
            STEP,
            format!(
                "{} changed: expected {}, got {actual}",
                path.display(),
                lock.artifact_sha256
            ),
        ));
    }
    Ok(())
}

pub(crate) struct ProgramCall<'a> {
    pub(crate) program: &'a str,
    pub(crate) arguments: Vec<OsString>,
    pub(crate) directory: &'a Path,
}

pub(crate) fn run_program(
    step: &'static str,
    call: &ProgramCall<'_>,
) -> Result<(), ReproduceError> {
    let status = Command::new(call.program)
        .args(&call.arguments)
        .current_dir(call.directory)
        .env(REENTRY_GUARD_VARIABLE, "1")
        .status()
        .map_err(|error| failure(step, format!("cannot run {}: {error}", describe(call))))?;
    if status.success() {
        return Ok(());
    }
    Err(failure(
        step,
        format!("{} failed: {status}", describe(call)),
    ))
}

fn describe(call: &ProgramCall<'_>) -> String {
    let mut line = call.program.to_owned();
    for argument in &call.arguments {
        line.push(' ');
        line.push_str(&argument.to_string_lossy());
    }
    line
}

pub(crate) fn words(values: &[&str]) -> Vec<OsString> {
    values.iter().map(|value| OsString::from(*value)).collect()
}

pub(crate) fn cargo_program() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

pub(crate) fn model_root(options: &ReproduceOptions) -> PathBuf {
    options.work_root.join("models")
}

fn open_project_file(
    step: &'static str,
    project_root: &Path,
    relative: &str,
) -> Result<File, ReproduceError> {
    File::open(project_root.join(relative))
        .map_err(|error| failure(step, format!("cannot read {relative}: {error}")))
}

fn file_digest(step: &'static str, path: &Path) -> Result<Sha256Digest, ReproduceError> {
    let bytes = fs::read(path)
        .map_err(|error| failure(step, format!("cannot read {}: {error}", path.display())))?;
    Ok(sha256_digest(&bytes))
}

pub(crate) fn failure(step: &'static str, message: String) -> ReproduceError {
    ReproduceError { step, message }
}
