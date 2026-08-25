use std::fs;
use std::path::{Path, PathBuf};

use blasphem::{
    CandidateViewKind, EvalLabel, Language, ReplyTarget, RuleId, TextDocument, analyze_with_rules,
    arabic_hindi_rules, cjk_rules, word_rules,
};
use blasphem_train::{
    BehaviorPanelError, ControlKind, EventType, load_panel, validate_event_distribution,
};
use tempfile::TempDir;

const WORD_LANGUAGES: [Language; 9] = [
    Language::En,
    Language::Ms,
    Language::Pt,
    Language::Fr,
    Language::Ru,
    Language::De,
    Language::Tr,
    Language::Vi,
    Language::It,
];

const ARABIC_HINDI_LANGUAGES: [Language; 2] = [Language::Ar, Language::Hi];

const CJK_LANGUAGES: [Language; 3] = [Language::Zh, Language::Ja, Language::Ko];

#[test]
fn word_languages_match_their_24_row_behavior_contracts() {
    let root = behavior_fixture_root();
    let mut mismatches = Vec::new();

    for language in WORD_LANGUAGES {
        let panel = load_panel(&root, language).expect("load behavior panel");
        assert_eq!(panel.len(), 24, "{} panel size", language.code());
        assert_eq!(
            panel.iter().filter(|row| row.expected_nudge).count(),
            8,
            "{} toxic count",
            language.code()
        );
        assert_eq!(
            panel.iter().filter(|row| !row.expected_nudge).count(),
            16,
            "{} clean count",
            language.code()
        );
        validate_event_distribution(&panel).expect("validate event distribution");
        assert_control_distribution(language, &panel);

        let rules = word_rules(language).expect("word-language rules");
        for row in panel {
            let result = analyze_with_rules(rules, &row.text, ReplyTarget::Unknown);
            if result.should_nudge != row.expected_nudge {
                let document = TextDocument::new(&row.text);
                mismatches.push(format!(
                    "{} {} expected={} actual={} normalized={:?} evidence={:?}",
                    language.code(),
                    row.case_id,
                    row.expected_nudge,
                    result.should_nudge,
                    document.view(CandidateViewKind::Normalized).text(),
                    result.evidence,
                ));
            }
            if row.expected_nudge {
                let expected_rule_id = rule_id_for(row.event_type);
                if !result
                    .evidence
                    .iter()
                    .any(|evidence| evidence.rule_id == expected_rule_id && evidence.points > 0)
                {
                    mismatches.push(format!(
                        "{} {} lacks declared {:?} evidence: {:?}",
                        language.code(),
                        row.case_id,
                        expected_rule_id,
                        result.evidence,
                    ));
                }
            }
        }
    }

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn arabic_hindi_arabic_possessive_death_phrases_stay_clean() {
    let rules = arabic_hindi_rules(Language::Ar).expect("Arabic rules");

    for text in ["خبر موتي", "تاريخ موتي", "موتي طبيعي"] {
        let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);
        assert_eq!(result.score, 0, "{text}: {:?}", result.evidence);
        assert!(!result.should_nudge, "{text}: {:?}", result.evidence);
    }
}

#[test]
fn arabic_hindi_extended_context_markers_suppress_self_harm_commands() {
    for (language, text, expected_rule_id) in [
        (
            Language::Ar,
            "قال لي الطبيب اقتل نفسك",
            RuleId::ReportedEvidence,
        ),
        (
            Language::Ar,
            "من الخطأ أن تقول لشخص اقتل نفسك",
            RuleId::CounterspeechEvidence,
        ),
        (
            Language::Hi,
            "उसने कहा कि आत्महत्या कर ले",
            RuleId::ReportedEvidence,
        ),
        (
            Language::Hi,
            "ऐसा कहना गलत है कि आत्महत्या कर ले",
            RuleId::CounterspeechEvidence,
        ),
    ] {
        let rules = arabic_hindi_rules(language).expect("Arabic or Hindi rules");
        let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);
        assert_eq!(result.score, 0, "{text}: {:?}", result.evidence);
        assert!(!result.should_nudge, "{text}: {:?}", result.evidence);
        assert!(
            result
                .evidence
                .iter()
                .any(|evidence| evidence.rule_id == expected_rule_id && evidence.points == 0),
            "{text}: {:?}",
            result.evidence
        );
    }
}

#[test]
fn arabic_hindi_panels_have_the_required_shape() {
    let root = behavior_fixture_root();

    for language in ARABIC_HINDI_LANGUAGES {
        let panel = load_panel(&root, language).expect("load behavior panel");
        assert_eq!(panel.len(), 24, "{} panel size", language.code());
        assert_eq!(
            panel.iter().filter(|row| row.expected_nudge).count(),
            8,
            "{} toxic count",
            language.code()
        );
        assert_eq!(
            panel.iter().filter(|row| !row.expected_nudge).count(),
            16,
            "{} clean count",
            language.code()
        );
        validate_event_distribution(&panel).expect("validate event distribution");
        assert_control_distribution(language, &panel);
    }
}

#[test]
fn arabic_hindi_rules_match_their_24_row_behavior_contracts() {
    let root = behavior_fixture_root();
    let mut mismatches = Vec::new();

    for language in ARABIC_HINDI_LANGUAGES {
        let panel = load_panel(&root, language).expect("load behavior panel");
        let rules = arabic_hindi_rules(language).expect("Arabic or Hindi rules");
        for row in panel {
            let result = analyze_with_rules(rules, &row.text, ReplyTarget::Unknown);
            if result.should_nudge != row.expected_nudge {
                let document = TextDocument::new(&row.text);
                mismatches.push(format!(
                    "{} {} expected={} actual={} normalized={:?} evidence={:?}",
                    language.code(),
                    row.case_id,
                    row.expected_nudge,
                    result.should_nudge,
                    document.view(CandidateViewKind::Normalized).text(),
                    result.evidence,
                ));
            }
            if row.expected_nudge {
                let expected_rule_id = rule_id_for(row.event_type);
                if !result
                    .evidence
                    .iter()
                    .any(|evidence| evidence.rule_id == expected_rule_id && evidence.points > 0)
                {
                    mismatches.push(format!(
                        "{} {} lacks declared {:?} evidence: {:?}",
                        language.code(),
                        row.case_id,
                        expected_rule_id,
                        result.evidence,
                    ));
                }
            } else if let Some(expected_rule_id) = suppression_rule_id(row.control_kind) {
                if !result
                    .evidence
                    .iter()
                    .any(|evidence| evidence.rule_id == expected_rule_id && evidence.points == 0)
                {
                    mismatches.push(format!(
                        "{} {} lacks declared {:?} suppression: {:?}",
                        language.code(),
                        row.case_id,
                        expected_rule_id,
                        result.evidence,
                    ));
                }
            }
        }
    }

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn cjk_panels_have_the_required_shape() {
    let root = behavior_fixture_root();

    for language in CJK_LANGUAGES {
        let panel = load_panel(&root, language).expect("load behavior panel");
        assert_eq!(panel.len(), 24, "{} panel size", language.code());
        assert_eq!(
            panel.iter().filter(|row| row.expected_nudge).count(),
            8,
            "{} toxic count",
            language.code()
        );
        assert_eq!(
            panel.iter().filter(|row| !row.expected_nudge).count(),
            16,
            "{} clean count",
            language.code()
        );
        validate_event_distribution(&panel).expect("validate event distribution");
        assert_control_distribution(language, &panel);
    }
}

#[test]
fn cjk_rules_match_their_24_row_behavior_contracts() {
    let root = behavior_fixture_root();
    let mut mismatches = Vec::new();

    for language in CJK_LANGUAGES {
        let panel = load_panel(&root, language).expect("load behavior panel");
        let rules = cjk_rules(language).expect("CJK rules");
        for row in panel {
            let result = analyze_with_rules(rules, &row.text, ReplyTarget::Unknown);
            if result.should_nudge != row.expected_nudge {
                let document = TextDocument::new(&row.text);
                mismatches.push(format!(
                    "{} {} expected={} actual={} normalized={:?} evidence={:?}",
                    language.code(),
                    row.case_id,
                    row.expected_nudge,
                    result.should_nudge,
                    document.view(CandidateViewKind::Normalized).text(),
                    result.evidence,
                ));
            }
            if row.expected_nudge {
                let expected_rule_id = rule_id_for(row.event_type);
                if !result
                    .evidence
                    .iter()
                    .any(|evidence| evidence.rule_id == expected_rule_id && evidence.points > 0)
                {
                    mismatches.push(format!(
                        "{} {} lacks declared {:?} evidence: {:?}",
                        language.code(),
                        row.case_id,
                        expected_rule_id,
                        result.evidence,
                    ));
                }
            }
        }
    }

    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn cjk_controls_with_complete_frames_emit_their_suppression_evidence() {
    let root = behavior_fixture_root();
    let cases = [
        (Language::Zh, "zh-c02", RuleId::NegatedEvidence),
        (Language::Zh, "zh-c03", RuleId::QuotedEvidence),
        (Language::Zh, "zh-c04", RuleId::QuotedEvidence),
        (Language::Zh, "zh-c06", RuleId::ReportedEvidence),
        (Language::Zh, "zh-c07", RuleId::CounterspeechEvidence),
        (Language::Zh, "zh-c08", RuleId::CounterspeechEvidence),
        (Language::Ja, "ja-c03", RuleId::QuotedEvidence),
        (Language::Ja, "ja-c04", RuleId::QuotedEvidence),
        (Language::Ja, "ja-c06", RuleId::ReportedEvidence),
        (Language::Ja, "ja-c07", RuleId::CounterspeechEvidence),
        (Language::Ja, "ja-c08", RuleId::CounterspeechEvidence),
        (Language::Ko, "ko-c03", RuleId::QuotedEvidence),
        (Language::Ko, "ko-c04", RuleId::QuotedEvidence),
        (Language::Ko, "ko-c06", RuleId::ReportedEvidence),
        (Language::Ko, "ko-c07", RuleId::CounterspeechEvidence),
        (Language::Ko, "ko-c08", RuleId::CounterspeechEvidence),
    ];

    for (language, case_id, expected_rule_id) in cases {
        let panel = load_panel(&root, language).expect("load behavior panel");
        let row = panel
            .iter()
            .find(|row| row.case_id == case_id)
            .expect("CJK control case");
        let rules = cjk_rules(language).expect("CJK rules");
        let result = analyze_with_rules(rules, &row.text, ReplyTarget::Unknown);

        assert!(
            result
                .evidence
                .iter()
                .any(|evidence| evidence.rule_id == expected_rule_id && evidence.points == 0),
            "{} {case_id}: {:?}",
            language.code(),
            result.evidence
        );
    }
}

#[test]
fn chinese_harm_wish_components_match_only_a_complete_clause_outcome() {
    let rules = cjk_rules(Language::Zh).expect("Chinese rules");
    assert!(analyze_with_rules(rules, "希望你们烧死", ReplyTarget::Unknown).should_nudge,);
    assert!(!analyze_with_rules(rules, "希望你去死海旅游", ReplyTarget::Unknown).should_nudge,);
}

#[test]
fn cjk_compound_words_do_not_become_directed_insults() {
    for (language, text) in [
        (Language::Ja, "君はバカラをする"),
        (Language::Ko, "네가 새끼손가락을 다쳤다"),
        (Language::Ko, "넌 쓰레기봉투를 버려"),
        (Language::Ko, "죽여주는 너의 노래"),
        (Language::Ko, "자살 해결책을 논의한다"),
        (Language::Ko, "죽여버려도 너무 쉬운 보스"),
        (Language::Zh, "杀光你电脑里的病毒"),
        (Language::Zh, "杀光你们电脑里的病毒"),
    ] {
        let rules = cjk_rules(language).expect("CJK rules");
        let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);
        assert!(
            !result.should_nudge,
            "{} {text}: {:?}",
            language.code(),
            result.evidence
        );
    }
}

#[test]
fn cjk_suffix_reports_and_compound_outcomes_stay_clean() {
    for (language, text, expected_evidence) in [
        (
            Language::Ja,
            "管理者がお前のようなバカがいると報告した",
            Some(RuleId::ReportedEvidence),
        ),
        (
            Language::Ja,
            "管理者がお前はクズだと報告した",
            Some(RuleId::ReportedEvidence),
        ),
        (
            Language::Ko,
            "관리자가 넌 쓰레기야라고 신고했다",
            Some(RuleId::ReportedEvidence),
        ),
        (Language::Zh, "希望你去死海旅游", None),
    ] {
        let rules = cjk_rules(language).expect("CJK rules");
        let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);
        assert!(
            !result.should_nudge,
            "{} {text}: {:?}",
            language.code(),
            result.evidence
        );
        if let Some(rule_id) = expected_evidence {
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|evidence| evidence.rule_id == rule_id && evidence.points == 0),
                "{} {text}: {:?}",
                language.code(),
                result.evidence
            );
        }
    }
}

#[test]
fn vietnamese_tones_prevent_folded_word_collisions() {
    let rules = word_rules(Language::Vi).expect("Vietnamese rules");

    for text in ["Mày ngủ", "Từ từ đi"] {
        assert!(
            !analyze_with_rules(rules, text, ReplyTarget::Unknown).should_nudge,
            "{text}"
        );
    }
}

#[test]
fn turkish_threat_matches_original_and_uppercase_text() {
    let rules = word_rules(Language::Tr).expect("Turkish rules");

    for text in [
        "Seni bulunca bütün dişlerini kıracağım",
        "SENİ BULUNCA BÜTÜN DİŞLERİNİ KIRACAĞIM",
    ] {
        let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);
        assert!(result.should_nudge, "{text}: {:?}", result.evidence);
        assert!(
            result
                .evidence
                .iter()
                .any(|evidence| evidence.rule_id == RuleId::DirectThreat && evidence.points > 0),
            "{text}: {:?}",
            result.evidence
        );
    }
}

#[test]
fn an_english_possessive_object_is_not_a_person_target() {
    let rules = word_rules(Language::En).expect("English rules");

    assert!(
        !analyze_with_rules(rules, "I will kill your process", ReplyTarget::Unknown).should_nudge
    );
    assert!(
        analyze_with_rules(
            rules,
            "When I find you, I will break every bone in your hand",
            ReplyTarget::Unknown,
        )
        .should_nudge
    );
}

#[test]
fn event_distribution_requires_two_toxic_rows_per_event_type() {
    let fixture = PanelFixture::valid();
    let mut panel = load_panel(fixture.root(), Language::En).expect("load panel");
    panel[0].event_type = EventType::HarmWish;

    assert!(matches!(
        validate_event_distribution(&panel),
        Err(BehaviorPanelError::InvalidEventDistribution { .. })
    ));
}

#[test]
fn duplicate_case_identifiers_are_rejected() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tEN\ttrue\tthreat\tEN-P01\tnone\tauthored\tauthored-v1#one\tI will kill you\n\
         EN-T01\tEN\tfalse\tnone\tEN-P01\tnegation\tauthored\tauthored-v1#two\tI will not kill you\n",
        &["authored-v1#one", "authored-v1#two"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::DuplicateCaseId(case_id)) if case_id == "EN-T01"
    ));
}

#[test]
fn a_row_for_another_language_is_rejected() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tFR\ttrue\tthreat\tEN-P01\tnone\tauthored\tauthored-v1#one\tje vais te tuer\n",
        &["authored-v1#one"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::WrongLanguage {
            expected: Language::En,
            actual: Language::Fr,
            ..
        })
    ));
}

#[test]
fn unknown_enum_values_are_rejected() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tEN\ttrue\tmenace\tEN-P01\tnone\tauthored\tauthored-v1#one\tI will kill you\n",
        &["authored-v1#one"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::Csv(_))
    ));
}

#[test]
fn missing_authored_evidence_is_rejected() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tEN\ttrue\tthreat\tEN-P01\tnone\tauthored\tauthored-v1#missing\tI will kill you\n",
        &[],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::MissingEvidence { case_id, .. }) if case_id == "EN-T01"
    ));
}

#[test]
fn a_registry_reference_for_another_language_is_rejected() {
    let fixture = PanelFixture::with_registry_language(Language::Fr);

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::EvidenceLanguageMismatch {
            expected: Language::En,
            actual: Language::Fr,
            ..
        })
    ));
}

#[test]
fn missing_development_dataset_evidence_is_rejected() {
    let fixture = PanelFixture::with_dataset_reference("dataset@example/train/missing");

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::MissingDevelopmentEvidence { case_id, .. })
            if case_id == "EN-T01"
    ));
}

#[test]
fn validation_only_dataset_evidence_is_rejected() {
    let fixture = PanelFixture::with_dataset_reference("dataset@example/train/validation-only");
    fixture.write_validation_source("dataset@example/train/validation-only");

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::MissingDevelopmentEvidence { source_id, .. })
            if source_id == "dataset@example/train/validation-only"
    ));
}

#[test]
fn invalid_event_and_control_pairs_are_rejected() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tEN\ttrue\tnone\tEN-P01\tnegation\tauthored\tauthored-v1#one\tI will kill you\n",
        &["authored-v1#one"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::InvalidRowPair { case_id }) if case_id == "EN-T01"
    ));
}

#[test]
fn a_clean_pair_must_reference_a_toxic_pair() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-C01\tEN\tfalse\tnone\tEN-P99\tnegation\tauthored\tauthored-v1#one\tI will not kill you\n",
        &["authored-v1#one"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::UnknownPair { case_id, pair_id })
            if case_id == "EN-C01" && pair_id == "EN-P99"
    ));
}

#[test]
fn every_toxic_pair_requires_one_clean_link() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tEN\ttrue\tthreat\tEN-P01\tnone\tauthored\tauthored-v1#one\tI will kill you\n",
        &["authored-v1#one"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::MissingCleanPair(pair_id)) if pair_id == "EN-P01"
    ));
}

#[test]
fn a_toxic_pair_rejects_duplicate_clean_links() {
    let fixture = PanelFixture::with_panel(
        "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
         EN-T01\tEN\ttrue\tthreat\tEN-P01\tnone\tauthored\tauthored-v1#one\tI will kill you\n\
         EN-C01\tEN\tfalse\tnone\tEN-P01\tnegation\tauthored\tauthored-v1#two\tI will not kill you\n\
         EN-C02\tEN\tfalse\tnone\tEN-P01\tquotation\tauthored\tauthored-v1#three\t\"I will kill you\"\n",
        &["authored-v1#one", "authored-v1#two", "authored-v1#three"],
    );

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::DuplicateCleanPair(pair_id)) if pair_id == "EN-P01"
    ));
}

#[test]
fn matching_clean_dataset_evidence_is_accepted() {
    let source_id = "dataset@example/train/clean";
    let fixture = PanelFixture::with_clean_dataset_reference(source_id);
    fixture.write_development_row(source_id, Language::En, "clean", "The report is clean");

    let panel = load_panel(fixture.root(), Language::En).expect("matching clean evidence");

    assert_eq!(panel.len(), 1);
    assert!(!panel[0].expected_nudge);
}

#[test]
fn toxic_panel_row_rejects_clean_dataset_evidence() {
    let source_id = "dataset@example/train/clean-for-toxic";
    let fixture = PanelFixture::with_dataset_reference(source_id);
    fixture.write_development_row(source_id, Language::En, "clean", "I will kill you");

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::DevelopmentEvidenceLabelMismatch {
            source_id: actual_source_id,
            expected: EvalLabel::Toxic,
            actual: EvalLabel::Clean,
            ..
        }) if actual_source_id == source_id
    ));
}

#[test]
fn clean_panel_row_rejects_toxic_dataset_evidence() {
    let source_id = "dataset@example/train/toxic-for-clean";
    let fixture = PanelFixture::with_clean_dataset_reference(source_id);
    fixture.write_development_row(source_id, Language::En, "toxic", "The report is clean");

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::DevelopmentEvidenceLabelMismatch {
            source_id: actual_source_id,
            expected: EvalLabel::Clean,
            actual: EvalLabel::Toxic,
            ..
        }) if actual_source_id == source_id
    ));
}

#[test]
fn mismatched_dataset_evidence_text_is_rejected() {
    let source_id = "dataset@example/train/wrong-text";
    let fixture = PanelFixture::with_dataset_reference(source_id);
    fixture.write_development_row(source_id, Language::En, "toxic", "different text");

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::DevelopmentEvidenceTextMismatch { source_id: actual, .. })
            if actual == source_id
    ));
}

#[test]
fn mismatched_dataset_evidence_language_is_rejected() {
    let source_id = "dataset@example/train/wrong-language";
    let fixture = PanelFixture::with_dataset_reference(source_id);
    fixture.write_development_row(source_id, Language::Fr, "toxic", "I will kill you");

    assert!(matches!(
        load_panel(fixture.root(), Language::En),
        Err(BehaviorPanelError::DevelopmentEvidenceLanguageMismatch {
            source_id: actual,
            expected: Language::En,
            actual: Language::Fr,
            ..
        }) if actual == source_id
    ));
}

fn rule_id_for(event_type: EventType) -> RuleId {
    match event_type {
        EventType::Threat => RuleId::DirectThreat,
        EventType::HarmWish => RuleId::HostileWish,
        EventType::SelfHarmCommand => RuleId::SelfHarmCommand,
        EventType::DirectedInsult => RuleId::SemanticDirectedHostility,
        EventType::None => panic!("a toxic behavior row must declare an event type"),
    }
}

fn suppression_rule_id(control_kind: ControlKind) -> Option<RuleId> {
    match control_kind {
        ControlKind::Negation => Some(RuleId::NegatedEvidence),
        ControlKind::Quotation => Some(RuleId::QuotedEvidence),
        ControlKind::Reporting => Some(RuleId::ReportedEvidence),
        ControlKind::Counterspeech => Some(RuleId::CounterspeechEvidence),
        ControlKind::None
        | ControlKind::ViolenceQuestion
        | ControlKind::Replacement
        | ControlKind::Context
        | ControlKind::Collision => None,
    }
}

fn assert_control_distribution(language: Language, panel: &[blasphem_train::BehaviorRow]) {
    for (kind, expected) in [
        (ControlKind::Negation, 2),
        (ControlKind::Quotation, 2),
        (ControlKind::Reporting, 2),
        (ControlKind::Counterspeech, 2),
        (ControlKind::ViolenceQuestion, 2),
    ] {
        assert_eq!(
            panel
                .iter()
                .filter(|row| !row.expected_nudge && row.control_kind == kind)
                .count(),
            expected,
            "{} {kind:?} control count",
            language.code()
        );
    }
    assert_eq!(
        panel
            .iter()
            .filter(|row| {
                !row.expected_nudge
                    && matches!(
                        row.control_kind,
                        ControlKind::Replacement | ControlKind::Context | ControlKind::Collision
                    )
            })
            .count(),
        6,
        "{} extra control count",
        language.code()
    );
}

fn behavior_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/behavior")
}

struct PanelFixture {
    temporary: TempDir,
    root: PathBuf,
}

impl PanelFixture {
    fn valid() -> Self {
        let mut rows = String::from(
            "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n",
        );
        let mut references = Vec::new();
        for (index, event) in [
            "threat",
            "threat",
            "harm_wish",
            "harm_wish",
            "self_harm_command",
            "self_harm_command",
            "directed_insult",
            "directed_insult",
        ]
        .into_iter()
        .enumerate()
        {
            let reference = format!("authored-v1#toxic-{index}");
            references.push(reference.clone());
            rows.push_str(&format!(
                "EN-T{index:02}\tEN\ttrue\t{event}\tEN-P{index:02}\tnone\tauthored\t{reference}\ttoxic {index}\n"
            ));
        }
        for index in 0..16 {
            let reference = format!("authored-v1#clean-{index}");
            references.push(reference.clone());
            let pair = if index < 8 {
                format!("EN-P{index:02}")
            } else {
                "none".to_owned()
            };
            rows.push_str(&format!(
                "EN-C{index:02}\tEN\tfalse\tnone\t{pair}\tcontext\tauthored\t{reference}\tclean {index}\n"
            ));
        }
        let refs = references.iter().map(String::as_str).collect::<Vec<_>>();
        Self::with_panel(&rows, &refs)
    }

    fn with_registry_language(language: Language) -> Self {
        let fixture = Self::with_panel(
            "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
             EN-T01\tEN\ttrue\tthreat\tEN-P01\tnone\tauthored\tauthored-v1#one\tI will kill you\n",
            &[],
        );
        fixture.write_authored_registry(&[("authored-v1#one", language)]);
        fixture
    }

    fn with_dataset_reference(source_id: &str) -> Self {
        Self::with_panel(
            &format!(
                "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
                 EN-T01\tEN\ttrue\tthreat\tEN-P01\tnone\tdataset\t{source_id}\tI will kill you\n"
            ),
            &[],
        )
    }

    fn with_clean_dataset_reference(source_id: &str) -> Self {
        Self::with_panel(
            &format!(
                "case_id\tlanguage\texpected_nudge\tevent_type\tpair_id\tcontrol_kind\tevidence_kind\tevidence_ref\ttext\n\
                 EN-C01\tEN\tfalse\tnone\tnone\tcontext\tdataset\t{source_id}\tThe report is clean\n"
            ),
            &[],
        )
    }

    fn with_panel(panel: &str, references: &[&str]) -> Self {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let root = temporary.path().join("tests/fixtures/behavior");
        fs::create_dir_all(&root).expect("create fixture root");
        fs::write(root.join("en.tsv"), panel).expect("write panel");
        fs::write(
            root.join("native-review-v1.tsv"),
            "evidence_ref\tlanguage\n",
        )
        .expect("write native registry");
        let fixture = Self { temporary, root };
        let authored = references
            .iter()
            .map(|reference| (*reference, Language::En))
            .collect::<Vec<_>>();
        fixture.write_authored_registry(&authored);
        fixture.write_development_sources(&[]);
        fixture
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn write_authored_registry(&self, rows: &[(&str, Language)]) {
        let mut contents = String::from("evidence_ref\tlanguage\n");
        for (reference, language) in rows {
            contents.push_str(reference);
            contents.push('\t');
            contents.push_str(language.code());
            contents.push('\n');
        }
        fs::write(self.root.join("authored-v1.tsv"), contents).expect("write authored registry");
    }

    fn write_development_sources(&self, source_ids: &[&str]) {
        self.write_dataset_split("development", source_ids);
    }

    fn write_development_row(&self, source_id: &str, language: Language, label: &str, text: &str) {
        let path = self.temporary.path().join("data/prepared-draft-v1/EN");
        fs::create_dir_all(&path).expect("create prepared data root");
        fs::write(
            path.join("development.tsv"),
            format!(
                "detector_language\tlabel\tsource_id\ttext\n{}\t{label}\t{source_id}\t{text}\n",
                language.code()
            ),
        )
        .expect("write prepared development row");
    }

    fn write_validation_source(&self, source_id: &str) {
        self.write_dataset_split("validation", &[source_id]);
    }

    fn write_dataset_split(&self, split: &str, source_ids: &[&str]) {
        let path = self.temporary.path().join("data/prepared-draft-v1/EN");
        fs::create_dir_all(&path).expect("create prepared data root");
        let mut contents = String::from("detector_language\tlabel\tsource_id\ttext\n");
        for source_id in source_ids {
            contents.push_str("EN\ttoxic\t");
            contents.push_str(source_id);
            contents.push_str("\tfixture text\n");
        }
        fs::write(path.join(format!("{split}.tsv")), contents).expect("write prepared split");
    }
}
