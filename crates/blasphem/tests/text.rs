use blasphem::{
    CandidateViewKind, Detector, LexiconEntry, MatchLevel, TextDocument, normalize_text,
};

fn detector(lemma: &str) -> Detector {
    Detector::new(vec![LexiconEntry {
        id: "EN-1".to_owned(),
        language: "EN".to_owned(),
        part_of_speech: "n".to_owned(),
        category: "cds".to_owned(),
        stereotype: false,
        lemma: lemma.to_owned(),
        level: MatchLevel::Conservative,
    }])
    .expect("valid detector")
}

#[test]
fn preserves_original_byte_spans_in_the_normalized_view() {
    let document = TextDocument::new("ERES ESTÚPIDO");
    let view = document.view(CandidateViewKind::Normalized);

    assert_eq!(view.text(), "eres estupido");
    assert_eq!(view.text(), normalize_text("ERES ESTÚPIDO"));
    assert_eq!(view.original_span(5, 13), Some(5..14));
}

#[test]
fn joins_only_a_run_of_separated_single_letters() {
    let document = TextDocument::new("you are i.d.i.o.t, class stays whole");
    let view = document.view(CandidateViewKind::Evasion);

    assert_eq!(view.text(), "you are idiot class stays whole");
}

#[test]
fn maps_digits_only_inside_a_mixed_token() {
    let document = TextDocument::new("you are 1d10t and scored 101");
    let view = document.view(CandidateViewKind::Evasion);

    assert_eq!(view.text(), "you are idiot and scored 101");
}

#[test]
fn builds_a_unicode_skeleton_from_the_confusable_view() {
    let document = TextDocument::new("idiоt");
    let view = document.view(CandidateViewKind::Confusable);

    assert_eq!(view.text(), "idiot");
}

#[test]
fn reports_the_original_span_for_a_normalized_match() {
    let detector = detector("estúpido");
    let result = detector.check("ERES ESTÚPIDO");

    assert_eq!(result.matches[0].raw_start, 5);
    assert_eq!(result.matches[0].raw_end, 14);
    assert_eq!(result.matches[0].view, CandidateViewKind::Normalized);
}

#[test]
fn matches_a_separated_letter_evasion_candidate() {
    let detector = detector("idiot");
    let result = detector.check("i.d.i.o.t");

    assert!(result.is_match());
    assert_eq!(result.matches[0].view, CandidateViewKind::Evasion);
    assert_eq!(result.matches[0].raw_start, 0);
    assert_eq!(result.matches[0].raw_end, 9);
}
