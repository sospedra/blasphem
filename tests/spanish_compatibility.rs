use sha2::{Digest, Sha256};
use toxcheck::{
    AnalysisContext, Detector, Language, MatchLevel, NudgeDetector, ReplyTarget, parse_hurtlex,
};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn spanish_assets_are_frozen() {
    let model = include_bytes!("../resources/models/es-chargram-v1.bin");
    let audit = include_bytes!("../samples/spanish-audit.tsv");
    assert_eq!(
        sha256_hex(model),
        "3e09ea4ef4db50f8e9024f5a2cfe14d428d0114e97e5d7defe9764184e4dae36"
    );
    assert_eq!(
        sha256_hex(audit),
        "8313713f8e18e5c066f6f320efb6ee340b7580cba4739fc4612e1dfe4a8a7575"
    );
}

use std::process::Command;

fn check(text: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_toxcheck"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "check",
            "--language",
            "ES",
            "--data-dir",
            "data/raw-v1/hurtlex",
            "--text",
            text,
        ])
        .output()
        .expect("run toxcheck");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("UTF-8 output")
}

#[test]
fn spanish_product_results_are_frozen() {
    assert!(
        check("Te voy a matar").starts_with("ok=false score=95 threshold=50 should_nudge=true")
    );
    assert!(
        check("No te voy a matar").starts_with("ok=true score=24 threshold=50 should_nudge=false")
    );
}

#[test]
fn spanish_runtime_matches_the_legacy_policy_for_the_complete_behavior_panel() {
    let hurtlex = include_bytes!("../data/raw-v1/hurtlex/ES/1.2/hurtlex_ES.tsv");
    let entries = parse_hurtlex(&hurtlex[..], "ES")
        .expect("Spanish HurtLex")
        .into_iter()
        .filter(|entry| entry.level == MatchLevel::Conservative)
        .collect();
    let legacy = Detector::new(entries).expect("legacy detector");
    let runtime =
        NudgeDetector::from_hurtlex_bytes(Language::Es, Some(hurtlex)).expect("runtime detector");
    let panel = include_str!("../samples/spanish-audit.tsv");

    for line in panel.lines().skip(1) {
        let mut fields = line.splitn(3, '\t');
        assert_eq!(fields.next(), Some("ES"));
        let _label = fields.next().expect("panel label");
        let text = fields.next().expect("panel text");
        let expected = legacy.analyze(text, AnalysisContext::for_language("ES"));
        let actual = runtime.analyze(text, ReplyTarget::Unknown);

        assert_eq!(actual, expected, "{text}");
    }
}
