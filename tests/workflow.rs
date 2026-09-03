use std::fs;

use blasphem::{
    EvalLabel, EvalRow, LevelSelection, LexiconEntry, MatchLevel, PolicyAction, evaluate,
    evaluate_policy, load_lexica,
};
use tempfile::tempdir;

const EN_LEXICON: &str = concat!(
    "id\tpos\tcategory\tstereotype\tlemma\tlevel\n",
    "EN1\tn\tcds\tno\tbuffoon\tconservative\n",
    "EN2\tn\tsvp\tno\tpride\tinclusive\n",
);

#[test]
fn loads_only_requested_languages_and_levels() {
    let directory = tempdir().expect("temporary directory");
    fs::write(directory.path().join("hurtlex_EN.tsv"), EN_LEXICON).expect("write English fixture");
    fs::write(
        directory.path().join("hurtlex_ES.tsv"),
        EN_LEXICON.replace("EN", "ES"),
    )
    .expect("write Spanish fixture");

    let entries = load_lexica(
        directory.path(),
        &["EN".to_owned()],
        LevelSelection::Conservative,
    )
    .expect("load fixtures");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].language, "EN");
    assert_eq!(entries[0].lemma, "buffoon");
}

#[test]
fn evaluates_each_message_with_its_language_lexicon() {
    let directory = tempdir().expect("temporary directory");
    fs::write(directory.path().join("hurtlex_EN.tsv"), EN_LEXICON).expect("write English fixture");
    let entries = load_lexica(
        directory.path(),
        &["EN".to_owned()],
        LevelSelection::Conservative,
    )
    .expect("load fixtures");
    let rows = vec![
        EvalRow {
            language: "EN".to_owned(),
            label: EvalLabel::Toxic,
            text: "You are a buffoon".to_owned(),
        },
        EvalRow {
            language: "EN".to_owned(),
            label: EvalLabel::Clean,
            text: "Thank you for your help".to_owned(),
        },
    ];

    let report = evaluate(&rows, entries, 0.8).expect("evaluate fixtures");

    assert_eq!(report.overall.true_positive, 1);
    assert_eq!(report.overall.true_negative, 1);
    assert_eq!(report.by_language["EN"].true_positive, 1);
    assert_eq!(report.by_language["EN"].true_negative, 1);
}

#[test]
fn rejects_an_invalid_threshold() {
    let error = evaluate(&[], Vec::new(), f64::NAN).expect_err("threshold must be finite");

    assert_eq!(error.to_string(), "threshold must be between 0 and 1");
}

#[test]
fn policy_evaluation_detects_a_threat_without_hurtlex_support() {
    let rows = vec![EvalRow {
        language: "EN".to_owned(),
        label: EvalLabel::Toxic,
        text: "I will kill you".to_owned(),
    }];
    let entries = vec![entry("EN", "idiot")];

    let report = evaluate_policy(&rows, entries, PolicyAction::Review).expect("policy evaluation");

    assert_eq!(report.overall.true_positive, 1);
}

#[test]
fn block_minimum_treats_a_profanity_review_as_clean() {
    let rows = vec![
        EvalRow {
            language: "EN".to_owned(),
            label: EvalLabel::Clean,
            text: "idiot".to_owned(),
        },
        EvalRow {
            language: "EN".to_owned(),
            label: EvalLabel::Toxic,
            text: "I will kill you".to_owned(),
        },
    ];

    let report = evaluate_policy(&rows, vec![entry("EN", "idiot")], PolicyAction::Block)
        .expect("policy evaluation");

    assert_eq!(report.overall.true_negative, 1);
    assert_eq!(report.overall.true_positive, 1);
}

#[test]
fn review_evaluation_uses_the_spanish_sparse_nudge() {
    let rows = vec![EvalRow {
        language: "ES".to_owned(),
        label: EvalLabel::Toxic,
        text: "marrana asquerosa farsante hipócrita manipuladora mentirosa traidora corrupta ladrona asesina genocida dictadora cínica descarada desgraciada".to_owned(),
    }];

    let report = evaluate_policy(
        &rows,
        vec![entry("ES", "irrelevante")],
        PolicyAction::Review,
    )
    .expect("policy evaluation");

    assert_eq!(report.overall.true_positive, 1);
}

#[test]
fn policy_evaluation_normalizes_and_isolates_language_groups() {
    let rows = vec![
        EvalRow {
            language: " en ".to_owned(),
            label: EvalLabel::Clean,
            text: "tonto".to_owned(),
        },
        EvalRow {
            language: " es ".to_owned(),
            label: EvalLabel::Clean,
            text: "idiot".to_owned(),
        },
    ];
    let entries = vec![entry(" en ", "idiot"), entry(" es ", "tonto")];

    let report =
        evaluate_policy(&rows, entries, PolicyAction::Review).expect("normalized language groups");

    assert_eq!(report.overall.true_negative, 2);
    assert_eq!(report.by_language["EN"].true_negative, 1);
    assert_eq!(report.by_language["ES"].true_negative, 1);
}

fn entry(language: &str, lemma: &str) -> LexiconEntry {
    LexiconEntry {
        id: format!("{language}-{lemma}"),
        language: language.to_owned(),
        part_of_speech: "n".to_owned(),
        category: "cds".to_owned(),
        stereotype: false,
        lemma: lemma.to_owned(),
        level: MatchLevel::Conservative,
    }
}
