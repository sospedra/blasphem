use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use blasphem::{Language, LanguageDetector, LanguageIdentifier, LanguageResolution};

use crate::{
    AutoTimingEvidence, BenchmarkError, FileSizeRecord, SizeError, record_file, run_auto_timing,
    sha256_hex,
};

const PINNED_ROWS: u64 = 418_882;
const PINNED_SUPPORTED_ROWS: u64 = 147_432;
const PINNED_UNSUPPORTED_ROWS: u64 = 271_450;
const PINNED_TEXT_SHA256: &str = "8c67c444dec9216991532dee6fdcf4b84843c349fbee218cf70fc6df3d8c5786";
const PINNED_LABEL_SHA256: &str =
    "f88ed093f49c0715b75cd6a2d66ad55db936183e35278515925de31c034d8549";
const LANGUAGE_MODEL_ARTIFACT_BYTES: u64 = 18_498_380;
const LANGUAGE_MODEL_ARTIFACT_SHA256: &str =
    "69dd5c22723bbe60073575a67fb94fc1fb8ba60c3ed1ac150ddbef1935dd84da";
const C_PARITY_ROWS: u64 = 100;
const C_PARITY_TOLERANCE: f32 = 0.000_001;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateEvidence {
    pub numerator: u64,
    pub denominator: u64,
    pub value: f64,
}

impl RateEvidence {
    fn new(numerator: u64, denominator: u64) -> Self {
        let value = if denominator == 0 {
            0.0
        } else {
            numerator as f64 / denominator as f64
        };
        Self {
            numerator,
            denominator,
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportedRouteEvidence {
    pub rows: u64,
    pub correct: u64,
    pub unknown: u64,
    pub misrouted: u64,
    pub route_accuracy: RateEvidence,
    pub unknown_rate: RateEvidence,
    pub misroute_rate: RateEvidence,
    pub known_route_precision: RateEvidence,
}

impl SupportedRouteEvidence {
    fn from_counts(rows: u64, correct: u64, unknown: u64, misrouted: u64) -> Self {
        Self {
            rows,
            correct,
            unknown,
            misrouted,
            route_accuracy: RateEvidence::new(correct, rows),
            unknown_rate: RateEvidence::new(unknown, rows),
            misroute_rate: RateEvidence::new(misrouted, rows),
            known_route_precision: RateEvidence::new(correct, correct + misrouted),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedRouteEvidence {
    pub rows: u64,
    pub rejected_as_unknown: u64,
    pub falsely_routed: u64,
    pub unsupported_rejection_rate: RateEvidence,
}

impl UnsupportedRouteEvidence {
    fn from_counts(rows: u64, rejected_as_unknown: u64, falsely_routed: u64) -> Self {
        Self {
            rows,
            rejected_as_unknown,
            falsely_routed,
            unsupported_rejection_rate: RateEvidence::new(rejected_as_unknown, rows),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoCorpusEvidence {
    pub rows: u64,
    pub supported_rows: u64,
    pub unsupported_rows: u64,
    pub text_sha256: String,
    pub label_sha256: String,
    pub text_has_final_newline: bool,
    pub labels_have_final_newline: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoCorpusEvaluation {
    pub corpus: AutoCorpusEvidence,
    pub supported: SupportedRouteEvidence,
    pub unsupported: UnsupportedRouteEvidence,
    pub languages: BTreeMap<String, SupportedRouteEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CParityEvidence {
    pub fixture: FileSizeRecord,
    pub rows: u64,
    pub matched_rows: u64,
    pub score_tolerance: String,
    pub verification_command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyEvidence {
    pub command: String,
    pub output_sha256: String,
    pub language_model_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoSizeEvidence {
    pub native_binary: FileSizeRecord,
    pub language_model_artifact: FileSizeRecord,
    pub browser_builds: BrowserBuildEvidence,
    pub explicit_only_dependency: DependencyEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoValidationEvidence {
    pub schema_version: u16,
    pub evidence_status: String,
    pub computer: String,
    pub rust_version: String,
    pub target_triple: String,
    pub model_manifest_sha256: String,
    pub cold_initialization_nanoseconds: u64,
    pub c_parity: CParityEvidence,
    pub corpus: AutoCorpusEvidence,
    pub supported: SupportedRouteEvidence,
    pub unsupported: UnsupportedRouteEvidence,
    pub languages: BTreeMap<String, SupportedRouteEvidence>,
    pub timing: AutoTimingEvidence,
    pub size: AutoSizeEvidence,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AutoValidationConfig {
    pub texts: PathBuf,
    pub labels: PathBuf,
    pub fixtures: PathBuf,
    pub hurtlex_root: PathBuf,
    pub model_manifest: PathBuf,
    pub native_binary: PathBuf,
    pub language_model_artifact: PathBuf,
    pub browser_report: PathBuf,
    pub c_parity_fixture: PathBuf,
    pub project_root: PathBuf,
    pub computer: String,
    pub rust_version: String,
    pub target_triple: String,
}

#[derive(Debug, Default, Clone, Copy)]
struct SupportedCounts {
    rows: u64,
    correct: u64,
    unknown: u64,
    misrouted: u64,
}

impl SupportedCounts {
    fn record(&mut self, expected: Language, actual: Option<Language>) {
        self.rows += 1;
        match actual {
            Some(language) if language == expected => self.correct += 1,
            Some(_) => self.misrouted += 1,
            None => self.unknown += 1,
        }
    }

    fn evidence(self) -> SupportedRouteEvidence {
        SupportedRouteEvidence::from_counts(self.rows, self.correct, self.unknown, self.misrouted)
    }
}

#[derive(Debug, Error)]
pub enum AutoEvidenceError {
    #[error("cannot read AUTO evidence input at {path}: {source}")]
    FileIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("AUTO evidence input is not UTF-8: {0}")]
    InvalidUtf8(PathBuf),
    #[error("text and label corpora have unequal row counts at row {0}")]
    UnequalRows(u64),
    #[error("empty language label at row {0}")]
    EmptyLabel(u64),
    #[error("cannot parse browser build evidence: {0}")]
    BrowserJson(#[from] serde_json::Error),
    #[error("browser build totals do not match their files for {0}")]
    BrowserTotals(String),
    #[error("the Tatoeba corpus does not match the pinned identity: {0}")]
    CorpusIdentity(String),
    #[error("the Rust Tatoeba route counts differ from the pinned C counts: {0}")]
    RouteCounts(String),
    #[error("cannot initialize the automatic language detector: {0}")]
    Detector(String),
    #[error("AUTO detector initialization time overflow")]
    InitializationTimeOverflow,
    #[error(transparent)]
    Benchmark(#[from] BenchmarkError),
    #[error(transparent)]
    Size(#[from] SizeError),
    #[error("the C parity fixture has {0} rows, expected 100")]
    ParityRows(u64),
    #[error("cannot parse C parity fixture line {line}: {source}")]
    ParityJson {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("C parity mismatch: {0}")]
    ParityMismatch(Box<str>),
    #[error("cannot initialize the language model C parity detector: {0}")]
    ParityDetector(blasphem_language::ModelError),
    #[error("cannot run the explicit-only dependency command: {0}")]
    DependencyCommand(std::io::Error),
    #[error("the explicit-only dependency command failed: {0}")]
    DependencyCommandFailed(String),
    #[error("the explicit-only WASM dependency tree contains the language model")]
    ExplicitTreeContainsLanguageModel,
}

/// Checks the exact identity and row counts of the pinned Tatoeba corpus.
///
/// # Errors
///
/// Returns an error when any identity, count, or termination fact differs.
pub fn validate_pinned_corpus(corpus: &AutoCorpusEvidence) -> Result<(), AutoEvidenceError> {
    if corpus.rows != PINNED_ROWS {
        return Err(AutoEvidenceError::CorpusIdentity(format!(
            "rows={}, expected {PINNED_ROWS}",
            corpus.rows
        )));
    }
    if corpus.supported_rows != PINNED_SUPPORTED_ROWS {
        return Err(AutoEvidenceError::CorpusIdentity(format!(
            "supported_rows={}, expected {PINNED_SUPPORTED_ROWS}",
            corpus.supported_rows
        )));
    }
    if corpus.unsupported_rows != PINNED_UNSUPPORTED_ROWS {
        return Err(AutoEvidenceError::CorpusIdentity(format!(
            "unsupported_rows={}, expected {PINNED_UNSUPPORTED_ROWS}",
            corpus.unsupported_rows
        )));
    }
    if corpus.text_sha256 != PINNED_TEXT_SHA256 {
        return Err(AutoEvidenceError::CorpusIdentity(
            "the text SHA-256 differs".to_owned(),
        ));
    }
    if corpus.label_sha256 != PINNED_LABEL_SHA256 {
        return Err(AutoEvidenceError::CorpusIdentity(
            "the label SHA-256 differs".to_owned(),
        ));
    }
    if corpus.text_has_final_newline {
        return Err(AutoEvidenceError::CorpusIdentity(
            "the text file has a final newline".to_owned(),
        ));
    }
    Ok(())
}

/// Reads the paired corpus and records its identity without running the language detector.
///
/// # Errors
///
/// Returns an error for unreadable, invalid, or unequal parallel files.
pub fn inspect_auto_corpus(
    text_path: &Path,
    label_path: &Path,
) -> Result<AutoCorpusEvidence, AutoEvidenceError> {
    let text_bytes = read_file(text_path)?;
    let label_bytes = read_file(label_path)?;
    let text = std::str::from_utf8(&text_bytes)
        .map_err(|_| AutoEvidenceError::InvalidUtf8(text_path.to_owned()))?;
    let labels = std::str::from_utf8(&label_bytes)
        .map_err(|_| AutoEvidenceError::InvalidUtf8(label_path.to_owned()))?;
    let mut text_lines = text.lines();
    let mut label_lines = labels.lines();
    let mut rows = 0_u64;
    let mut supported_rows = 0_u64;
    let mut unsupported_rows = 0_u64;
    loop {
        match (text_lines.next(), label_lines.next()) {
            (Some(_), Some(label)) => {
                rows += 1;
                if label.is_empty() {
                    return Err(AutoEvidenceError::EmptyLabel(rows));
                }
                if corpus_language(label).is_some() {
                    supported_rows += 1;
                } else {
                    unsupported_rows += 1;
                }
            }
            (None, None) => break,
            _ => return Err(AutoEvidenceError::UnequalRows(rows + 1)),
        }
    }
    Ok(AutoCorpusEvidence {
        rows,
        supported_rows,
        unsupported_rows,
        text_sha256: sha256_hex(&text_bytes),
        label_sha256: sha256_hex(&label_bytes),
        text_has_final_newline: text_bytes.ends_with(b"\n"),
        labels_have_final_newline: label_bytes.ends_with(b"\n"),
    })
}

/// Runs the complete pinned automatic-routing evidence process.
///
/// # Errors
///
/// Returns an error when an input, route, timing sample, size, or dependency check fails.
pub fn run_auto_validation(
    config: &AutoValidationConfig,
) -> Result<AutoValidationEvidence, AutoEvidenceError> {
    let inspected_corpus = inspect_auto_corpus(&config.texts, &config.labels)?;
    validate_pinned_corpus(&inspected_corpus)?;

    let initialization_start = Instant::now();
    let identifier =
        LanguageDetector::new().map_err(|error| AutoEvidenceError::Detector(error.to_string()))?;
    let cold_initialization_nanoseconds = u64::try_from(initialization_start.elapsed().as_nanos())
        .map_err(|_| AutoEvidenceError::InitializationTimeOverflow)?;

    let evaluation = evaluate_auto_corpus(&config.texts, &config.labels, &identifier)?;
    validate_pinned_routes(&evaluation)?;
    let timing = run_auto_timing(
        &identifier,
        config.fixtures.as_path(),
        config.hurtlex_root.as_path(),
    )?;
    let model_manifest = read_file(&config.model_manifest)?;
    let browser_builds = load_browser_build_evidence(&config.browser_report)?;
    let native_binary = record_file(
        &config.native_binary,
        &relative_label(&config.project_root, &config.native_binary),
        None,
        None,
    )?;
    let language_model_artifact = record_file(
        &config.language_model_artifact,
        &relative_label(&config.project_root, &config.language_model_artifact),
        Some(LANGUAGE_MODEL_ARTIFACT_SHA256),
        Some(LANGUAGE_MODEL_ARTIFACT_BYTES),
    )?;
    let c_parity = c_parity_evidence(&config.project_root, &config.c_parity_fixture)?;
    let explicit_only_dependency = explicit_dependency_evidence(&config.project_root)?;

    Ok(AutoValidationEvidence {
        schema_version: 1,
        evidence_status: "experimental".to_owned(),
        computer: config.computer.clone(),
        rust_version: config.rust_version.clone(),
        target_triple: config.target_triple.clone(),
        model_manifest_sha256: sha256_hex(&model_manifest),
        cold_initialization_nanoseconds,
        c_parity,
        corpus: evaluation.corpus,
        supported: evaluation.supported,
        unsupported: evaluation.unsupported,
        languages: evaluation.languages,
        timing,
        size: AutoSizeEvidence {
            native_binary,
            language_model_artifact,
            browser_builds,
            explicit_only_dependency,
        },
        limitations: vec![
            "Tatoeba route accuracy is not social-message toxicity accuracy.".to_owned(),
            "The corpus does not cover code-switching or romanized chat well.".to_owned(),
            "Unsupported-language rejection is best-effort with this 15-profile model.".to_owned(),
            "The current WASM build embeds all 15 toxicity packs.".to_owned(),
        ],
    })
}

fn validate_pinned_routes(evaluation: &AutoCorpusEvaluation) -> Result<(), AutoEvidenceError> {
    let supported = &evaluation.supported;
    let unsupported = &evaluation.unsupported;
    if supported.correct != 144_150 {
        return Err(AutoEvidenceError::RouteCounts(format!(
            "correct={}, expected 144150",
            supported.correct
        )));
    }
    if supported.unknown != 3_150 {
        return Err(AutoEvidenceError::RouteCounts(format!(
            "unknown={}, expected 3150",
            supported.unknown
        )));
    }
    if supported.misrouted != 132 {
        return Err(AutoEvidenceError::RouteCounts(format!(
            "misrouted={}, expected 132",
            supported.misrouted
        )));
    }
    if unsupported.rejected_as_unknown != 249_593 {
        return Err(AutoEvidenceError::RouteCounts(format!(
            "unsupported_unknown={}, expected 249593",
            unsupported.rejected_as_unknown
        )));
    }
    if unsupported.falsely_routed != 21_857 {
        return Err(AutoEvidenceError::RouteCounts(format!(
            "unsupported_routed={}, expected 21857",
            unsupported.falsely_routed
        )));
    }
    Ok(())
}

fn c_parity_evidence(
    project_root: &Path,
    path: &Path,
) -> Result<CParityEvidence, AutoEvidenceError> {
    let rows = verify_c_parity_fixture(path)?;
    if rows != C_PARITY_ROWS {
        return Err(AutoEvidenceError::ParityRows(rows));
    }
    let fixture = record_file(path, &relative_label(project_root, path), None, None)?;
    Ok(CParityEvidence {
        fixture,
        rows,
        matched_rows: rows,
        score_tolerance: format!("{C_PARITY_TOLERANCE:.6}"),
        verification_command: "cargo test -p blasphem-language --test parity --locked".to_owned(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CParityFixtureRow {
    id: String,
    category: String,
    input: String,
    language: Option<String>,
    reliable: bool,
    feature_count: usize,
    top_score: f32,
    second_score: f32,
    ranked_scores: Vec<CParityFixtureScore>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CParityFixtureScore {
    language: String,
    score: f32,
}

/// Verifies every frozen C detector field against the Rust language detector.
///
/// # Errors
///
/// Returns an error for an invalid fixture, detector error, or field mismatch.
pub fn verify_c_parity_fixture(path: &Path) -> Result<u64, AutoEvidenceError> {
    let bytes = read_file(path)?;
    let fixture =
        std::str::from_utf8(&bytes).map_err(|_| AutoEvidenceError::InvalidUtf8(path.to_owned()))?;
    let detector = blasphem_language::Detector::new().map_err(AutoEvidenceError::ParityDetector)?;
    let mut rows = 0_u64;

    for (line_index, line) in fixture.lines().enumerate() {
        let row_number = line_index + 1;
        let row: CParityFixtureRow =
            serde_json::from_str(line).map_err(|source| AutoEvidenceError::ParityJson {
                line: row_number,
                source,
            })?;
        let actual = detector.detect(&row.input);
        verify_c_parity_row(row_number, &row, &actual)?;
        rows = rows
            .checked_add(1)
            .ok_or(AutoEvidenceError::ParityRows(u64::MAX))?;
    }

    Ok(rows)
}

fn verify_c_parity_row(
    row_number: usize,
    expected: &CParityFixtureRow,
    actual: &blasphem_language::Detection,
) -> Result<(), AutoEvidenceError> {
    compare_parity_field(
        row_number,
        expected,
        "language",
        expected.language.as_deref(),
        actual.language.map(blasphem_language::Language::code),
    )?;
    compare_parity_field(
        row_number,
        expected,
        "reliable",
        expected.reliable,
        actual.reliable,
    )?;
    compare_parity_field(
        row_number,
        expected,
        "feature_count",
        expected.feature_count,
        actual.feature_count,
    )?;
    compare_parity_score(
        row_number,
        expected,
        "top_score",
        expected.top_score,
        actual.top_score,
    )?;
    compare_parity_score(
        row_number,
        expected,
        "second_score",
        expected.second_score,
        actual.second_score,
    )?;
    compare_parity_field(
        row_number,
        expected,
        "ranked_scores.len",
        expected.ranked_scores.len(),
        actual.ranked_scores.len(),
    )?;

    for (index, (expected_score, actual_score)) in expected
        .ranked_scores
        .iter()
        .zip(&actual.ranked_scores)
        .enumerate()
    {
        compare_parity_field(
            row_number,
            expected,
            &format!("ranked_scores[{index}].language"),
            expected_score.language.as_str(),
            actual_score.language.code(),
        )?;
        compare_parity_score(
            row_number,
            expected,
            &format!("ranked_scores[{index}].score"),
            expected_score.score,
            actual_score.score,
        )?;
    }

    Ok(())
}

fn compare_parity_field<T: PartialEq + std::fmt::Debug>(
    row_number: usize,
    row: &CParityFixtureRow,
    field: &str,
    expected: T,
    actual: T,
) -> Result<(), AutoEvidenceError> {
    if expected == actual {
        return Ok(());
    }
    Err(parity_mismatch(
        row_number,
        row,
        field,
        format!("{expected:?}"),
        format!("{actual:?}"),
    ))
}

fn compare_parity_score(
    row_number: usize,
    row: &CParityFixtureRow,
    field: &str,
    expected: f32,
    actual: f32,
) -> Result<(), AutoEvidenceError> {
    if (expected - actual).abs() <= C_PARITY_TOLERANCE {
        return Ok(());
    }
    Err(parity_mismatch(
        row_number,
        row,
        field,
        format!("{expected:.9}"),
        format!("{actual:.9}"),
    ))
}

fn parity_mismatch(
    row_number: usize,
    row: &CParityFixtureRow,
    field: &str,
    expected: String,
    actual: String,
) -> AutoEvidenceError {
    AutoEvidenceError::ParityMismatch(
        format!(
            "row {row_number} {} ({}) differs at {field}: expected {expected}, got {actual}",
            row.id, row.category
        )
        .into_boxed_str(),
    )
}

fn explicit_dependency_evidence(
    project_root: &Path,
) -> Result<DependencyEvidence, AutoEvidenceError> {
    let command = "cargo tree -p blasphem-wasm --no-default-features -e normal";
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "blasphem-wasm",
            "--no-default-features",
            "-e",
            "normal",
        ])
        .current_dir(project_root)
        .output()
        .map_err(AutoEvidenceError::DependencyCommand)?;
    if !output.status.success() {
        return Err(AutoEvidenceError::DependencyCommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let language_model_present =
        String::from_utf8_lossy(&output.stdout).contains("blasphem-language v");
    if language_model_present {
        return Err(AutoEvidenceError::ExplicitTreeContainsLanguageModel);
    }
    Ok(DependencyEvidence {
        command: command.to_owned(),
        output_sha256: sha256_hex(&output.stdout),
        language_model_present,
    })
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// Evaluates product AUTO routing against parallel text and language files.
///
/// # Errors
///
/// Returns an error for unreadable, invalid, or unequal parallel files.
pub fn evaluate_auto_corpus<I: LanguageIdentifier + ?Sized>(
    text_path: &Path,
    label_path: &Path,
    identifier: &I,
) -> Result<AutoCorpusEvaluation, AutoEvidenceError> {
    let text_bytes = read_file(text_path)?;
    let label_bytes = read_file(label_path)?;
    let text = std::str::from_utf8(&text_bytes)
        .map_err(|_| AutoEvidenceError::InvalidUtf8(text_path.to_owned()))?;
    let labels = std::str::from_utf8(&label_bytes)
        .map_err(|_| AutoEvidenceError::InvalidUtf8(label_path.to_owned()))?;

    let mut text_lines = text.lines();
    let mut label_lines = labels.lines();
    let mut rows = 0_u64;
    let mut supported = SupportedCounts::default();
    let mut per_language = [SupportedCounts::default(); 15];
    let mut unsupported_rows = 0_u64;
    let mut unsupported_unknown = 0_u64;
    let mut unsupported_routed = 0_u64;

    loop {
        let text_line = text_lines.next();
        let label_line = label_lines.next();
        match (text_line, label_line) {
            (Some(text_line), Some(label_line)) => {
                rows += 1;
                if label_line.is_empty() {
                    return Err(AutoEvidenceError::EmptyLabel(rows));
                }
                let detection = identifier.identify(text_line);
                let actual = if detection.reliable {
                    match detection.resolution {
                        LanguageResolution::Known(language) => Some(language),
                        LanguageResolution::Unknown => None,
                    }
                } else {
                    None
                };
                if let Some(expected) = corpus_language(label_line) {
                    supported.record(expected, actual);
                    per_language[expected.index()].record(expected, actual);
                } else {
                    unsupported_rows += 1;
                    if actual.is_some() {
                        unsupported_routed += 1;
                    } else {
                        unsupported_unknown += 1;
                    }
                }
            }
            (None, None) => break,
            _ => return Err(AutoEvidenceError::UnequalRows(rows + 1)),
        }
    }

    let languages = Language::ALL
        .into_iter()
        .map(|language| {
            (
                language.code().to_owned(),
                per_language[language.index()].evidence(),
            )
        })
        .collect();

    Ok(AutoCorpusEvaluation {
        corpus: AutoCorpusEvidence {
            rows,
            supported_rows: supported.rows,
            unsupported_rows,
            text_sha256: sha256_hex(&text_bytes),
            label_sha256: sha256_hex(&label_bytes),
            text_has_final_newline: text_bytes.ends_with(b"\n"),
            labels_have_final_newline: label_bytes.ends_with(b"\n"),
        },
        supported: supported.evidence(),
        unsupported: UnsupportedRouteEvidence::from_counts(
            unsupported_rows,
            unsupported_unknown,
            unsupported_routed,
        ),
        languages,
    })
}

fn read_file(path: &Path) -> Result<Vec<u8>, AutoEvidenceError> {
    fs::read(path).map_err(|source| AutoEvidenceError::FileIo {
        path: path.to_owned(),
        source,
    })
}

fn corpus_language(code: &str) -> Option<Language> {
    match code {
        "ar" => Some(Language::Ar),
        "de" => Some(Language::De),
        "en" => Some(Language::En),
        "es" => Some(Language::Es),
        "fr" => Some(Language::Fr),
        "hi" => Some(Language::Hi),
        "it" => Some(Language::It),
        "ja" => Some(Language::Ja),
        "ko" => Some(Language::Ko),
        "ms" => Some(Language::Ms),
        "pt" => Some(Language::Pt),
        "ru" => Some(Language::Ru),
        "tr" => Some(Language::Tr),
        "vi" => Some(Language::Vi),
        "zh" => Some(Language::Zh),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompressedFileRecord {
    pub relative_path: String,
    pub sha256: String,
    pub raw_bytes: u64,
    pub gzip_bytes: u64,
    pub brotli_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebBundleRecord {
    pub wasm: CompressedFileRecord,
    pub javascript_glue: CompressedFileRecord,
    pub raw_total_bytes: u64,
    pub gzip_total_bytes: u64,
    pub brotli_total_bytes: u64,
}

impl WebBundleRecord {
    fn totals_match(&self) -> bool {
        self.raw_total_bytes == self.wasm.raw_bytes + self.javascript_glue.raw_bytes
            && self.gzip_total_bytes == self.wasm.gzip_bytes + self.javascript_glue.gzip_bytes
            && self.brotli_total_bytes == self.wasm.brotli_bytes + self.javascript_glue.brotli_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserBuildEvidence {
    pub full: WebBundleRecord,
    pub explicit_only: WebBundleRecord,
}

#[derive(Debug, Deserialize)]
struct BrowserReport {
    browser_builds: BrowserBuildEvidence,
}

/// Loads size records for the full and explicit-only browser builds.
///
/// # Errors
///
/// Returns an error when the report is unreadable, invalid, or internally inconsistent.
pub fn load_browser_build_evidence(path: &Path) -> Result<BrowserBuildEvidence, AutoEvidenceError> {
    let bytes = read_file(path)?;
    let report: BrowserReport = serde_json::from_slice(&bytes)?;
    if !report.browser_builds.full.totals_match() {
        return Err(AutoEvidenceError::BrowserTotals("full".to_owned()));
    }
    if !report.browser_builds.explicit_only.totals_match() {
        return Err(AutoEvidenceError::BrowserTotals("explicit_only".to_owned()));
    }
    Ok(report.browser_builds)
}
