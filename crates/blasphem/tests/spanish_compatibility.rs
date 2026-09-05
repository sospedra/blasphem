use blasphem::{
    Detector, Language, MatchLevel, NudgeDetector, ReplyTarget, RuleContext, parse_lexicon,
};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn spanish_assets_match_the_published_manifest() {
    let model = include_bytes!("../../../resources/models/es-sparse.bin");
    let audit = include_bytes!("fixtures/spanish-audit.tsv");
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../resources/metadata/model-manifest.json"
    ))
    .expect("model manifest");
    let published = manifest["entries"]
        .as_array()
        .expect("manifest entries")
        .iter()
        .find(|entry| entry["language"] == "ES")
        .expect("Spanish manifest entry");

    assert_eq!(
        published["artifact_sha256"].as_str(),
        Some(sha256_hex(model).as_str())
    );
    assert_eq!(
        published["artifact_bytes"].as_u64(),
        Some(model.len() as u64)
    );
    assert_eq!(
        sha256_hex(audit),
        "8313713f8e18e5c066f6f320efb6ee340b7580cba4739fc4612e1dfe4a8a7575"
    );
}

use std::process::Command;

fn check(text: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .args([
            "check",
            "--language",
            "ES",
            "--data-dir",
            "resources/lexicon",
            "--text",
            text,
        ])
        .output()
        .expect("run blasphem");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

#[test]
fn spanish_product_results_are_frozen() {
    assert!(
        check("Te voy a matar").starts_with("ok=false score=95 threshold=50 should_nudge=true")
    );
    assert!(
        check("No te voy a matar").starts_with("ok=true score=30 threshold=50 should_nudge=false")
    );
}

#[test]
fn spanish_runtime_preserves_rules_for_the_complete_behavior_panel() {
    let lexicon = include_bytes!("../../../resources/lexicon/ES.tsv");
    let entries = parse_lexicon(&lexicon[..], "ES")
        .expect("Spanish Lexicon")
        .into_iter()
        .filter(|entry| entry.level == MatchLevel::Conservative)
        .collect();
    let rules = Detector::new(entries).expect("rule detector");
    let runtime =
        NudgeDetector::from_lexicon_bytes(Language::Es, Some(lexicon)).expect("runtime detector");
    let panel = include_str!("fixtures/spanish-audit.tsv");

    for line in panel.lines().skip(1) {
        let mut fields = line.splitn(3, '\t');
        assert_eq!(fields.next(), Some("ES"));
        let _label = fields.next().expect("panel label");
        let text = fields.next().expect("panel text");
        let expected = rules.analyze_rules(text, RuleContext::for_language("ES"));
        let actual = runtime.analyze(text, ReplyTarget::Unknown);

        assert_eq!(actual.scores, expected.scores, "{text}");
        assert_eq!(actual.evidence, expected.evidence, "{text}");
        assert_eq!(actual.lexical, expected.lexical, "{text}");
    }
}
