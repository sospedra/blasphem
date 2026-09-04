use blasphem::{
    CandidateViewKind, Detector, LexiconEntry, MatchLevel, PolicyAction, ReplyTarget, RuleContext,
    RuleId, parse_lexicon,
};

use std::{fs::File, path::PathBuf, sync::OnceLock};

fn entry(language: &str, lemma: &str, category: &str, stereotype: bool) -> LexiconEntry {
    entry_with_level(
        language,
        lemma,
        category,
        stereotype,
        MatchLevel::Conservative,
    )
}

fn entry_with_level(
    language: &str,
    lemma: &str,
    category: &str,
    stereotype: bool,
    level: MatchLevel,
) -> LexiconEntry {
    LexiconEntry {
        id: format!("{language}-{lemma}"),
        language: language.to_owned(),
        part_of_speech: "n".to_owned(),
        category: category.to_owned(),
        stereotype,
        lemma: lemma.to_owned(),
        level,
    }
}

#[test]
fn scores_duplicate_conservative_rows_as_one_lexical_event() {
    let detector = detector(vec![
        entry("EN", "idiot", "cds", false),
        entry("EN", "idiot", "cds", false),
    ]);
    let result = detector.analyze_rules("You idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 70);
    assert_eq!(
        result
            .evidence
            .iter()
            .filter(|item| item.rule_id == RuleId::LexicalMatch)
            .count(),
        1
    );
}

#[test]
fn selects_the_strongest_level_for_duplicate_rows() {
    let detector = detector(vec![
        entry_with_level("EN", "idiot", "cds", false, MatchLevel::Inclusive),
        entry("EN", "idiot", "cds", false),
    ]);
    let result = detector.analyze_rules("Idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 30);
}

#[test]
fn scores_each_candidate_before_selecting_the_strongest_match() {
    let detector = detector(vec![
        entry("EN", "іdiot", "cds", false),
        entry_with_level("EN", "idiot", "cds", false, MatchLevel::Inclusive),
    ]);
    let result = detector.analyze_rules("іdiot", RuleContext::for_language("EN"));

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 30);
    let evidence = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::LexicalMatch)
        .expect("lexical evidence");
    assert_eq!(evidence.candidate_view, Some(CandidateViewKind::Normalized));
    assert_eq!(evidence.normalized_start, Some(0));
    assert_eq!(evidence.normalized_end, Some(7));
}

#[test]
fn identity_evidence_uses_the_identity_candidate_when_a_stronger_nonidentity_view_shares_its_span()
{
    let detector = detector(vec![
        entry("EN", "іdiot", "cds", false),
        entry_with_level("EN", "idiot", "ps", true, MatchLevel::Inclusive),
    ]);
    let result = detector.analyze_rules("Immigrants are іdiot.", RuleContext::for_language("EN"));

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);

    let lexical = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::LexicalMatch)
        .expect("lexical evidence");
    assert_eq!(lexical.candidate_view, Some(CandidateViewKind::Normalized));

    for rule_id in [
        RuleId::IdentityGroupTarget,
        RuleId::IdentityStereotypeSupport,
    ] {
        let evidence = result
            .evidence
            .iter()
            .find(|item| item.rule_id == rule_id)
            .expect("identity evidence");
        assert_eq!(evidence.candidate_view, Some(CandidateViewKind::Confusable));
        assert_eq!(evidence.normalized_start, Some(17));
        assert_eq!(evidence.normalized_end, Some(22));
        assert_eq!(evidence.raw_start, Some(15));
        assert_eq!(evidence.raw_end, Some(21));
        assert_eq!(evidence.matched_text, "іdiot");
    }
}

#[test]
fn stereotype_evidence_uses_the_stereotype_candidate_when_views_share_a_raw_span() {
    let detector = detector(vec![
        entry("EN", "іdiot", "cds", true),
        entry_with_level("EN", "idiot", "ps", false, MatchLevel::Inclusive),
    ]);
    let result = detector.analyze_rules("Immigrants are іdiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);

    let identity = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::IdentityGroupTarget)
        .expect("identity evidence");
    assert_eq!(identity.candidate_view, Some(CandidateViewKind::Confusable));
    assert_eq!(identity.normalized_start, Some(17));
    assert_eq!(identity.normalized_end, Some(22));

    let stereotype = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::IdentityStereotypeSupport)
        .expect("stereotype evidence");
    assert_eq!(
        stereotype.candidate_view,
        Some(CandidateViewKind::Normalized)
    );
    assert_eq!(stereotype.normalized_start, Some(15));
    assert_eq!(stereotype.normalized_end, Some(22));
    assert_eq!(stereotype.raw_start, Some(15));
    assert_eq!(stereotype.raw_end, Some(21));
    assert_eq!(stereotype.matched_text, "іdiot");
}

#[test]
fn lexical_evidence_copies_evasion_candidate_provenance() {
    let detector = detector(vec![entry("EN", "idiot", "cds", false)]);
    let result = detector.analyze_rules("i.d.i.o.t", RuleContext::for_language("EN"));
    let evidence = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::LexicalMatch)
        .expect("lexical evidence");

    assert_eq!(result.scores.profanity, 35);
    assert_eq!(evidence.points, 35);
    assert_eq!(evidence.matched_text, "i.d.i.o.t");
    assert_eq!(evidence.candidate_view, Some(CandidateViewKind::Evasion));
    assert_eq!(evidence.normalized_start, Some(0));
    assert_eq!(evidence.normalized_end, Some(5));
    assert_eq!(evidence.raw_start, Some(0));
    assert_eq!(evidence.raw_end, Some(9));
}

#[test]
fn lexical_evidence_keeps_normalized_and_raw_offsets_distinct() {
    let detector = detector(vec![entry("ES", "estúpido", "cds", false)]);
    let result = detector.analyze_rules("ERES ESTÚPIDO", RuleContext::for_language("ES"));
    let evidence = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::LexicalMatch)
        .expect("lexical evidence");

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(evidence.candidate_view, Some(CandidateViewKind::Normalized));
    assert_eq!(evidence.normalized_start, Some(5));
    assert_eq!(evidence.normalized_end, Some(13));
    assert_eq!(evidence.raw_start, Some(5));
    assert_eq!(evidence.raw_end, Some(14));
}

#[test]
fn excluded_evidence_copies_evasion_candidate_provenance() {
    let detector = detector(vec![entry("EN", "love", "asm", false)]);
    let result = detector.analyze_rules("l o v e", RuleContext::for_language("EN"));
    let evidence = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::LexicalCollisionExcluded)
        .expect("collision evidence");

    assert_eq!(result.scores.profanity, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert_eq!(evidence.points, 0);
    assert_eq!(evidence.candidate_view, Some(CandidateViewKind::Evasion));
    assert_eq!(evidence.normalized_start, Some(0));
    assert_eq!(evidence.normalized_end, Some(4));
    assert_eq!(evidence.raw_start, Some(0));
    assert_eq!(evidence.raw_end, Some(7));
}

#[test]
fn rule_only_evidence_has_no_candidate_provenance() {
    let result =
        english_detector().analyze_rules("I will kill you", RuleContext::for_language("EN"));
    let evidence = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::DirectThreat)
        .expect("threat evidence");

    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
    assert_eq!(evidence.candidate_view, None);
    assert_eq!(evidence.normalized_start, None);
    assert_eq!(evidence.normalized_end, None);
    assert_eq!(evidence.raw_start, Some(7));
    assert_eq!(evidence.raw_end, Some(11));
}

#[test]
fn lexical_derived_context_evidence_uses_the_same_candidate() {
    let detector = detector(vec![entry("EN", "idiot", "cds", false)]);
    let result = detector.analyze_rules(
        "i.d.i.o.t",
        RuleContext {
            language: Some("EN"),
            reply_target: ReplyTarget::Person,
        },
    );

    assert_eq!(result.scores.profanity, 35);
    assert_eq!(result.scores.targeted_abuse, 70);
    for rule_id in [RuleId::LexicalMatch, RuleId::ReplyTargetedLexicalMatch] {
        let evidence = result
            .evidence
            .iter()
            .find(|item| item.rule_id == rule_id)
            .expect("lexical-derived evidence");
        assert_eq!(evidence.candidate_view, Some(CandidateViewKind::Evasion));
        assert_eq!(evidence.normalized_start, Some(0));
        assert_eq!(evidence.normalized_end, Some(5));
        assert_eq!(evidence.raw_start, Some(0));
        assert_eq!(evidence.raw_end, Some(9));
    }
}

#[test]
fn caps_duplicate_suppressed_rows_as_one_lexical_event() {
    let detector = detector(vec![
        entry("EN", "idiot", "cds", false),
        entry("EN", "idiot", "cds", false),
    ]);
    let result = detector.analyze_rules("You are not an idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(
        result
            .evidence
            .iter()
            .filter(|item| item.rule_id == RuleId::NegatedEvidence)
            .count(),
        1
    );
}

#[test]
fn caps_multiple_suppressed_spans_at_ten_total() {
    let detector = detector(vec![
        entry("EN", "morons", "cds", false),
        entry("EN", "idiots", "ddp", true),
    ]);
    let result = detector.analyze_rules(
        "Do not call morons idiots.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn exact_do_not_counterspeech_keeps_only_the_suppressed_insult_score() {
    let detector = detector(vec![
        entry("EN", "do", "asm", false),
        entry("EN", "people", "cds", false),
        entry("EN", "idiots", "ddp", true),
    ]);
    let result = detector.analyze_rules(
        "Do not call people idiots.",
        RuleContext::for_language("EN"),
    );

    assert!(result.lexical.is_match());
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::NegatedEvidence));
    assert_eq!(
        result
            .evidence
            .iter()
            .filter(|item| item.rule_id == RuleId::LexicalCollisionExcluded)
            .count(),
        2
    );
}

#[test]
fn exact_contracted_counterspeech_keeps_only_the_suppressed_insult_score() {
    let detector = detector(vec![
        entry("EN", "people", "cds", false),
        entry("EN", "idiots", "ddp", true),
    ]);
    let result =
        detector.analyze_rules("Don't call people idiots.", RuleContext::for_language("EN"));

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::NegatedEvidence));
}

#[test]
fn exact_stop_counterspeech_keeps_only_the_suppressed_insult_score() {
    let detector = detector(vec![
        entry("EN", "people", "cds", false),
        entry("EN", "idiots", "ddp", true),
    ]);
    let result = detector.analyze_rules(
        "Stop calling people idiots.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::CounterspeechEvidence));
}

#[test]
fn preserves_a_later_active_insult_outside_the_negation_window() {
    let detector = detector(vec![
        entry("EN", "do", "asm", false),
        entry("EN", "people", "cds", false),
        entry("EN", "idiots", "ddp", true),
        entry("EN", "idiot", "cds", false),
    ]);
    let result = detector.analyze_rules(
        "Do not call people idiots, you idiot.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.lexical.matches.len(), 4);
    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 70);
    assert_eq!(result.action, PolicyAction::Review);
}

fn detector(entries: Vec<LexiconEntry>) -> Detector {
    Detector::new(entries).expect("valid detector")
}

fn english_detector() -> Detector {
    detector(vec![entry("EN", "idiot", "cds", false)])
}

fn spanish_detector() -> Detector {
    detector(vec![
        entry("ES", "estúpido", "cds", false),
        entry("ES", "idiota", "cds", false),
        entry("ES", "imbéciles", "cds", false),
        entry("ES", "basura", "cds", false),
    ])
}

#[test]
fn spanish_hostile_wishes_score_threat_language() {
    let detector = spanish_detector();

    for text in [
        "Ojalá te mueras.",
        "Ojala se muera toda tu familia.",
        "Espero que tu hijo sufra.",
        "Que se mueran todos ustedes.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert!(result.scores.threat_language >= 85, "{text}");
        assert_eq!(result.action, PolicyAction::Block, "{text}");
    }
}

#[test]
fn spanish_semantic_threat_variants_do_not_accumulate() {
    let result = spanish_detector().analyze_rules(
        "Ojalá te mueras. Espero que te mueras.",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.scores.threat_language, 85);
}

#[test]
fn spanish_self_harm_commands_score_threat_language() {
    let detector = spanish_detector();

    for text in ["Mátate de una vez.", "Suicídate.", "Tírate por un puente."] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert!(result.scores.threat_language >= 85, "{text}");
        assert_eq!(result.action, PolicyAction::Block, "{text}");
    }
}

#[test]
fn spanish_implicit_second_person_insults_are_targeted() {
    let detector = spanish_detector();

    for text in [
        "Eres un idiota.",
        "Eres estúpido.",
        "Sois unos imbéciles.",
        "Estás hecho una basura.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert_eq!(result.scores.targeted_abuse, 70, "{text}");
    }
}

#[test]
fn spanish_directed_hostility_uses_encoded_targets() {
    let detector = spanish_detector();

    for text in [
        "Me das asco.",
        "No te soporto.",
        "Cállate y escucha.",
        "Vete a la mierda.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert_eq!(result.scores.targeted_abuse, 60, "{text}");
        assert!(result.nudge().should_nudge, "{text}");
    }
}

#[test]
fn spanish_directed_hostility_respects_quotes() {
    let result = spanish_detector().analyze_rules(
        "Ana escribió «no te soporto» en el informe.",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.scores.targeted_abuse, 0);
    assert!(!result.nudge().should_nudge);
    assert!(result.has_rule(RuleId::QuotedEvidence));
}

#[test]
fn spanish_inflected_harm_intents_score_threat_language() {
    let detector = spanish_detector();

    for text in [
        "Te voy a violar.",
        "Te voy a manosear aunque no quieras.",
        "Cuando te vea, te rompo la cara.",
        "Mañana te pego un tiro.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert_eq!(result.scores.threat_language, 95, "{text}");
        assert!(result.nudge().should_nudge, "{text}");
    }
}

#[test]
fn spanish_future_assault_and_desired_death_score_threat_language() {
    let detector = spanish_detector();

    let assault =
        detector.analyze_rules("Te partiré los dientes.", RuleContext::for_language("ES"));
    let desired_death =
        detector.analyze_rules("Tu familia debería morir.", RuleContext::for_language("ES"));

    assert_eq!(assault.scores.threat_language, 95);
    assert_eq!(desired_death.scores.threat_language, 85);
}

#[test]
fn spanish_negation_suppresses_desired_death() {
    let result = spanish_detector().analyze_rules(
        "Tu familia no debería morir.",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.scores.threat_language, 0);
    assert!(!result.nudge().should_nudge);
    assert!(result.has_rule(RuleId::NegatedEvidence));
}

#[test]
fn spanish_negation_suppresses_the_complete_intent_frame() {
    let result =
        spanish_detector().analyze_rules("No te voy a matar.", RuleContext::for_language("ES"));

    assert_eq!(result.scores.threat_language, 0);
    assert!(!result.nudge().should_nudge);
    assert!(result.has_rule(RuleId::NegatedEvidence));
}

#[test]
fn spanish_laughter_idiom_does_not_create_a_threat() {
    let result = spanish_detector().analyze_rules(
        "Te voy a matar de risa con este vídeo.",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.scores.threat_language, 0);
    assert!(!result.nudge().should_nudge);
}

#[test]
fn spanish_guillemets_suppress_a_self_harm_quote() {
    let result = spanish_detector().analyze_rules(
        "La línea de ayuda recomienda no decir «suicídate».",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.scores.threat_language, 0);
    assert!(!result.nudge().should_nudge);
    assert!(result.has_rule(RuleId::QuotedEvidence));
}

#[test]
fn scores_a_direct_insult_as_targeted_abuse() {
    let result =
        english_detector().analyze_rules("You are an idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 70);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(result.has_rule(RuleId::TargetedLexicalMatch));
    assert_eq!(result.max_risk_points(), 70);
}

#[test]
fn untargeted_lexical_matches_do_not_accumulate_into_a_nudge() {
    let detector = detector(vec![
        entry("EN", "idiot", "cds", false),
        entry("EN", "buffoon", "cds", false),
    ]);
    let result = detector.analyze_rules(
        "An idiot and a buffoon appear in the story.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 30);
    assert!(!result.nudge().should_nudge);
}

#[test]
fn nudge_result_uses_the_maximum_rule_score_and_fixed_threshold() {
    let result =
        english_detector().analyze_rules("I will kill you.", RuleContext::for_language("EN"));

    let nudge = result.nudge();

    assert_eq!(nudge.score, 95);
    assert_eq!(nudge.threshold, 50);
    assert!(nudge.should_nudge);
}

#[test]
fn nudge_boolean_is_derived_from_the_score_threshold() {
    let result = english_detector().analyze_rules("Idiot.", RuleContext::for_language("EN"));

    let nudge = result.nudge();

    assert_eq!(nudge.score, 30);
    assert_eq!(nudge.threshold, 50);
    assert_eq!(nudge.should_nudge, nudge.score >= nudge.threshold);
    assert!(!nudge.should_nudge);
}

#[test]
fn spanish_rule_diagnostics_do_not_load_a_sparse_model() {
    let result = spanish_detector().analyze_rules(
        "La gente del pueblo habló con la policía.",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.sparse_score, None);
    assert_eq!(result.nudge().score, result.max_risk_points());
    assert!(!result.nudge().should_nudge);
}

#[test]
fn english_rule_diagnostics_do_not_load_a_sparse_model() {
    let result = english_detector().analyze_rules("Thank you.", RuleContext::for_language("EN"));

    assert_eq!(result.sparse_score, None);
}

#[test]
fn uses_the_reply_target_for_a_bare_insult() {
    let result = english_detector().analyze_rules(
        "Idiot.",
        RuleContext {
            language: Some("EN"),
            reply_target: ReplyTarget::Person,
        },
    );

    assert_eq!(result.scores.targeted_abuse, 70);
}

#[test]
fn blocks_a_direct_threat_without_a_lexical_match() {
    let result =
        english_detector().analyze_rules("I will kill you.", RuleContext::for_language("EN"));

    assert!(!result.lexical.is_match());
    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(result.has_rule(RuleId::DirectThreat));
}

#[test]
fn records_a_nearby_threat_intent_marker_without_extra_points() {
    let result =
        english_detector().analyze_rules("I will kill you.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.threat_language, 95);
    let intent = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::ThreatIntentMarker)
        .expect("intent evidence");
    assert_eq!(intent.points, 0);
    assert_eq!(intent.matched_text, "will");
}

#[test]
fn detects_a_threat_without_an_intent_marker() {
    let result = english_detector().analyze_rules("I kill you.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(!result.has_rule(RuleId::ThreatIntentMarker));
}

#[test]
fn detects_kill_yourself_without_an_intent_marker() {
    let result =
        english_detector().analyze_rules("Kill yourself.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(!result.has_rule(RuleId::ThreatIntentMarker));
}

#[test]
fn a_technical_kill_request_does_not_form_a_direct_threat() {
    let result = english_detector().analyze_rules(
        "Can you kill this process?",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.threat_language, 0);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn an_english_laughter_idiom_does_not_form_a_direct_threat() {
    let result = english_detector().analyze_rules(
        "This comedy will kill you with laughter.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.threat_language, 0);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn an_unrelated_negator_before_a_threat_does_not_suppress_the_threat() {
    let result =
        english_detector().analyze_rules("No, I will kill you.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
}

#[test]
fn an_unrelated_report_before_a_threat_does_not_suppress_the_threat() {
    for text in [
        "She reported the bug, then I will kill you.",
        "I reported the bug and will kill you.",
    ] {
        let result = english_detector().analyze_rules(text, RuleContext::for_language("EN"));

        assert_eq!(result.scores.threat_language, 95, "{text}");
        assert_eq!(result.action, PolicyAction::Block, "{text}");
    }
}

#[test]
fn compact_rules_scope_reports_and_quotes_to_the_matched_clause() {
    let detector = Detector::rules_only();
    for (language, text) in [
        ("ZH", "管理员报告了消息。我要杀了你"),
        ("ZH", "我要杀了你。\u{201c}你好\u{201d}"),
        ("ZH", "管理员报告了消息，我要杀了你"),
        ("ZH", "我要杀了你，今天说\u{201c}你好\u{201d}"),
        ("JA", "管理者が投稿を報告した。お前を殺してやる"),
        ("JA", "お前を殺してやる。\u{300c}こんにちは\u{300d}"),
        ("JA", "管理者が投稿を報告した、お前を殺してやる"),
        (
            "JA",
            "お前を殺してやる、今日は\u{300c}こんにちは\u{300d}と言った",
        ),
        ("KO", "관리자가 메시지를 신고했다.너를 죽이겠다"),
        ("KO", "너를 죽이겠다.\u{201c}안녕\u{201d}"),
        ("KO", "관리자가 메시지를 신고했다,너를 죽이겠다"),
        ("KO", "너를 죽이겠다,오늘 \u{201c}안녕\u{201d}이라고 말했다"),
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language(language));

        assert_eq!(result.scores.threat_language, 95, "{language}: {text}");
        assert_eq!(result.action, PolicyAction::Block, "{language}: {text}");
        let evidence = result
            .evidence
            .iter()
            .find(|item| item.rule_id == RuleId::DirectThreat)
            .expect("direct threat evidence");
        assert!(!evidence.matched_text.contains(','), "{language}: {text}");
        assert!(!evidence.matched_text.contains('，'), "{language}: {text}");
    }
}

#[test]
fn suppresses_a_negated_insult_but_keeps_the_lexical_match() {
    let result =
        english_detector().analyze_rules("You are not an idiot.", RuleContext::for_language("EN"));

    assert!(result.lexical.is_match());
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::NegatedEvidence));
}

#[test]
fn suppresses_an_insult_after_a_multi_token_normalized_negator() {
    let result =
        english_detector().analyze_rules("You don't idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::NegatedEvidence));
}

#[test]
fn suppresses_a_quoted_insult_but_keeps_the_lexical_match() {
    let result = english_detector().analyze_rules(
        "She wrote “you are an idiot”.",
        RuleContext::for_language("EN"),
    );

    assert!(result.lexical.is_match());
    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::QuotedEvidence));
}

#[test]
fn suppresses_a_reported_insult() {
    let result = english_detector()
        .analyze_rules("She called me an idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::ReportedEvidence));
}

#[test]
fn suppresses_a_counterspeech_identity_attack() {
    let detector = detector(vec![entry("EN", "vermin", "ps", true)]);
    let result = detector.analyze_rules(
        "Do not call immigrants vermin.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 10);
    assert_eq!(result.scores.identity_attack, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::NegatedEvidence));
}

#[test]
fn blocks_an_identity_attack_supported_by_a_stereotype_entry() {
    let detector = detector(vec![entry("EN", "vermin", "ps", true)]);
    let result = detector.analyze_rules("Immigrants are vermin.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(result.has_rule(RuleId::IdentityGroupTarget));
    let stereotype = result
        .evidence
        .iter()
        .find(|item| item.rule_id == RuleId::IdentityStereotypeSupport)
        .expect("stereotype support evidence");
    assert_eq!(stereotype.points, 0);
    assert_eq!(stereotype.matched_text, "vermin");
}

#[test]
fn a_direct_group_predicate_uses_a_general_abuse_entry() {
    let detector = detector(vec![entry("ES", "rata", "an", false)]);
    let result = detector.analyze_rules(
        "Todos los judíos son ratas.",
        RuleContext::for_language("ES"),
    );

    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);
}

#[test]
fn spanish_group_hostility_uses_semantic_predicates() {
    let detector = spanish_detector();

    for text in [
        "Los inmigrantes son una plaga.",
        "Las mujeres son inferiores.",
        "Los gais dan asco.",
        "Los inmigrantes sobran aquí.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert_eq!(result.scores.identity_attack, 85, "{text}");
        assert_eq!(result.action, PolicyAction::Block, "{text}");
    }
}

#[test]
fn spanish_semantic_group_hostility_respects_negation_and_quotes() {
    let detector = spanish_detector();

    for text in [
        "Los inmigrantes no son una plaga.",
        "El informe recoge «los inmigrantes son una plaga».",
        "Es falso decir que las mujeres son inferiores.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("ES"));

        assert_eq!(result.scores.identity_attack, 0, "{text}");
        assert!(!result.nudge().should_nudge, "{text}");
    }
}

#[test]
fn a_direct_group_predicate_does_not_require_an_identity_category() {
    let detector = detector(vec![entry("EN", "vermin", "cds", true)]);
    let result = detector.analyze_rules("Immigrants are vermin.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(result.has_rule(RuleId::IdentityGroupTarget));
    assert!(result.has_rule(RuleId::IdentityStereotypeSupport));
}

#[test]
fn a_neutral_group_list_does_not_form_an_identity_attack() {
    let result = local_lexicon_detector().analyze_rules(
        "The index lists immigrants, idiots, and verbs.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.identity_attack, 0);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(!result.has_rule(RuleId::IdentityGroupTarget));
}

#[test]
fn punctuation_support_does_not_open_the_identity_gate() {
    let result = local_lexicon_detector().analyze_rules(
        "The index lists immigrants, idiots, and verbs!!!",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.sentiment_support, 1);
    assert_eq!(result.scores.identity_attack, 0);
    assert_eq!(result.action, PolicyAction::Review);
}

#[test]
fn negative_lexical_sentiment_opens_the_identity_gate_without_a_direct_relation() {
    let detector = detector(vec![entry("EN", "vermin", "ps", true)]);
    let result = detector.analyze_rules(
        "Immigrants, disgusting vermin.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.sentiment_support, 1);
    assert_eq!(result.scores.identity_attack, 86);
    assert_eq!(result.action, PolicyAction::Block);
}

#[test]
fn every_language_pack_distinguishes_a_list_from_a_direct_identity_relation() {
    let fixtures = [
        (
            "EN",
            "vermin",
            "The index lists immigrants, vermin, and verbs.",
            "Immigrants are vermin.",
        ),
        (
            "ES",
            "alimañas",
            "El índice enumera inmigrantes, alimañas y verbos.",
            "Inmigrantes son alimañas.",
        ),
        (
            "FR",
            "vermine",
            "L'index énumère immigrés, vermine et verbes.",
            "Les immigrés sont de la vermine.",
        ),
        (
            "DE",
            "Ungeziefer",
            "Der Index nennt Immigranten, Ungeziefer und Verben.",
            "Immigranten sind Ungeziefer.",
        ),
        (
            "IT",
            "parassiti",
            "L'indice elenca immigrati, parassiti e verbi.",
            "Immigrati sono parassiti.",
        ),
        (
            "PT",
            "parasitas",
            "O índice lista imigrantes, parasitas e verbos.",
            "Imigrantes são parasitas.",
        ),
        (
            "RU",
            "паразиты",
            "В списке: иммигранты, паразиты и глаголы.",
            "Иммигранты это паразиты.",
        ),
        (
            "AR",
            "حشرات",
            "القائمة: مهاجرون، حشرات، وأفعال.",
            "مهاجرون هم حشرات.",
        ),
    ];

    for (language, lemma, neutral_text, hostile_text) in fixtures {
        let detector = detector(vec![entry(language, lemma, "ps", true)]);
        let neutral = detector.analyze_rules(neutral_text, RuleContext::for_language(language));
        let hostile = detector.analyze_rules(hostile_text, RuleContext::for_language(language));

        assert_eq!(neutral.scores.identity_attack, 0, "{language}: {neutral:?}");
        assert_eq!(neutral.action, PolicyAction::Review, "{language}");
        assert_eq!(
            hostile.scores.identity_attack, 85,
            "{language}: {hostile:?}"
        );
        assert_eq!(hostile.action, PolicyAction::Block, "{language}");
    }
}

#[test]
fn whitespace_only_adjacency_supports_zero_copula_identity_syntax() {
    let fixtures = [
        ("RU", "паразиты", "Иммигранты паразиты."),
        ("AR", "حشرات", "مهاجرون حشرات."),
    ];

    for (language, lemma, text) in fixtures {
        let detector = detector(vec![entry(language, lemma, "ps", true)]);
        let result = detector.analyze_rules(text, RuleContext::for_language(language));

        assert_eq!(result.scores.identity_attack, 85, "{language}: {result:?}");
        assert_eq!(result.action, PolicyAction::Block, "{language}");
    }
}

#[test]
fn whitespace_only_adjacency_does_not_enable_zero_copula_in_other_language_packs() {
    let fixtures = [
        ("EN", "vermin", "Immigrants vermin."),
        ("ES", "alimañas", "Inmigrantes alimañas."),
        ("FR", "vermine", "Immigrés vermine."),
        ("DE", "Ungeziefer", "Immigranten Ungeziefer."),
        ("IT", "parassiti", "Immigrati parassiti."),
        ("PT", "parasitas", "Imigrantes parasitas."),
    ];

    for (language, lemma, text) in fixtures {
        let detector = detector(vec![entry(language, lemma, "ps", true)]);
        let result = detector.analyze_rules(text, RuleContext::for_language(language));

        assert_eq!(result.scores.identity_attack, 0, "{language}: {result:?}");
        assert_eq!(result.action, PolicyAction::Review, "{language}");
    }
}

#[test]
fn a_protected_group_reply_supplies_a_direct_identity_target() {
    let detector = detector(vec![entry("EN", "vermin", "ps", true)]);
    let result = detector.analyze_rules(
        "Vermin.",
        RuleContext {
            language: Some("EN"),
            reply_target: ReplyTarget::ProtectedGroup,
        },
    );

    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);
}

#[test]
fn records_sentiment_support_without_selecting_an_action() {
    let result = english_detector()
        .analyze_rules("This service is awful!!!", RuleContext::for_language("EN"));

    assert_eq!(result.scores.sentiment_support, 2);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::NegativeSentiment));
    assert!(result.has_rule(RuleId::PunctuationSupport));
}

#[test]
fn adds_sentiment_only_to_a_context_category_in_the_same_clause() {
    let result = english_detector()
        .analyze_rules("You idiot. This is awful.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.sentiment_support, 1);
    assert_eq!(result.scores.targeted_abuse, 70);
}

#[test]
fn positive_sentiment_does_not_cancel_another_clauses_negative_sentiment() {
    let result = english_detector().analyze_rules("Awful. Good.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.sentiment_support, 1);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn applies_a_multi_token_normalized_diminisher_to_sentiment() {
    let result = english_detector()
        .analyze_rules("C'est peut-être horrible.", RuleContext::for_language("FR"));

    assert_eq!(result.scores.sentiment_support, 0);
    assert_eq!(result.action, PolicyAction::Allow);
}

#[test]
fn does_not_target_an_insult_across_clauses() {
    let result =
        english_detector().analyze_rules("You are kind. Idiot.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.targeted_abuse, 0);
}

#[test]
fn absent_language_uses_lexical_only_mode() {
    let result = english_detector().analyze_rules(
        "You are awful. I will kill you, idiot.",
        RuleContext {
            language: None,
            reply_target: ReplyTarget::ProtectedGroup,
        },
    );

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(result.scores.identity_attack, 0);
    assert_eq!(result.scores.threat_language, 0);
    assert_eq!(result.scores.sentiment_support, 0);
}

#[test]
fn excludes_all_verified_language_lemma_collisions_but_keeps_raw_matches() {
    let fixtures: [(&str, &str, &[&str]); 8] = [
        (
            "EN",
            "People do know that love takes time.",
            &["people", "do", "know", "love"],
        ),
        (
            "DE",
            "Das Volk hört Rock. Der Hund sitzt neben der Dose.",
            &["volk", "rock", "hund", "dose"],
        ),
        (
            "ES",
            concat!(
                "La gente del pueblo habló con la policía. ",
                "La agente revisó la bomba. El gato es curioso. ",
                "Mi vecino es gay y gitano. ",
                "Pinté el muro negro. El tren está retrasado. El servicio funciona."
            ),
            &[
                "gente",
                "pueblo",
                "policía",
                "agente",
                "curioso",
                "bomba",
                "gay",
                "gitano",
                "negro",
                "retrasado",
                "servicio",
            ],
        ),
        (
            "FR",
            "Les gens du peuple parlent avec la police.",
            &["gens", "peuple", "police"],
        ),
        (
            "IT",
            "La gente del popolo visita il sito di arte.",
            &["gente", "popolo", "sito", "arte"],
        ),
        (
            "PT",
            "A gente do povo falou com a polícia.",
            &["gente", "povo", "polícia"],
        ),
        (
            "RU",
            "Эта девушка есть член группы. Ее юбка синяя.",
            &["девушка", "член", "юбка"],
        ),
        (
            "AR",
            "هذه فتاة تقابل عامل وشرطي وضابط في تجمع.",
            &["فتاة", "عامل", "تجمع"],
        ),
    ];

    for (language, text, lemmas) in fixtures {
        let detector = detector(
            lemmas
                .iter()
                .map(|lemma| entry(language, lemma, "cds", false))
                .collect(),
        );
        let result = detector.analyze_rules(text, RuleContext::for_language(language));

        assert_eq!(
            result.lexical.matches.len(),
            lemmas.len(),
            "{language}: {text}"
        );
        assert_eq!(result.scores.profanity, 0, "{language}: {text}");
        assert_eq!(result.action, PolicyAction::Allow, "{language}: {text}");
        let exclusions = result
            .evidence
            .iter()
            .filter(|item| item.rule_id == RuleId::LexicalCollisionExcluded)
            .collect::<Vec<_>>();
        assert_eq!(exclusions.len(), lemmas.len(), "{language}: {text}");
        assert!(exclusions.iter().all(|item| item.points == 0));
        assert!(
            exclusions
                .iter()
                .all(|item| item.language.as_deref() == Some(language))
        );
    }
}

#[test]
fn synthetic_common_english_collisions_stay_allow() {
    let detector = detector(vec![
        entry("EN", "love", "asm", false),
        entry("EN", "people", "cds", false),
        entry("EN", "proud", "svp", false),
    ]);
    let result = detector.analyze_rules(
        "People love their proud community.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.lexical.matches.len(), 3);
    assert_eq!(result.scores.profanity, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert_eq!(
        result
            .evidence
            .iter()
            .filter(|item| item.rule_id == RuleId::LexicalCollisionExcluded)
            .count(),
        3
    );
}

#[test]
fn reactivates_the_audited_german_direct_abuse_phrase() {
    let detector = detector(vec![entry("DE", "hund", "an", false)]);
    let result = detector.analyze_rules("Du Hund.", RuleContext::for_language("DE"));

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 70);
    assert_eq!(result.scores.identity_attack, 0);
    assert_eq!(result.scores.threat_language, 0);
    assert_eq!(result.scores.sentiment_support, 0);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(result.has_rule(RuleId::LexicalMatch));
    assert!(result.has_rule(RuleId::TargetedLexicalMatch));
    assert!(!result.has_rule(RuleId::LexicalCollisionExcluded));
}

#[test]
fn keeps_neutral_german_hund_contexts_excluded() {
    let detector = detector(vec![entry("DE", "hund", "an", false)]);

    for text in [
        "Der Hund schläft.",
        "Ihr Hund schläft.",
        "Du hast einen Hund.",
    ] {
        let result = detector.analyze_rules(text, RuleContext::for_language("DE"));

        assert_eq!(result.scores.profanity, 0, "{text}");
        assert_eq!(result.scores.targeted_abuse, 0, "{text}");
        assert_eq!(result.action, PolicyAction::Allow, "{text}");
        assert!(result.has_rule(RuleId::LexicalCollisionExcluded), "{text}");
    }
}

#[test]
fn auto_mode_does_not_reactivate_the_german_direct_abuse_phrase() {
    let detector = detector(vec![entry("DE", "hund", "an", false)]);
    let result = detector.analyze_rules(
        "Du Hund.",
        RuleContext {
            language: None,
            reply_target: ReplyTarget::Unknown,
        },
    );

    assert_eq!(result.scores.profanity, 0);
    assert_eq!(result.scores.targeted_abuse, 0);
    assert_eq!(result.action, PolicyAction::Allow);
    assert!(result.has_rule(RuleId::LexicalCollisionExcluded));
}

#[test]
fn excludes_italian_gente_in_explicit_and_auto_inclusive_modes() {
    let detector = detector(vec![entry_with_level(
        "IT",
        "gente",
        "cds",
        false,
        MatchLevel::Inclusive,
    )]);

    for context in [
        RuleContext::for_language("IT"),
        RuleContext {
            language: None,
            reply_target: ReplyTarget::Unknown,
        },
    ] {
        let result = detector.analyze_rules("La gente aspetta.", context);

        assert_eq!(result.lexical.matches.len(), 1);
        assert!((result.lexical.score - 0.6).abs() < f64::EPSILON);
        assert_eq!(result.scores.profanity, 0);
        assert_eq!(result.action, PolicyAction::Allow);
        assert!(result.has_rule(RuleId::LexicalCollisionExcluded));
    }
}

#[test]
fn local_lexicon_keeps_explicit_profanity_active_in_all_language_packs() {
    let fixtures = [
        ("EN", "fuck"),
        ("DE", "arschloch"),
        ("ES", "puta"),
        ("FR", "pute"),
        ("IT", "stronzo"),
        ("PT", "puta"),
        ("RU", "сука"),
        ("AR", "كس"),
    ];

    for (language, text) in fixtures {
        let result =
            local_lexicon_detector().analyze_rules(text, RuleContext::for_language(language));

        assert!(result.lexical.is_match(), "{language}: {text}");
        assert!(result.scores.profanity >= 20, "{language}: {text}");
        assert_eq!(result.action, PolicyAction::Review, "{language}: {text}");
    }
}

#[test]
fn an_excluded_lexical_event_does_not_disable_a_direct_threat() {
    let detector = detector(vec![entry("EN", "people", "cds", false)]);
    let result = detector.analyze_rules("I will kill you people.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 0);
    assert_eq!(result.scores.threat_language, 95);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(result.has_rule(RuleId::DirectThreat));
}

#[test]
fn an_excluded_mention_cue_still_targets_another_lexical_event() {
    let detector = detector(vec![
        entry("EN", "people", "cds", false),
        entry("EN", "idiots", "cds", false),
    ]);
    let result = detector.analyze_rules("@people idiots.", RuleContext::for_language("EN"));

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.targeted_abuse, 70);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(result.has_rule(RuleId::TargetedLexicalMatch));
}

#[test]
fn auto_mode_excludes_known_collisions_and_keeps_other_lexical_matches() {
    let detector = detector(vec![
        entry("EN", "love", "asm", false),
        entry("EN", "fuck", "cds", false),
        entry("FR", "police", "cds", false),
    ]);
    let neutral = detector.analyze_rules(
        "Love takes time.",
        RuleContext {
            language: None,
            reply_target: ReplyTarget::Unknown,
        },
    );
    let profanity = detector.analyze_rules(
        "Fuck.",
        RuleContext {
            language: None,
            reply_target: ReplyTarget::Unknown,
        },
    );
    let non_english_neutral = detector.analyze_rules(
        "Police.",
        RuleContext {
            language: None,
            reply_target: ReplyTarget::Unknown,
        },
    );

    assert!(neutral.lexical.is_match());
    assert_eq!(neutral.scores.profanity, 0);
    assert_eq!(neutral.action, PolicyAction::Allow);
    assert!(profanity.lexical.is_match());
    assert_eq!(profanity.scores.profanity, 30);
    assert_eq!(profanity.action, PolicyAction::Review);
    assert!(non_english_neutral.lexical.is_match());
    assert_eq!(non_english_neutral.scores.profanity, 0);
    assert_eq!(non_english_neutral.action, PolicyAction::Allow);
    assert!(non_english_neutral.has_rule(RuleId::LexicalCollisionExcluded));
}

#[test]
fn an_explicit_pack_does_not_apply_another_packs_collision_exclusion() {
    let detector = detector(vec![entry("FR", "police", "cds", false)]);
    let result = detector.analyze_rules("Police.", RuleContext::for_language("EN"));

    assert!(result.lexical.is_match());
    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(!result.has_rule(RuleId::LexicalCollisionExcluded));
}

#[test]
fn an_unsupported_explicit_language_does_not_enter_auto_collision_mode() {
    let detector = detector(vec![entry("FR", "police", "cds", false)]);
    let result = detector.analyze_rules("Police.", RuleContext::for_language("JA"));

    assert!(result.lexical.is_match());
    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(!result.has_rule(RuleId::LexicalCollisionExcluded));
}

#[test]
fn duplicate_excluded_rows_emit_one_zero_point_event() {
    let detector = detector(vec![
        entry("EN", "love", "asm", false),
        entry("EN", "love", "svp", false),
    ]);
    let result = detector.analyze_rules("Love.", RuleContext::for_language("EN"));

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 0);
    let exclusions = result
        .evidence
        .iter()
        .filter(|item| item.rule_id == RuleId::LexicalCollisionExcluded)
        .collect::<Vec<_>>();
    assert_eq!(exclusions.len(), 1);
    assert_eq!(exclusions[0].points, 0);
    assert_eq!(exclusions[0].matched_text, "Love");
    assert_eq!(exclusions[0].raw_start, Some(0));
    assert_eq!(exclusions[0].raw_end, Some(4));
}

#[test]
fn a_same_span_active_entry_survives_an_excluded_entry() {
    let detector = detector(vec![
        entry("EN", "love", "asm", false),
        entry("FR", "love", "cds", false),
    ]);
    let result = detector.analyze_rules("Love.", RuleContext::for_language("EN"));

    assert_eq!(result.lexical.matches.len(), 2);
    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.action, PolicyAction::Review);
    assert!(result.has_rule(RuleId::LexicalCollisionExcluded));
    assert!(result.has_rule(RuleId::LexicalMatch));
}

#[test]
fn an_excluded_event_does_not_remove_a_group_cue_for_another_event() {
    let detector = detector(vec![
        entry("EN", "people", "cds", false),
        entry("EN", "vermin", "ps", true),
    ]);
    let result = detector.analyze_rules(
        "Immigrants are vermin near people.",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.profanity, 30);
    assert_eq!(result.scores.identity_attack, 85);
    assert_eq!(result.action, PolicyAction::Block);
    assert!(result.has_rule(RuleId::IdentityGroupTarget));
}

#[test]
fn detects_threats_in_each_language_pack() {
    let fixtures = [
        ("EN", "I will kill you"),
        ("ES", "Te voy a matar"),
        ("FR", "Je vais te tuer"),
        ("DE", "Ich werde dich töten"),
        ("IT", "Ti ucciderò"),
        ("PT", "Eu vou matar você"),
        ("RU", "Я убью тебя"),
        ("AR", "سأقتلك أنت"),
    ];
    let detector = english_detector();

    for (language, text) in fixtures {
        let result = detector.analyze_rules(text, RuleContext::for_language(language));
        assert!(
            result.scores.threat_language >= 95,
            "{language}: {text}: {result:?}"
        );
        assert_eq!(result.action, PolicyAction::Block, "{language}: {text}");
    }
}

#[test]
fn caps_sentiment_support_added_to_active_categories_at_five() {
    let result = english_detector().analyze_rules(
        "REALLY SO EXTREMELY AWFUL HORRIBLE DISGUSTING HATE EVIL YOU IDIOT!!!",
        RuleContext::for_language("EN"),
    );

    assert_eq!(result.scores.sentiment_support, 8);
    assert_eq!(result.scores.targeted_abuse, 75);
}

#[test]
fn displays_public_policy_values_as_lowercase_snake_case() {
    assert_eq!(PolicyAction::Block.to_string(), "block");
    assert_eq!(
        blasphem::PolicyCategory::IdentityAttack.to_string(),
        "identity_attack"
    );
    assert_eq!(
        RuleId::CounterspeechEvidence.to_string(),
        "counterspeech_evidence"
    );
    assert_eq!(
        RuleId::LexicalCollisionExcluded.to_string(),
        "lexical_collision_excluded"
    );
}

fn local_lexicon_detector() -> &'static Detector {
    static DETECTOR: OnceLock<Detector> = OnceLock::new();
    DETECTOR.get_or_init(|| {
        let mut entries = Vec::new();
        for language in ["AR", "DE", "EN", "ES", "FR", "IT", "PT", "RU"] {
            let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
                .join("lexicon")
                .join(format!("{language}.tsv"));
            let file = File::open(&path).unwrap_or_else(|error| {
                panic!("cannot open local Lexicon file {}: {error}", path.display())
            });
            let parsed = parse_lexicon(file, language).unwrap_or_else(|error| {
                panic!(
                    "cannot parse local Lexicon file {}: {error}",
                    path.display()
                )
            });
            entries.extend(
                parsed
                    .into_iter()
                    .filter(|entry| entry.level == MatchLevel::Conservative),
            );
        }
        Detector::new(entries).expect("local conservative Lexicon detector")
    })
}
