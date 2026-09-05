use std::path::Path;

use blasphem_bench::{
    FixtureLength, assert_unique_dimensions, load_benchmark_fixtures, sha256_hex,
};

#[test]
fn benchmark_fixture_matrix_is_complete_and_exact() {
    let path = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/blasphem/tests/fixtures/benchmark/messages.jsonl",
    ));
    let fixtures = load_benchmark_fixtures(path).expect("fixtures");

    assert_eq!(fixtures.len(), 90);
    assert_unique_dimensions(&fixtures).expect("unique dimensions");
    for fixture in fixtures {
        match fixture.length {
            FixtureLength::UnicodeScalars280 => assert_eq!(fixture.text.chars().count(), 280),
            FixtureLength::Utf8Bytes4096 => assert_eq!(fixture.text.len(), 4_096),
        }
        assert_eq!(sha256_hex(fixture.text.as_bytes()), fixture.sha256);
    }
}

#[test]
fn rate_formulas_use_the_measured_sample_total() {
    let rates = blasphem_bench::calculate_rates(100, 280, 1_000_000_000).expect("rates");

    assert!((rates.checks_per_second - 100.0).abs() <= 1.0e-12);
    assert!((rates.bytes_per_second - 28_000.0).abs() <= 1.0e-12);
}

#[test]
fn nearest_rank_percentiles_use_literal_sample_ranks() {
    let sorted = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    assert_eq!(blasphem_bench::nearest_rank(&sorted, 50), Some(5));
    assert_eq!(blasphem_bench::nearest_rank(&sorted, 95), Some(10));
    assert_eq!(blasphem_bench::nearest_rank(&[], 95), None);
}

#[test]
fn fixture_latency_gates_use_strict_length_specific_limits() {
    assert!(blasphem_bench::latency_gate(
        FixtureLength::UnicodeScalars280,
        999_999
    ));
    assert!(!blasphem_bench::latency_gate(
        FixtureLength::UnicodeScalars280,
        1_000_000,
    ));
    assert!(blasphem_bench::latency_gate(
        FixtureLength::Utf8Bytes4096,
        9_999_999,
    ));
    assert!(!blasphem_bench::latency_gate(
        FixtureLength::Utf8Bytes4096,
        10_000_000,
    ));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn peak_rss_reports_allocated_bytes() {
    let allocation = vec![1_u8; 8 * 1024 * 1024];
    std::hint::black_box(&allocation);
    let peak_rss = blasphem_bench::peak_rss_bytes().expect("peak RSS");
    assert!(peak_rss >= u64::try_from(allocation.len()).expect("allocation size"));
}
