//! Accuracy evidence: the shipped binary judged over the corpus test split.

use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use blasphem::{ConfusionMatrix, EvalLabel, Language, Metrics};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::sha256_hex;

pub const CORPUS_ROOT: &str = "corpus";
pub const MODEL_MANIFEST: &str = "resources/models/multilingual-v2/manifest.json";
pub const EMBEDDED_SOURCE: &str = "crates/blasphem/src/embedded.rs";
pub const NATIVE_BINARY: &str = "target/release/blasphem";
pub const VALIDATION_REPORT: &str = "reports/multilingual-validation.json";
const CORPUS_HEADER: &str = "split\tlabel\ttext";
const TEST_SPLIT: &str = "test";
const VALIDATION_SPLIT: &str = "validation";
const DIGEST_HEX_LENGTH: usize = 64;

#[derive(Debug, Error)]
pub enum AccuracyError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("corpus {path} line {line}: {reason}")]
    Corpus {
        path: PathBuf,
        line: usize,
        reason: String,
    },
    #[error("cannot run `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("`{command}` failed with {status}: {stderr}")]
    CommandFailed {
        command: String,
        status: String,
        stderr: String,
    },
    #[error("{language} verdicts: {reason}")]
    Verdicts {
        language: &'static str,
        reason: String,
    },
    #[error("model manifest: {0}")]
    Manifest(String),
    #[error("validation report {path}: {reason}")]
    ValidationReport { path: PathBuf, reason: String },
    #[error("{EMBEDDED_SOURCE}: {0}")]
    EmbeddedSource(String),
    #[error("cannot serialize: {0}")]
    Json(#[from] serde_json::Error),
}

/// What to measure and where the repository lives.
#[derive(Debug, Clone)]
pub struct AccuracyConfig {
    pub project_root: PathBuf,
    /// A binary to measure as is. Without it, the run retrains, syncs digests, and rebuilds.
    pub binary: Option<PathBuf>,
    pub commit: String,
    pub label: Option<String>,
    /// The validation report to read. Defaults to the one the retrain writes.
    pub validation_report: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageAccuracy {
    pub matrix: ConfusionMatrix,
    pub metrics: Metrics,
}

impl From<ConfusionMatrix> for LanguageAccuracy {
    fn from(matrix: ConfusionMatrix) -> Self {
        Self {
            matrix,
            metrics: matrix.metrics(),
        }
    }
}

/// Per-language results over one split.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub split: String,
    /// Where the numbers come from: the report path, or the judged binary.
    pub source: String,
    pub languages: BTreeMap<String, LanguageAccuracy>,
    pub pooled: LanguageAccuracy,
}

/// One measurement: the validation report the pipeline writes, and the binary over the test split.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccuracyRun {
    pub schema_version: u32,
    pub label: String,
    pub commit: String,
    pub dirty: bool,
    pub retrained: bool,
    pub generated_unix_seconds: u64,
    pub binary_sha256: String,
    /// Calibration evidence paired with this run's held-out test measurements.
    pub validation: Section,
    /// The shipped binary judged over held-out rows.
    pub test: Section,
}

#[derive(Deserialize)]
struct ReportLanguage {
    matrix: ConfusionMatrix,
}

#[derive(Deserialize)]
struct ValidationReport {
    languages: BTreeMap<String, ReportLanguage>,
}

struct CorpusRow {
    label: EvalLabel,
    text: String,
}

#[derive(Deserialize)]
struct Verdict {
    safe: bool,
}

#[derive(Deserialize)]
struct ManifestEntry {
    artifact_relative_path: String,
    artifact_sha256: String,
    lexicon_sha256: Option<String>,
}

#[derive(Deserialize)]
struct Manifest {
    entries: Vec<ManifestEntry>,
}

/// Retrains when no binary is given, then judges every test row per language.
///
/// # Errors
///
/// Returns an error when a pipeline step, the corpus, or the binary output is unusable.
pub fn run_accuracy(config: &AccuracyConfig) -> Result<AccuracyRun, AccuracyError> {
    let root = &config.project_root;
    let retrained = config.binary.is_none();
    let binary = match &config.binary {
        Some(binary) => binary.clone(),
        None => retrain_and_build(root)?,
    };
    let report = config
        .validation_report
        .clone()
        .unwrap_or_else(|| root.join(VALIDATION_REPORT));
    let validation = read_validation_section(&report)?;
    let test = judge_test_section(root, &binary)?;
    let commit = git(root, &["rev-parse", &config.commit])?;
    let head = git(root, &["rev-parse", "HEAD"])?;
    let dirty = commit == head
        && !git(root, &["status", "--porcelain", "--untracked-files=no"])?.is_empty();
    Ok(AccuracyRun {
        schema_version: 1,
        label: config
            .label
            .clone()
            .unwrap_or_else(|| commit[..7].to_owned()),
        commit,
        dirty,
        retrained,
        generated_unix_seconds: unix_seconds(),
        binary_sha256: sha256_hex(&read(&binary)?),
        validation,
        test,
    })
}

fn read_validation_section(path: &Path) -> Result<Section, AccuracyError> {
    let report: ValidationReport =
        serde_json::from_slice(&read(path)?).map_err(|error| AccuracyError::ValidationReport {
            path: path.to_owned(),
            reason: error.to_string(),
        })?;
    let languages = report
        .languages
        .into_iter()
        .map(|(code, entry)| (code, LanguageAccuracy::from(entry.matrix)))
        .collect();
    Ok(section(
        VALIDATION_SPLIT,
        &path.display().to_string(),
        languages,
    ))
}

fn judge_test_section(root: &Path, binary: &Path) -> Result<Section, AccuracyError> {
    let languages = Language::ALL
        .iter()
        .map(|&language| measure_language(root, binary, language))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(section(
        TEST_SPLIT,
        &binary.display().to_string(),
        languages,
    ))
}

fn section(split: &str, source: &str, languages: BTreeMap<String, LanguageAccuracy>) -> Section {
    let pooled = languages
        .values()
        .fold(ConfusionMatrix::default(), |sum, entry| {
            add(sum, entry.matrix)
        });
    Section {
        split: split.to_owned(),
        source: source.to_owned(),
        languages,
        pooled: pooled.into(),
    }
}

/// Prints every language as `XX had R: a% and P: b%, now it has R: c% and P: d%`, validation first.
pub fn print_comparison(baseline: &AccuracyRun, run: &AccuracyRun) {
    println!("baseline {} -> run {}", baseline.label, run.label);
    print_section(&baseline.validation, &run.validation);
    print_section(&baseline.test, &run.test);
}

fn print_section(baseline: &Section, run: &Section) {
    println!("\n{} split", run.split);
    for (code, current) in &run.languages {
        let previous = baseline.languages.get(code).map(|entry| entry.metrics);
        println!(
            "{code} had R: {} and P: {}, now it has R: {} and P: {}",
            percent(previous.and_then(|metrics| metrics.recall)),
            percent(previous.and_then(|metrics| metrics.precision)),
            percent(current.metrics.recall),
            percent(current.metrics.precision),
        );
    }
}

const REGENERATE: [&str; 7] = [
    "run",
    "--release",
    "--locked",
    "-p",
    "blasphem-train",
    "--",
    "regenerate",
];

/// Retrains, then rebuilds the binary against the published artifacts.
///
/// The evidence step inside `regenerate` judges with the embedded models. When a retrain changes an
/// artifact, that step fails until the digests are synced and the tool is rebuilt, so the retrain
/// runs again after the sync. The second run reproduces the same artifacts and writes the evidence.
fn retrain_and_build(root: &Path) -> Result<PathBuf, AccuracyError> {
    let first = run_cargo(root, &REGENERATE);
    let synced = sync_embedded_digests(root)?;
    println!("status=synced-digests changed={synced}");
    match (first, synced) {
        (Ok(()), _) => {}
        (Err(_), changed) if changed > 0 => run_cargo(root, &REGENERATE)?,
        (Err(error), _) => return Err(error),
    }
    run_cargo(
        root,
        &["build", "--release", "--locked", "--bin", "blasphem"],
    )?;
    Ok(root.join(NATIVE_BINARY))
}

/// Copies the manifest digests into the embedded table, so the rebuilt binary accepts its own artifacts.
///
/// # Errors
///
/// Returns an error when the manifest or the embedded source cannot be read, matched, or written.
pub fn sync_embedded_digests(root: &Path) -> Result<usize, AccuracyError> {
    let manifest: Manifest = serde_json::from_slice(&read(&root.join(MODEL_MANIFEST))?)?;
    let source_path = root.join(EMBEDDED_SOURCE);
    let source = String::from_utf8_lossy(&read(&source_path)?).into_owned();
    let mut lines: Vec<String> = source.split('\n').map(str::to_owned).collect();
    let mut changed = 0;
    for entry in &manifest.entries {
        changed += sync_entry(&mut lines, entry)?;
    }
    if changed > 0 {
        write(&source_path, lines.join("\n").as_bytes())?;
    }
    Ok(changed)
}

fn sync_entry(lines: &mut [String], entry: &ManifestEntry) -> Result<usize, AccuracyError> {
    let needle = format!("multilingual-v2/{}\")", entry.artifact_relative_path);
    let start = lines
        .iter()
        .position(|line| line.contains(&needle))
        .ok_or_else(|| {
            AccuracyError::EmbeddedSource(format!(
                "no include for {}",
                entry.artifact_relative_path
            ))
        })?;
    let wanted = [
        Some(entry.artifact_sha256.as_str()),
        entry.lexicon_sha256.as_deref(),
    ];
    let mut changed = 0;
    let mut cursor = start + 1;
    for digest in wanted.into_iter().flatten() {
        let (index, replaced) =
            next_digest_line(lines, cursor, digest, &entry.artifact_relative_path)?;
        changed += usize::from(replaced);
        cursor = index + 1;
    }
    Ok(changed)
}

fn next_digest_line(
    lines: &mut [String],
    from: usize,
    digest: &str,
    artifact: &str,
) -> Result<(usize, bool), AccuracyError> {
    let relative = lines[from..]
        .iter()
        .position(|line| hex_span(line).is_some())
        .ok_or_else(|| AccuracyError::EmbeddedSource(format!("no digest after {artifact}")))?;
    let index = from + relative;
    let span = hex_span(&lines[index]).expect("position found a span");
    let current = &lines[index][span.clone()];
    if current == digest {
        return Ok((index, false));
    }
    lines[index].replace_range(span, digest);
    Ok((index, true))
}

fn hex_span(line: &str) -> Option<std::ops::Range<usize>> {
    let start = line.find('"')? + 1;
    let end = start + line[start..].find('"')?;
    let candidate = &line[start..end];
    let is_digest = candidate.len() == DIGEST_HEX_LENGTH
        && candidate.bytes().all(|byte| byte.is_ascii_hexdigit());
    is_digest.then_some(start..end)
}

fn measure_language(
    root: &Path,
    binary: &Path,
    language: Language,
) -> Result<(String, LanguageAccuracy), AccuracyError> {
    let rows = test_rows(root, language)?;
    let nudged = AccuracyJudge {
        root,
        binary,
        language,
    }
    .predict(&rows)?;
    let matrix = rows
        .iter()
        .zip(&nudged)
        .fold(ConfusionMatrix::default(), |matrix, (row, &hit)| {
            count(matrix, row.label, hit)
        });
    Ok((language.code().to_owned(), matrix.into()))
}

fn count(mut matrix: ConfusionMatrix, label: EvalLabel, hit: bool) -> ConfusionMatrix {
    match (label, hit) {
        (EvalLabel::Toxic, true) => matrix.true_positive += 1,
        (EvalLabel::Toxic, false) => matrix.false_negative += 1,
        (EvalLabel::Clean, true) => matrix.false_positive += 1,
        (EvalLabel::Clean, false) => matrix.true_negative += 1,
    }
    matrix
}

fn add(left: ConfusionMatrix, right: ConfusionMatrix) -> ConfusionMatrix {
    ConfusionMatrix {
        true_positive: left.true_positive + right.true_positive,
        true_negative: left.true_negative + right.true_negative,
        false_positive: left.false_positive + right.false_positive,
        false_negative: left.false_negative + right.false_negative,
    }
}

fn test_rows(root: &Path, language: Language) -> Result<Vec<CorpusRow>, AccuracyError> {
    let path = root
        .join(CORPUS_ROOT)
        .join(format!("{}.tsv", language.storage_code()));
    let text = String::from_utf8(read(&path)?)
        .map_err(|_| corpus_error(&path, 1, "corpus is not valid UTF-8"))?;
    let mut lines = text.split('\n').enumerate();
    let header = lines.next().map(|(_, line)| line);
    if header != Some(CORPUS_HEADER) {
        return Err(corpus_error(&path, 1, "unexpected header"));
    }
    lines
        .filter(|(_, line)| !line.is_empty())
        .map(|(index, line)| parse_row(&path, index + 1, line))
        .filter_map(Result::transpose)
        .collect()
}

fn parse_row(
    path: &Path,
    line_number: usize,
    line: &str,
) -> Result<Option<CorpusRow>, AccuracyError> {
    let fields: Vec<&str> = line.split('\t').collect();
    let [split, label, text] = fields.as_slice() else {
        return Err(corpus_error(
            path,
            line_number,
            "expected three tab-separated fields",
        ));
    };
    if *split != TEST_SPLIT {
        return Ok(None);
    }
    let label = label
        .parse::<EvalLabel>()
        .map_err(|_| corpus_error(path, line_number, "label must be clean or toxic"))?;
    if text.contains(['\t', '\n', '\r']) {
        return Err(corpus_error(
            path,
            line_number,
            "text contains an unescaped control character",
        ));
    }
    Ok(Some(CorpusRow {
        label,
        text: unescape_text(text),
    }))
}

// Keep these escapes identical to blasphem-train's corpus::unescape_text.
fn unescape_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next() {
            Some('t') => output.push('\t'),
            Some('n') => output.push('\n'),
            Some('r') => output.push('\r'),
            Some('\\') => output.push('\\'),
            other => {
                output.push('\\');
                output.extend(other);
            }
        }
    }
    output
}

fn corpus_error(path: &Path, line: usize, reason: &str) -> AccuracyError {
    AccuracyError::Corpus {
        path: path.to_owned(),
        line,
        reason: reason.to_owned(),
    }
}

struct AccuracyJudge<'a> {
    root: &'a Path,
    binary: &'a Path,
    language: Language,
}

impl AccuracyJudge<'_> {
    fn predict(&self, rows: &[CorpusRow]) -> Result<Vec<bool>, AccuracyError> {
        let single_line = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| !row.text.contains(['\r', '\n']))
            .collect::<Vec<_>>();
        let mut nudged = vec![false; rows.len()];
        let batch = self.batch(&single_line)?;
        for ((index, _), predicted) in single_line.into_iter().zip(batch) {
            nudged[index] = predicted;
        }
        for (index, row) in rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.text.contains(['\r', '\n']))
        {
            nudged[index] = self.single(&row.text)?;
        }
        Ok(nudged)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(self.binary);
        command
            .args([
                "judge",
                "--locales",
                self.language.code(),
                "--no-detect",
                "--json",
            ])
            .current_dir(self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }

    fn description(&self) -> String {
        format!(
            "{} judge --locales {} --no-detect --json",
            self.binary.display(),
            self.language.code()
        )
    }

    fn spawn_error(&self, source: std::io::Error) -> AccuracyError {
        AccuracyError::Spawn {
            command: self.description(),
            source,
        }
    }

    fn batch(&self, rows: &[(usize, &CorpusRow)]) -> Result<Vec<bool>, AccuracyError> {
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let mut child = self
            .command()
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|source| self.spawn_error(source))?;
        let mut stdin = child.stdin.take().expect("stdin was piped");
        let input: String = rows
            .iter()
            .map(|(_, row)| format!("{}\n", row.text))
            .collect();
        let (written, output) = std::thread::scope(|scope| {
            let writer = scope.spawn(move || stdin.write_all(input.as_bytes()));
            let output = child.wait_with_output();
            (writer.join().expect("stdin writer panicked"), output)
        });
        let output = output.map_err(|source| self.spawn_error(source))?;
        let verdicts = self.decode_output(output, rows.len())?;
        written.map_err(|source| self.spawn_error(source))?;
        Ok(verdicts)
    }

    fn single(&self, text: &str) -> Result<bool, AccuracyError> {
        let output = self
            .command()
            .arg("--")
            .arg(text)
            .stdin(Stdio::null())
            .output()
            .map_err(|source| self.spawn_error(source))?;
        let verdicts = self.decode_output(output, 1)?;
        Ok(verdicts[0])
    }

    fn decode_output(&self, output: Output, expected: usize) -> Result<Vec<bool>, AccuracyError> {
        if !matches!(output.status.code(), Some(0 | 1)) {
            return Err(AccuracyError::CommandFailed {
                command: self.description(),
                status: output.status.to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
        parse_verdicts(
            self.language,
            expected,
            &String::from_utf8_lossy(&output.stdout),
        )
    }
}

fn parse_verdicts(
    language: Language,
    expected: usize,
    stdout: &str,
) -> Result<Vec<bool>, AccuracyError> {
    let nudged = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str::<Verdict>(line).map(|verdict| !verdict.safe))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| AccuracyError::Verdicts {
            language: language.code(),
            reason: error.to_string(),
        })?;
    if nudged.len() != expected {
        return Err(AccuracyError::Verdicts {
            language: language.code(),
            reason: format!("{expected} rows in, {} verdicts out", nudged.len()),
        });
    }
    Ok(nudged)
}

fn run_cargo(root: &Path, arguments: &[&str]) -> Result<(), AccuracyError> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let command = format!("{cargo} {}", arguments.join(" "));
    println!("status=running command=`{command}`");
    let status = Command::new(&cargo)
        .args(arguments)
        .current_dir(root)
        .status()
        .map_err(|source| AccuracyError::Spawn {
            command: command.clone(),
            source,
        })?;
    if !status.success() {
        return Err(AccuracyError::CommandFailed {
            command,
            status: status.to_string(),
            stderr: String::new(),
        });
    }
    Ok(())
}

fn git(root: &Path, arguments: &[&str]) -> Result<String, AccuracyError> {
    let command = format!("git {}", arguments.join(" "));
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .map_err(|source| AccuracyError::Spawn {
            command: command.clone(),
            source,
        })?;
    if !output.status.success() {
        return Err(AccuracyError::CommandFailed {
            command,
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn percent(value: Option<f64>) -> String {
    value.map_or_else(
        || "n/a".to_owned(),
        |ratio| format!("{:.1}%", ratio * 100.0),
    )
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

fn read(path: &Path) -> Result<Vec<u8>, AccuracyError> {
    fs::read(path).map_err(|source| AccuracyError::Io {
        path: path.to_owned(),
        source,
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), AccuracyError> {
    fs::write(path, bytes).map_err(|source| AccuracyError::Write {
        path: path.to_owned(),
        source,
    })
}
