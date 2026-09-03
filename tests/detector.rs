use std::io::Cursor;

use blasphem::{Detector, LexiconEntry, MatchLevel, parse_hurtlex};

fn entry(language: &str, lemma: &str, level: MatchLevel) -> LexiconEntry {
    LexiconEntry {
        id: format!("{language}-1"),
        language: language.to_owned(),
        part_of_speech: "n".to_owned(),
        category: "cds".to_owned(),
        stereotype: false,
        lemma: lemma.to_owned(),
        level,
    }
}

#[test]
fn parses_hurtlex_rows_with_metadata() {
    let input = concat!(
        "id\tpos\tcategory\tstereotype\tlemma\tlevel\n",
        "ES1\tn\tcds\tyes\tidiota\tconservative\n",
    );

    let entries = parse_hurtlex(Cursor::new(input), "es").expect("valid HurtLex data");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].language, "ES");
    assert_eq!(entries[0].lemma, "idiota");
    assert_eq!(entries[0].level, MatchLevel::Conservative);
    assert!(entries[0].stereotype);
}

#[test]
fn matches_a_normalized_multilingual_phrase() {
    let detector = Detector::new(vec![entry("ES", "estúpido", MatchLevel::Conservative)])
        .expect("valid detector");

    let result = detector.check("ERES ESTÚPIDO");

    assert!(result.is_match());
    assert_eq!(result.matches[0].entry.language, "ES");
    assert_eq!(result.matches[0].entry.lemma, "estúpido");
}

#[test]
fn matches_a_spanish_plural_from_a_nominal_lemma() {
    let source = entry("ES", "imbécil", MatchLevel::Conservative);
    let detector = Detector::new(vec![source.clone()]).expect("valid detector");

    let result = detector.check("Sois unos imbéciles.");

    assert!(result.is_match());
    assert_eq!(*result.matches[0].entry, source);
    assert_eq!(
        &"Sois unos imbéciles."[result.matches[0].raw_start..result.matches[0].raw_end],
        "imbéciles"
    );
}

#[test]
fn matches_high_confidence_spanish_plural_rules() {
    let detector = Detector::new(vec![
        entry("ES", "rata", MatchLevel::Conservative),
        entry("ES", "incapaz", MatchLevel::Conservative),
    ])
    .expect("valid detector");

    let result = detector.check("ratas incapaces");
    let lemmas = result
        .matches
        .iter()
        .map(|found| found.entry.lemma.as_str())
        .collect::<Vec<_>>();

    assert_eq!(lemmas, ["rata", "incapaz"]);
}

#[test]
fn limits_spanish_plural_rules_by_language_pos_and_token_count() {
    let mut verb = entry("ES", "rata", MatchLevel::Conservative);
    verb.part_of_speech = "v".to_owned();
    let portuguese = entry("PT", "rata", MatchLevel::Conservative);
    let phrase = entry("ES", "mala rata", MatchLevel::Conservative);
    let detector = Detector::new(vec![verb, portuguese, phrase]).expect("valid detector");

    let result = detector.check("ratas malas ratas");

    assert!(!result.is_match());
}

#[test]
fn does_not_invent_spanish_gender_variants() {
    let mut adjective = entry("ES", "negro", MatchLevel::Conservative);
    adjective.part_of_speech = "a".to_owned();
    let detector = Detector::new(vec![adjective]).expect("valid detector");

    let result = detector.check("Las paredes negras.");

    assert!(!result.is_match());
}

#[test]
fn does_not_invent_a_feminine_form_for_a_spanish_noun() {
    let detector = Detector::new(vec![entry("ES", "servicio", MatchLevel::Conservative)])
        .expect("valid detector");

    let result = detector.check("La palabra servicia no existe.");

    assert!(!result.is_match());
}

#[test]
fn rejects_a_match_inside_a_longer_word() {
    let detector =
        Detector::new(vec![entry("EN", "ass", MatchLevel::Conservative)]).expect("valid detector");

    let result = detector.check("The class starts now");

    assert!(!result.is_match());
}

#[test]
fn matches_entries_inside_unspaced_chinese_and_japanese_text() {
    for (language, lemma, text) in [
        ("ZH", "烂裤裆", "什么烂裤裆？"),
        ("JA", "クズ", "お前はクズだ"),
    ] {
        let detector = Detector::new(vec![entry(language, lemma, MatchLevel::Conservative)])
            .expect("valid detector");

        let result = detector.check(text);

        assert!(result.is_match(), "{language}");
        assert_eq!(result.matches[0].entry.lemma, lemma);
    }
}

#[test]
fn keeps_hangul_entries_word_delimited() {
    let detector = Detector::new(vec![entry("KO", "돼지새끼", MatchLevel::Conservative)])
        .expect("valid detector");

    assert!(!detector.check("북괴돼지새끼 살아있으면 좋겠네").is_match());
    assert!(detector.check("돼지새끼 같아").is_match());
}

#[test]
fn matches_a_unicode_confusable_view() {
    let detector = Detector::new(vec![entry("EN", "idiot", MatchLevel::Conservative)])
        .expect("valid detector");

    let result = detector.check("idiоt");

    assert!(result.is_match());
    assert!(result.matches[0].matched_confusable_view);
}
