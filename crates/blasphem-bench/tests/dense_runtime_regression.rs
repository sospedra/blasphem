use std::{fs, hint::black_box, path::Path, time::Instant};

use toxbench::{FixtureKind, FixtureLength, latency_gate, load_benchmark_fixtures, nearest_rank};
use toxcheck::{Language, NudgeDetector, ReplyTarget};

const EXPECTED_DENSE_FIXTURES: usize = 30;
const SAMPLES: usize = 64;
const WARM_UP_CALLS: usize = 16;

#[test]
#[cfg_attr(debug_assertions, ignore = "latency gates require a release build")]
fn dense_messages_meet_the_public_check_latency_gates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = load_benchmark_fixtures(&root.join("tests/fixtures/benchmark/messages.jsonl"))
        .expect("benchmark fixtures");
    let dense_fixtures = fixtures
        .iter()
        .filter(|fixture| fixture.kind == FixtureKind::Dense)
        .collect::<Vec<_>>();

    assert_eq!(dense_fixtures.len(), EXPECTED_DENSE_FIXTURES);
    for language in Language::ALL {
        for length in [
            FixtureLength::UnicodeScalars280,
            FixtureLength::Utf8Bytes4096,
        ] {
            assert_eq!(
                dense_fixtures
                    .iter()
                    .filter(|fixture| fixture.language == language && fixture.length == length)
                    .count(),
                1,
                "missing or duplicate dense fixture for {} and {length:?}",
                language.code(),
            );
        }
    }

    for language in Language::ALL {
        let hurtlex_path = root
            .join("data/raw-v1/hurtlex")
            .join(language.storage_code())
            .join("1.2")
            .join(format!("hurtlex_{}.tsv", language.storage_code()));
        let hurtlex = fs::read(hurtlex_path).expect("HurtLex data");
        let detector = NudgeDetector::from_hurtlex_bytes(language, Some(&hurtlex))
            .expect("initialized detector");

        for fixture in dense_fixtures
            .iter()
            .copied()
            .filter(|fixture| fixture.language == language)
        {
            for _ in 0..WARM_UP_CALLS {
                black_box(detector.check(black_box(&fixture.text), ReplyTarget::Unknown));
            }

            let mut samples = Vec::with_capacity(SAMPLES);
            for _ in 0..SAMPLES {
                let start = Instant::now();
                black_box(detector.check(black_box(&fixture.text), ReplyTarget::Unknown));
                samples.push(u64::try_from(start.elapsed().as_nanos()).expect("u64 latency"));
            }
            samples.sort_unstable();
            let p95 = nearest_rank(&samples, 95).expect("p95");

            assert!(
                latency_gate(fixture.length, p95),
                "{} failed the {:?} gate with p95={}ns",
                fixture.id,
                fixture.length,
                p95,
            );
        }
    }
}
