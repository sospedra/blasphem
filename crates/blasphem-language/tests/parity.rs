use eldc::{Detection, Detector, Language};
use serde::Deserialize;

const FIXTURE: &str = include_str!("fixtures/c-parity-v1.jsonl");
const SCORE_TOLERANCE: f32 = 0.000_001;

#[derive(Debug, Deserialize)]
struct FixtureRow {
    id: String,
    category: String,
    input: String,
    language: Option<String>,
    reliable: bool,
    feature_count: usize,
    top_score: f32,
    second_score: f32,
    ranked_scores: Vec<FixtureScore>,
}

#[derive(Debug, Deserialize)]
struct FixtureScore {
    language: String,
    score: f32,
}

#[test]
fn rust_matches_the_frozen_c_oracle() {
    let detector = Detector::new().expect("the embedded model must load");
    let rows = fixture_rows();

    assert!(!rows.is_empty(), "the C parity fixture must contain rows");
    for row in rows {
        let actual = detector.detect(&row.input);
        assert_detection(&row, &actual);
    }
}

fn fixture_rows() -> Vec<FixtureRow> {
    FIXTURE
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .unwrap_or_else(|error| panic!("fixture line {} is invalid: {error}", index + 1))
        })
        .collect()
}

fn assert_detection(row: &FixtureRow, actual: &Detection) {
    let context = format!("{} ({})", row.id, row.category);
    assert_eq!(
        actual.language.map(Language::code),
        row.language.as_deref(),
        "top language differs for {context}"
    );
    assert_eq!(
        actual.reliable, row.reliable,
        "reliability differs for {context}"
    );
    assert_eq!(
        actual.feature_count, row.feature_count,
        "feature count differs for {context}"
    );
    assert_score(actual.top_score, row.top_score, "top score", &context);
    assert_score(
        actual.second_score,
        row.second_score,
        "second score",
        &context,
    );
    assert_eq!(
        actual.ranked_scores.len(),
        row.ranked_scores.len(),
        "ranked score count differs for {context}"
    );

    for (index, (actual_score, expected_score)) in actual
        .ranked_scores
        .iter()
        .zip(&row.ranked_scores)
        .enumerate()
    {
        assert_eq!(
            actual_score.language.code(),
            expected_score.language,
            "ranked language {index} differs for {context}"
        );
        assert_score(
            actual_score.score,
            expected_score.score,
            &format!("ranked score {index}"),
            &context,
        );
    }
}

fn assert_score(actual: f32, expected: f32, field: &str, context: &str) {
    let error = (actual - expected).abs();
    assert!(
        error <= SCORE_TOLERANCE,
        "{field} differs for {context}: expected {expected:.9}, got {actual:.9}, error {error:.9}"
    );
}
