use std::{fs, hint::black_box, path::Path, time::Instant};

use anyhow::{Context, Result};
use toxbench::{FixtureKind, FixtureLength, load_benchmark_fixtures};
use toxcheck::{
    Detector, Language, MatchLevel, NudgeDetector, ReplyTarget, SparseModel, analyze_with_rules,
    arabic_hindi_rules, cjk_rules, parse_hurtlex, word_rules,
};

fn main() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = load_benchmark_fixtures(&root.join("tests/fixtures/benchmark/messages.jsonl"))?;
    for language in [Language::En, Language::Es, Language::Ar, Language::Ja] {
        let fixture = fixtures
            .iter()
            .find(|fixture| {
                fixture.language == language
                    && fixture.kind == FixtureKind::Dense
                    && fixture.length == FixtureLength::Utf8Bytes4096
            })
            .context("missing dense fixture")?;
        let hurtlex_path = root
            .join("data/raw-v1/hurtlex")
            .join(language.storage_code())
            .join("1.2")
            .join(format!("hurtlex_{}.tsv", language.storage_code()));
        let hurtlex = fs::read(&hurtlex_path)?;
        let entries = parse_hurtlex(hurtlex.as_slice(), language.storage_code())?
            .into_iter()
            .filter(|entry| entry.level == MatchLevel::Conservative)
            .collect();
        let lexical = Detector::new(entries)?;
        let model_name = if language == Language::Es {
            "es-chargram-v1.bin".to_owned()
        } else {
            format!(
                "{}-sparse-v2.bin",
                language.storage_code().to_ascii_lowercase()
            )
        };
        let model = SparseModel::from_bytes(&fs::read(
            root.join("resources/models/multilingual-v2")
                .join(model_name),
        )?)?;
        let rules = word_rules(language)
            .or_else(|| arabic_hindi_rules(language))
            .or_else(|| cjk_rules(language));
        let detector = NudgeDetector::from_hurtlex_bytes(language, Some(&hurtlex))?;
        let nudge = detector.check(&fixture.text, ReplyTarget::Unknown);

        let lexical_result = lexical.check(&fixture.text);
        let semantic_count = rules.map_or(0, |rules| {
            analyze_with_rules(rules, &fixture.text, ReplyTarget::Unknown)
                .evidence
                .len()
        });
        println!(
            "{} lexical_matches={} semantic_events={} sparse_score={} result_score={} should_nudge={}",
            language.code(),
            lexical_result.matches.len(),
            semantic_count,
            model.score(&fixture.text),
            nudge.score,
            nudge.should_nudge,
        );
        time("lexical", 50, || lexical.check(black_box(&fixture.text)));
        time("sparse", 50, || model.score(black_box(&fixture.text)));
        if let Some(rules) = rules {
            time("semantic", 50, || {
                analyze_with_rules(rules, black_box(&fixture.text), ReplyTarget::Unknown)
            });
        }
        time("full-check", 50, || {
            detector.check(black_box(&fixture.text), ReplyTarget::Unknown)
        });
    }
    Ok(())
}

fn time<T>(label: &str, samples: u32, mut operation: impl FnMut() -> T) {
    let start = Instant::now();
    for _ in 0..samples {
        black_box(operation());
    }
    let per_call = start.elapsed().as_nanos() / u128::from(samples);
    println!("  {label}={per_call}ns");
}
