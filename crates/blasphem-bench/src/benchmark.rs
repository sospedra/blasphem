use std::{
    collections::BTreeMap,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::Instant,
};

use blasphem::{
    Language, LanguageDetection, LanguageIdentifier, LanguageResolution, NudgeDetector, ReplyTarget,
};
use serde::{Deserialize, Serialize};

use crate::{
    BenchmarkError, BenchmarkFixture, FixtureLength, assert_unique_dimensions, calculate_rates,
    load_benchmark_fixtures, nearest_rank, sha256_hex,
};

const SHORT_SAMPLES: u64 = 5_000;
const LONG_SAMPLES: u64 = 1_000;
const WARM_UP_CALLS: usize = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimingSummary {
    pub samples: u64,
    pub input_bytes: u64,
    pub p50_nanoseconds: u64,
    pub p95_nanoseconds: u64,
    pub p99_nanoseconds: u64,
    pub maximum_nanoseconds: u64,
    pub checks_per_second: f64,
    pub bytes_per_second: f64,
    pub peak_rss_bytes: u64,
    pub latency_gate_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BenchmarkEvidence {
    pub schema_version: u16,
    pub evidence_status: String,
    pub computer: String,
    pub rust_version: String,
    pub target_triple: String,
    pub model_manifest_sha256: String,
    pub fixtures_sha256: String,
    pub fixtures: BTreeMap<String, TimingSummary>,
    pub peak_rss_bytes: u64,
    pub all_latency_gates_passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoTimingSummary {
    pub samples: u64,
    pub input_bytes: u64,
    pub p50_nanoseconds: u64,
    pub p95_nanoseconds: u64,
    pub p99_nanoseconds: u64,
    pub maximum_nanoseconds: u64,
    pub checks_per_second: f64,
    pub bytes_per_second: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoTimingEvidence {
    pub fixtures_sha256: String,
    pub fixture_count: u64,
    pub eligible_fixture_count: u64,
    pub warm_up_checks: u64,
    pub rejected_fixtures: Vec<AutoTimingRejection>,
    pub groups: BTreeMap<String, AutoTimingSummary>,
    pub peak_rss_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutoTimingRejection {
    pub fixture: String,
    pub expected_language: String,
    pub actual_language: Option<String>,
    pub reason: String,
}

#[must_use]
pub const fn latency_gate(length: FixtureLength, p95_nanoseconds: u64) -> bool {
    match length {
        FixtureLength::UnicodeScalars280 => p95_nanoseconds < 1_000_000,
        FixtureLength::Utf8Bytes4096 => p95_nanoseconds < 10_000_000,
    }
}

/// Runs the fixed 90-fixture benchmark through the public Boolean path.
///
/// # Errors
///
/// Returns an error for invalid fixtures, resources, detector initialization, or timing data.
pub fn run_benchmark(
    fixtures_path: &Path,
    model_manifest_path: &Path,
    hurtlex_root: &Path,
    computer: &str,
    target_triple: &str,
    rust_version: &str,
) -> Result<BenchmarkEvidence, BenchmarkError> {
    let fixtures_bytes = fs::read(fixtures_path).map_err(|source| BenchmarkError::FixtureIo {
        path: fixtures_path.to_owned(),
        source,
    })?;
    let fixtures = load_benchmark_fixtures(fixtures_path)?;
    validate_fixtures(&fixtures)?;
    let model_manifest =
        fs::read(model_manifest_path).map_err(|source| BenchmarkError::ResourceIo {
            path: model_manifest_path.to_owned(),
            source,
        })?;

    let mut detectors = BTreeMap::new();
    for language in Language::ALL {
        let path = hurtlex_path(hurtlex_root, language);
        let bytes = fs::read(&path).map_err(|source| BenchmarkError::ResourceIo {
            path: path.clone(),
            source,
        })?;
        let detector =
            NudgeDetector::from_hurtlex_bytes(language, Some(&bytes)).map_err(|source| {
                BenchmarkError::Detector {
                    language,
                    reason: source.to_string(),
                }
            })?;
        detectors.insert(language, detector);
    }

    let mut summaries = BTreeMap::new();
    let mut process_peak_rss = 0_u64;
    for fixture in &fixtures {
        let detector = detectors
            .get(&fixture.language)
            .ok_or(BenchmarkError::MissingDetector(fixture.language))?;
        let summary = measure(detector, fixture)?;
        process_peak_rss = process_peak_rss.max(summary.peak_rss_bytes);
        summaries.insert(fixture.id.clone(), summary);
    }

    Ok(BenchmarkEvidence {
        schema_version: 1,
        evidence_status: "experimental".to_owned(),
        computer: computer.to_owned(),
        rust_version: rust_version.to_owned(),
        target_triple: target_triple.to_owned(),
        model_manifest_sha256: sha256_hex(&model_manifest),
        fixtures_sha256: sha256_hex(&fixtures_bytes),
        all_latency_gates_passed: summaries
            .values()
            .all(|summary| summary.latency_gate_passed),
        fixtures: summaries,
        peak_rss_bytes: process_peak_rss,
    })
}

/// Measures the complete automatic route over the fixed 90-fixture matrix.
///
/// # Errors
///
/// Returns an error for invalid fixtures, resources, routes, or timing values.
pub fn run_auto_timing<I: LanguageIdentifier + ?Sized>(
    identifier: &I,
    fixtures_path: &Path,
    hurtlex_root: &Path,
) -> Result<AutoTimingEvidence, BenchmarkError> {
    let fixtures_bytes = fs::read(fixtures_path).map_err(|source| BenchmarkError::FixtureIo {
        path: fixtures_path.to_owned(),
        source,
    })?;
    let fixtures = load_benchmark_fixtures(fixtures_path)?;
    validate_fixtures(&fixtures)?;
    let detectors = load_detectors(hurtlex_root)?;

    let mut eligible = Vec::with_capacity(fixtures.len());
    let mut rejected_fixtures = Vec::new();
    for fixture in &fixtures {
        let detection = identifier.identify(&fixture.text);
        match validate_auto_resolution(fixture.language, detection) {
            Ok(_) => eligible.push(fixture),
            Err(actual) => rejected_fixtures.push(AutoTimingRejection {
                fixture: fixture.id.clone(),
                expected_language: fixture.language.code().to_owned(),
                actual_language: actual.map(|language| language.code().to_owned()),
                reason: if actual.is_some() {
                    "misrouted".to_owned()
                } else {
                    "unknown".to_owned()
                },
            }),
        }
    }

    for fixture in &eligible {
        for _ in 0..WARM_UP_CALLS {
            run_auto_check(identifier, &detectors, fixture)?;
        }
    }

    let mut short = AutoTimingPool::default();
    let mut long = AutoTimingPool::default();
    for fixture in &eligible {
        let (samples, pool) = match fixture.length {
            FixtureLength::UnicodeScalars280 => (SHORT_SAMPLES, &mut short),
            FixtureLength::Utf8Bytes4096 => (LONG_SAMPLES, &mut long),
        };
        let fixture_start = Instant::now();
        for _ in 0..samples {
            let start = Instant::now();
            run_auto_check(identifier, &detectors, fixture)?;
            pool.elapsed_samples.push(
                u64::try_from(start.elapsed().as_nanos())
                    .map_err(|_| BenchmarkError::TimingOverflow)?,
            );
        }
        pool.total_elapsed_nanoseconds = pool
            .total_elapsed_nanoseconds
            .checked_add(fixture_start.elapsed().as_nanos())
            .ok_or(BenchmarkError::TimingOverflow)?;
        pool.samples = pool
            .samples
            .checked_add(samples)
            .ok_or(BenchmarkError::TimingOverflow)?;
        let fixture_bytes = u64::try_from(fixture.text.len())
            .map_err(|_| BenchmarkError::TimingOverflow)?
            .checked_mul(samples)
            .ok_or(BenchmarkError::TimingOverflow)?;
        pool.input_bytes = pool
            .input_bytes
            .checked_add(fixture_bytes)
            .ok_or(BenchmarkError::TimingOverflow)?;
    }

    let mut groups = BTreeMap::new();
    groups.insert("unicode_scalars_280".to_owned(), short.finish()?);
    groups.insert("utf8_bytes_4096".to_owned(), long.finish()?);
    let fixture_count =
        u64::try_from(fixtures.len()).map_err(|_| BenchmarkError::TimingOverflow)?;
    let eligible_fixture_count =
        u64::try_from(eligible.len()).map_err(|_| BenchmarkError::TimingOverflow)?;
    let warm_up_checks = eligible_fixture_count
        .checked_mul(u64::try_from(WARM_UP_CALLS).map_err(|_| BenchmarkError::TimingOverflow)?)
        .ok_or(BenchmarkError::TimingOverflow)?;

    Ok(AutoTimingEvidence {
        fixtures_sha256: sha256_hex(&fixtures_bytes),
        fixture_count,
        eligible_fixture_count,
        warm_up_checks,
        rejected_fixtures,
        groups,
        peak_rss_bytes: peak_rss_bytes().ok(),
    })
}

fn load_detectors(root: &Path) -> Result<BTreeMap<Language, NudgeDetector>, BenchmarkError> {
    let mut detectors = BTreeMap::new();
    for language in Language::ALL {
        let path = hurtlex_path(root, language);
        let bytes = fs::read(&path).map_err(|source| BenchmarkError::ResourceIo {
            path: path.clone(),
            source,
        })?;
        let detector =
            NudgeDetector::from_hurtlex_bytes(language, Some(&bytes)).map_err(|source| {
                BenchmarkError::Detector {
                    language,
                    reason: source.to_string(),
                }
            })?;
        detectors.insert(language, detector);
    }
    Ok(detectors)
}

fn run_auto_check<I: LanguageIdentifier + ?Sized>(
    identifier: &I,
    detectors: &BTreeMap<Language, NudgeDetector>,
    fixture: &BenchmarkFixture,
) -> Result<(), BenchmarkError> {
    let detection = black_box(identifier.identify(black_box(&fixture.text)));
    let actual = validate_auto_resolution(fixture.language, detection).map_err(|actual| {
        BenchmarkError::AutoRoute {
            fixture: fixture.id.clone(),
            expected: fixture.language,
            actual,
        }
    })?;
    let detector = detectors
        .get(&actual)
        .ok_or(BenchmarkError::MissingDetector(actual))?;
    black_box(detector.check(black_box(&fixture.text), ReplyTarget::Unknown));
    Ok(())
}

fn validate_auto_resolution(
    expected: Language,
    detection: LanguageDetection,
) -> Result<Language, Option<Language>> {
    let actual = match (detection.reliable, detection.resolution) {
        (true, LanguageResolution::Known(language)) => language,
        _ => return Err(None),
    };
    if actual == expected {
        Ok(actual)
    } else {
        Err(Some(actual))
    }
}

#[derive(Debug, Default)]
struct AutoTimingPool {
    samples: u64,
    input_bytes: u64,
    total_elapsed_nanoseconds: u128,
    elapsed_samples: Vec<u64>,
}

impl AutoTimingPool {
    fn finish(mut self) -> Result<AutoTimingSummary, BenchmarkError> {
        self.elapsed_samples.sort_unstable();
        let p50_nanoseconds =
            nearest_rank(&self.elapsed_samples, 50).ok_or(BenchmarkError::NoSamples)?;
        let p95_nanoseconds =
            nearest_rank(&self.elapsed_samples, 95).ok_or(BenchmarkError::NoSamples)?;
        let p99_nanoseconds =
            nearest_rank(&self.elapsed_samples, 99).ok_or(BenchmarkError::NoSamples)?;
        let maximum_nanoseconds = self
            .elapsed_samples
            .last()
            .copied()
            .ok_or(BenchmarkError::NoSamples)?;
        if self.samples == 0 || self.total_elapsed_nanoseconds == 0 {
            return Err(BenchmarkError::InvalidRateInput);
        }
        let seconds = self.total_elapsed_nanoseconds as f64 / 1_000_000_000.0;
        let checks_per_second = self.samples as f64 / seconds;
        let bytes_per_second = self.input_bytes as f64 / seconds;
        if !checks_per_second.is_finite() || !bytes_per_second.is_finite() {
            return Err(BenchmarkError::NonFiniteRate);
        }
        Ok(AutoTimingSummary {
            samples: self.samples,
            input_bytes: self.input_bytes,
            p50_nanoseconds,
            p95_nanoseconds,
            p99_nanoseconds,
            maximum_nanoseconds,
            checks_per_second,
            bytes_per_second,
        })
    }
}

fn validate_fixtures(fixtures: &[BenchmarkFixture]) -> Result<(), BenchmarkError> {
    if fixtures.len() != 90 {
        return Err(BenchmarkError::WrongFixtureCount(fixtures.len()));
    }
    assert_unique_dimensions(fixtures)?;
    for fixture in fixtures {
        match fixture.length {
            FixtureLength::UnicodeScalars280 if fixture.text.chars().count() != 280 => {
                return Err(BenchmarkError::WrongFixtureLength(fixture.id.clone()));
            }
            FixtureLength::Utf8Bytes4096 if fixture.text.len() != 4_096 => {
                return Err(BenchmarkError::WrongFixtureLength(fixture.id.clone()));
            }
            _ => {}
        }
        if sha256_hex(fixture.text.as_bytes()) != fixture.sha256 {
            return Err(BenchmarkError::FixtureDigestMismatch(fixture.id.clone()));
        }
    }
    Ok(())
}

fn measure(
    detector: &NudgeDetector,
    fixture: &BenchmarkFixture,
) -> Result<TimingSummary, BenchmarkError> {
    for _ in 0..WARM_UP_CALLS {
        black_box(detector.check(black_box(&fixture.text), ReplyTarget::Unknown));
    }
    let samples = match fixture.length {
        FixtureLength::UnicodeScalars280 => SHORT_SAMPLES,
        FixtureLength::Utf8Bytes4096 => LONG_SAMPLES,
    };
    let sample_capacity = usize::try_from(samples).map_err(|_| BenchmarkError::TimingOverflow)?;
    let mut elapsed_samples = Vec::with_capacity(sample_capacity);
    let total_start = Instant::now();
    for _ in 0..samples {
        let start = Instant::now();
        black_box(detector.check(black_box(&fixture.text), ReplyTarget::Unknown));
        let elapsed = start.elapsed().as_nanos();
        elapsed_samples.push(u64::try_from(elapsed).map_err(|_| BenchmarkError::TimingOverflow)?);
    }
    let total_elapsed = total_start.elapsed().as_nanos();
    elapsed_samples.sort_unstable();
    let p50_nanoseconds = nearest_rank(&elapsed_samples, 50).ok_or(BenchmarkError::NoSamples)?;
    let p95_nanoseconds = nearest_rank(&elapsed_samples, 95).ok_or(BenchmarkError::NoSamples)?;
    let p99_nanoseconds = nearest_rank(&elapsed_samples, 99).ok_or(BenchmarkError::NoSamples)?;
    let maximum_nanoseconds = elapsed_samples
        .last()
        .copied()
        .ok_or(BenchmarkError::NoSamples)?;
    let input_bytes =
        u64::try_from(fixture.text.len()).map_err(|_| BenchmarkError::TimingOverflow)?;
    let rates = calculate_rates(samples, input_bytes, total_elapsed)?;
    Ok(TimingSummary {
        samples,
        input_bytes,
        p50_nanoseconds,
        p95_nanoseconds,
        p99_nanoseconds,
        maximum_nanoseconds,
        checks_per_second: rates.checks_per_second,
        bytes_per_second: rates.bytes_per_second,
        peak_rss_bytes: peak_rss_bytes()?,
        latency_gate_passed: latency_gate(fixture.length, p95_nanoseconds),
    })
}

fn hurtlex_path(root: &Path, language: Language) -> PathBuf {
    root.join(language.storage_code())
        .join("1.2")
        .join(format!("hurtlex_{}.tsv", language.storage_code()))
}

/// Reads the process-wide peak resident memory in bytes.
///
/// # Errors
///
/// Returns an error when the operating system cannot provide the value.
#[cfg(target_os = "macos")]
pub fn peak_rss_bytes() -> Result<u64, BenchmarkError> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();

    // SAFETY: The pointer is valid and writable for one libc::rusage value.
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if status != 0 {
        return Err(BenchmarkError::PeakRss(std::io::Error::last_os_error()));
    }

    // SAFETY: A successful getrusage call initialized the complete value.
    let usage = unsafe { usage.assume_init() };
    u64::try_from(usage.ru_maxrss).map_err(|_| BenchmarkError::NegativePeakRss)
}

/// Reports that peak resident memory is unavailable on this target.
///
/// # Errors
///
/// Always returns the unsupported-target error.
#[cfg(not(target_os = "macos"))]
pub fn peak_rss_bytes() -> Result<u64, BenchmarkError> {
    Err(BenchmarkError::UnsupportedPeakRssTarget)
}

#[cfg(test)]
mod tests {
    use blasphem::{LanguageDetection, LanguageSource};

    use super::*;

    #[test]
    fn automatic_timing_rejects_unknown_and_misrouted_fixtures() {
        let unknown = LanguageDetection {
            source: LanguageSource::Automatic,
            resolution: LanguageResolution::Unknown,
            reliable: false,
            score: None,
            feature_count: Some(2),
        };
        let wrong = LanguageDetection {
            source: LanguageSource::Automatic,
            resolution: LanguageResolution::Known(Language::Es),
            reliable: true,
            score: Some(0.9),
            feature_count: Some(10),
        };

        assert_eq!(validate_auto_resolution(Language::En, unknown), Err(None));
        assert_eq!(
            validate_auto_resolution(Language::En, wrong),
            Err(Some(Language::Es))
        );
    }
}
