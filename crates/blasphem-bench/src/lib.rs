//! Experimental performance and size evidence tools.

use std::{
    collections::BTreeSet,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use toxcheck::Language;

mod auto;
mod benchmark;
mod size;

pub use auto::{
    AutoCorpusEvaluation, AutoCorpusEvidence, AutoEvidenceError, AutoSizeEvidence,
    AutoValidationConfig, AutoValidationEvidence, BrowserBuildEvidence, CParityEvidence,
    CompressedFileRecord, DependencyEvidence, RateEvidence, SupportedRouteEvidence,
    UnsupportedRouteEvidence, WebBundleRecord, evaluate_auto_corpus, inspect_auto_corpus,
    load_browser_build_evidence, run_auto_validation, validate_pinned_corpus,
    verify_c_parity_fixture,
};
pub use benchmark::{
    AutoTimingEvidence, AutoTimingRejection, AutoTimingSummary, BenchmarkEvidence, TimingSummary,
    latency_gate, peak_rss_bytes, run_auto_timing, run_benchmark,
};
pub use size::{FileSizeRecord, SizeError, SizeEvidence, collect_size_evidence, record_file};
pub use toxcheck::{LanguageDetection, LanguageIdentifier, LanguageResolution, LanguageSource};

pub const BINARY_SIZE_LIMIT_BYTES: u64 = 7_340_032;
pub const ARTIFACT_SIZE_LIMIT_BYTES: u64 = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureKind {
    Clean,
    Toxic,
    Dense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureLength {
    UnicodeScalars280,
    Utf8Bytes4096,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkFixture {
    pub id: String,
    pub language: Language,
    pub kind: FixtureKind,
    pub length: FixtureLength,
    pub text: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rates {
    pub checks_per_second: f64,
    pub bytes_per_second: f64,
}

#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("cannot read benchmark fixtures at {path}: {source}")]
    FixtureIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse benchmark fixture line {line}: {source}")]
    FixtureJson {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
    #[error("duplicate benchmark fixture identifier: {0}")]
    DuplicateIdentifier(String),
    #[error("duplicate benchmark fixture dimension: {0}")]
    DuplicateDimension(String),
    #[error("missing benchmark fixture dimension: {0}")]
    MissingDimension(String),
    #[error("a benchmark rate cannot use zero samples or elapsed time")]
    InvalidRateInput,
    #[error("a calculated benchmark rate is not finite")]
    NonFiniteRate,
    #[error("benchmark fixture count must be 90, got {0}")]
    WrongFixtureCount(usize),
    #[error("benchmark fixture has the wrong length: {0}")]
    WrongFixtureLength(String),
    #[error("benchmark fixture digest mismatch: {0}")]
    FixtureDigestMismatch(String),
    #[error("cannot read benchmark resource at {path}: {source}")]
    ResourceIo {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot initialize the detector for {}: {reason}", .language.code())]
    Detector { language: Language, reason: String },
    #[error("benchmark detector is missing for {}", .0.code())]
    MissingDetector(Language),
    #[error(
        "AUTO routed fixture {fixture} away from {expected}: {actual}",
        actual = .actual.map_or("unknown", Language::code)
    )]
    AutoRoute {
        fixture: String,
        expected: Language,
        actual: Option<Language>,
    },
    #[error("benchmark timing value overflow")]
    TimingOverflow,
    #[error("benchmark produced no samples")]
    NoSamples,
    #[error("cannot read peak resident memory: {0}")]
    PeakRss(std::io::Error),
    #[error("peak resident memory was negative")]
    NegativePeakRss,
    #[error("peak resident memory is unsupported on this target")]
    UnsupportedPeakRssTarget,
}

#[derive(Debug, Error)]
pub enum SizeGateError {
    #[error("shipping binary has {actual} bytes; limit is {limit} bytes")]
    BinaryTooLarge { actual: u64, limit: u64 },
    #[error("sparse artifact has {actual} bytes; limit is below {limit} bytes")]
    ArtifactTooLarge { actual: u64, limit: u64 },
}

/// Loads strict JSON Lines benchmark fixtures.
///
/// # Errors
///
/// Returns an error when a line cannot be read or parsed.
pub fn load_benchmark_fixtures(path: &Path) -> Result<Vec<BenchmarkFixture>, BenchmarkError> {
    let file = File::open(path).map_err(|source| BenchmarkError::FixtureIo {
        path: path.to_owned(),
        source,
    })?;
    let mut fixtures = Vec::new();
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|source| BenchmarkError::FixtureIo {
            path: path.to_owned(),
            source,
        })?;
        let fixture =
            serde_json::from_str(&line).map_err(|source| BenchmarkError::FixtureJson {
                line: line_index + 1,
                source,
            })?;
        fixtures.push(fixture);
    }
    Ok(fixtures)
}

/// Checks the exact language, kind, and length matrix.
///
/// # Errors
///
/// Returns an error for duplicate identifiers, duplicate dimensions, or missing dimensions.
pub fn assert_unique_dimensions(fixtures: &[BenchmarkFixture]) -> Result<(), BenchmarkError> {
    let mut identifiers = BTreeSet::new();
    let mut dimensions = BTreeSet::new();
    for fixture in fixtures {
        if !identifiers.insert(fixture.id.as_str()) {
            return Err(BenchmarkError::DuplicateIdentifier(fixture.id.clone()));
        }
        let dimension = (fixture.language, fixture.kind, fixture.length);
        if !dimensions.insert(dimension) {
            return Err(BenchmarkError::DuplicateDimension(fixture.id.clone()));
        }
    }
    for language in Language::ALL {
        for kind in [FixtureKind::Clean, FixtureKind::Toxic, FixtureKind::Dense] {
            for length in [
                FixtureLength::UnicodeScalars280,
                FixtureLength::Utf8Bytes4096,
            ] {
                if !dimensions.contains(&(language, kind, length)) {
                    return Err(BenchmarkError::MissingDimension(format!(
                        "{}-{kind:?}-{length:?}",
                        language.code(),
                    )));
                }
            }
        }
    }
    Ok(())
}

#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut result, "{byte:02x}").expect("writing into a String cannot fail");
    }
    result
}

/// Serializes one value as canonical JSON without a final newline.
///
/// # Errors
///
/// Returns an error when the value cannot be serialized as JSON.
pub fn canonical_json_bytes(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    serde_jcs::to_vec(value)
}

/// Calculates throughput from the same measured sample interval.
///
/// # Errors
///
/// Returns an error for zero inputs or a non-finite result.
pub fn calculate_rates(
    samples: u64,
    input_bytes: u64,
    total_elapsed_nanoseconds: u128,
) -> Result<Rates, BenchmarkError> {
    if samples == 0 || total_elapsed_nanoseconds == 0 {
        return Err(BenchmarkError::InvalidRateInput);
    }
    let seconds = total_elapsed_nanoseconds as f64 / 1_000_000_000.0;
    let checks_per_second = samples as f64 / seconds;
    let bytes_per_second = samples as f64 * input_bytes as f64 / seconds;
    if !checks_per_second.is_finite() || !bytes_per_second.is_finite() {
        return Err(BenchmarkError::NonFiniteRate);
    }
    Ok(Rates {
        checks_per_second,
        bytes_per_second,
    })
}

#[must_use]
pub fn nearest_rank(sorted: &[u64], percentile: usize) -> Option<u64> {
    if sorted.is_empty() || percentile == 0 {
        return None;
    }
    let rank = sorted
        .len()
        .saturating_mul(percentile)
        .div_ceil(100)
        .clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

/// Checks the native shipping binary size.
///
/// # Errors
///
/// Returns an error when the binary exceeds the shipping limit.
pub const fn check_binary_size(bytes: u64) -> Result<(), SizeGateError> {
    if bytes <= BINARY_SIZE_LIMIT_BYTES {
        Ok(())
    } else {
        Err(SizeGateError::BinaryTooLarge {
            actual: bytes,
            limit: BINARY_SIZE_LIMIT_BYTES,
        })
    }
}

/// Checks one sparse artifact size.
///
/// # Errors
///
/// Returns an error when the artifact reaches the exclusive limit.
pub const fn check_artifact_size(bytes: u64) -> Result<(), SizeGateError> {
    if bytes < ARTIFACT_SIZE_LIMIT_BYTES {
        Ok(())
    } else {
        Err(SizeGateError::ArtifactTooLarge {
            actual: bytes,
            limit: ARTIFACT_SIZE_LIMIT_BYTES,
        })
    }
}
