use sha2::{Digest, Sha256};
use toxcheck::{
    DIRECT_THREAT_SCORE, DIRECTED_INSULT_SCORE, HARM_WISH_SCORE, Language, LanguageRules,
    NEGATIVE_SENTIMENT_SCORE, PhraseSet, ReplyTarget, RuleId, RuleMatchProfile,
    SELF_HARM_COMMAND_SCORE, analyze_with_rules, arabic_hindi_rules, canonical_rule_identity_for,
    cjk_rules, word_rules,
};

fn fixture_rules() -> LanguageRules {
    fixture_rules_with(PhraseSet::new(&["it is wrong to say"]))
}

#[test]
fn language_rules_cover_second_person_morphology_and_normalized_future_threats() {
    let cases = [
        (
            arabic_hindi_rules(Language::Ar).expect("Arabic rules"),
            "أتمنى أن تموت وحيدًا هذه الليلة",
            HARM_WISH_SCORE,
        ),
        (
            arabic_hindi_rules(Language::Hi).expect("Hindi rules"),
            "मैं तुम्हें ढूँढकर तुम्हारी हड्डियाँ तोड़ दूँगा",
            DIRECT_THREAT_SCORE,
        ),
        (
            word_rules(Language::It).expect("Italian rules"),
            "ti ucciderò",
            DIRECT_THREAT_SCORE,
        ),
    ];

    for (rules, text, expected) in cases {
        assert_eq!(
            analyze_with_rules(rules, text, ReplyTarget::Unknown).score,
            expected,
            "{}: {text}",
            rules.language.code(),
        );
    }
}

#[test]
fn morphology_rules_require_a_complete_hostile_frame() {
    let cases = [
        (
            arabic_hindi_rules(Language::Ar).expect("Arabic rules"),
            "قد تموت البكتيريا في المختبر",
        ),
        (
            arabic_hindi_rules(Language::Ar).expect("Arabic rules"),
            "أتمنى أن تموت البكتيريا في المختبر",
        ),
        (
            arabic_hindi_rules(Language::Ar).expect("Arabic rules"),
            "أتمنى أن تموت الزهور بعد الشتاء",
        ),
        (
            arabic_hindi_rules(Language::Hi).expect("Hindi rules"),
            "तुम्हारी हड्डियाँ मजबूत हैं",
        ),
        (
            word_rules(Language::It).expect("Italian rules"),
            "Il film parla della parola ucciderò",
        ),
    ];

    for (rules, text) in cases {
        assert_eq!(
            analyze_with_rules(rules, text, ReplyTarget::Unknown).score,
            0,
            "{}: {text}",
            rules.language.code(),
        );
    }
}

fn fixture_rules_with(counterspeech_markers: PhraseSet) -> LanguageRules {
    LanguageRules {
        language: Language::En,
        version: 2,
        targets: PhraseSet::new(&["you", "him"]),
        harm_predicates: PhraseSet::new(&["kill"]),
        intent_markers: PhraseSet::new(&["i will"]),
        implicit_target_threats: PhraseSet::empty(),
        wish_markers: PhraseSet::new(&["i hope"]),
        harm_outcomes: PhraseSet::new(&["die"]),
        implicit_target_harm_wishes: PhraseSet::empty(),
        self_harm_commands: PhraseSet::new(&["kill yourself"]),
        strong_insults: PhraseSet::new(&["idiot"]),
        implicit_target_directed_insults: PhraseSet::empty(),
        negative_sentiment: PhraseSet::new(&["awful"]),
        copulas_or_vocatives: PhraseSet::new(&["are"]),
        negators: PhraseSet::new(&["not"]),
        reports: PhraseSet::new(&["wrote"]),
        counterspeech_markers,
        proposition_boundaries: PhraseSet::new(&["but"]),
        matching: RuleMatchProfile::WordClauses,
    }
}

fn compact_fixture_rules() -> LanguageRules {
    LanguageRules {
        language: Language::Ja,
        version: 1,
        targets: PhraseSet::empty(),
        harm_predicates: PhraseSet::empty(),
        intent_markers: PhraseSet::empty(),
        implicit_target_threats: PhraseSet::new(&["殺してやる"]),
        wish_markers: PhraseSet::empty(),
        harm_outcomes: PhraseSet::empty(),
        implicit_target_harm_wishes: PhraseSet::new(&["死ねばいい"]),
        self_harm_commands: PhraseSet::empty(),
        strong_insults: PhraseSet::empty(),
        implicit_target_directed_insults: PhraseSet::empty(),
        negative_sentiment: PhraseSet::empty(),
        copulas_or_vocatives: PhraseSet::empty(),
        negators: PhraseSet::empty(),
        reports: PhraseSet::empty(),
        counterspeech_markers: PhraseSet::empty(),
        proposition_boundaries: PhraseSet::empty(),
        matching: RuleMatchProfile::CompactClauses,
    }
}

fn with_profile(matching: RuleMatchProfile) -> LanguageRules {
    LanguageRules {
        matching,
        ..fixture_rules()
    }
}

fn compact_exact_rules(
    language: Language,
    threats: PhraseSet,
    harm_wishes: PhraseSet,
) -> LanguageRules {
    LanguageRules {
        language,
        version: 1,
        targets: PhraseSet::empty(),
        harm_predicates: PhraseSet::empty(),
        intent_markers: PhraseSet::empty(),
        implicit_target_threats: threats,
        wish_markers: PhraseSet::empty(),
        harm_outcomes: PhraseSet::empty(),
        implicit_target_harm_wishes: harm_wishes,
        self_harm_commands: PhraseSet::empty(),
        strong_insults: PhraseSet::empty(),
        implicit_target_directed_insults: PhraseSet::empty(),
        negative_sentiment: PhraseSet::empty(),
        copulas_or_vocatives: PhraseSet::empty(),
        negators: PhraseSet::empty(),
        reports: PhraseSet::empty(),
        counterspeech_markers: PhraseSet::empty(),
        proposition_boundaries: PhraseSet::empty(),
        matching: RuleMatchProfile::CompactClauses,
    }
}

fn word_implicit_rules() -> LanguageRules {
    LanguageRules {
        implicit_target_threats: PhraseSet::new(&["will kill"]),
        implicit_target_harm_wishes: PhraseSet::new(&["hope they die"]),
        implicit_target_directed_insults: PhraseSet::new(&["absolute idiot"]),
        ..fixture_rules()
    }
}

#[test]
fn harm_words_without_speaker_intent_do_not_form_a_threat() {
    let result = analyze_with_rules(&fixture_rules(), "Did you kill him?", ReplyTarget::Unknown);

    assert_eq!(result.score, 0);
    assert!(!result.should_nudge);
}

#[test]
fn harm_first_questions_are_not_imperatives() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let rules = with_profile(matching);

        for text in [
            "Kill you?",
            "Kill you？",
            "Kill you؟",
            "Kill you!?",
            "Kill you! ?",
        ] {
            assert_eq!(
                analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
                0,
                "{matching:?}: {text}"
            );
        }
        for text in ["Kill you", "Kill you!", "Kill you."] {
            assert_eq!(
                analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
                DIRECT_THREAT_SCORE,
                "{matching:?}: {text}"
            );
        }
    }
}

#[test]
fn event_components_must_stay_within_a_bounded_gap() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let rules = with_profile(matching);
        for text in [
            "I will explain why they kill you",
            "I hope you recover fully before you die",
        ] {
            assert_eq!(
                analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
                0,
                "{matching:?}: {text}"
            );
        }

        assert_eq!(
            analyze_with_rules(&rules, "I will try to kill you", ReplyTarget::Unknown).score,
            DIRECT_THREAT_SCORE,
            "{matching:?}"
        );
        assert_eq!(
            analyze_with_rules(&rules, "I hope you die", ReplyTarget::Unknown).score,
            HARM_WISH_SCORE,
            "{matching:?}"
        );
    }
}

#[test]
fn arabic_and_indic_terminators_bound_event_frames() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let rules = with_profile(matching);
        for text in [
            "I will explain। They kill you",
            "I will۔ They kill you",
            "I will॥ They kill you",
        ] {
            assert_eq!(
                analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
                0,
                "{matching:?}: {text}"
            );
        }
    }
}

#[test]
fn non_exact_negators_suppress_the_complete_frame() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let direct_rules = with_profile(matching);
        let wish_rules = LanguageRules {
            wish_markers: PhraseSet::new(&["hope"]),
            matching,
            ..fixture_rules()
        };
        for (rules, text) in [
            (&direct_rules, "I will not try to kill you"),
            (&wish_rules, "I do not hope you die"),
        ] {
            let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);

            assert_eq!(result.score, 0, "{matching:?}: {text}");
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|item| item.rule_id == RuleId::NegatedEvidence),
                "{matching:?}: {text}"
            );
        }
    }
}

#[test]
fn nearby_negators_before_an_event_frame_are_suppressed() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let command_rules = with_profile(matching);
        let wish_rules = LanguageRules {
            wish_markers: PhraseSet::new(&["hope"]),
            matching,
            ..fixture_rules()
        };
        for (rules, text) in [
            (&command_rules, "Do not ever kill yourself"),
            (&wish_rules, "I do not ever hope you die"),
        ] {
            let result = analyze_with_rules(rules, text, ReplyTarget::Unknown);

            assert_eq!(result.score, 0, "{matching:?}: {text}");
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|item| item.rule_id == RuleId::NegatedEvidence),
                "{matching:?}: {text}"
            );
        }

        assert_eq!(
            analyze_with_rules(
                &command_rules,
                "Do not discuss this topic for days before kill yourself",
                ReplyTarget::Unknown,
            )
            .score,
            SELF_HARM_COMMAND_SCORE,
            "{matching:?}"
        );
        assert_eq!(
            analyze_with_rules(
                &wish_rules,
                "I do not discuss this topic before hope you die",
                ReplyTarget::Unknown,
            )
            .score,
            HARM_WISH_SCORE,
            "{matching:?}"
        );
    }
}

#[test]
fn negators_do_not_cross_propositions_or_comma_scopes() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let rules = with_profile(matching);
        for text in [
            "Do not wait, but kill yourself",
            "I am not ready, kill yourself",
        ] {
            assert_eq!(
                analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
                SELF_HARM_COMMAND_SCORE,
                "{matching:?}: {text}"
            );
        }
    }
}

#[test]
fn exact_implicit_events_keep_internal_negators() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let rules = LanguageRules {
            implicit_target_threats: PhraseSet::new(&["will not kill"]),
            matching,
            ..fixture_rules()
        };

        assert_eq!(
            analyze_with_rules(&rules, "will not kill", ReplyTarget::Unknown).score,
            DIRECT_THREAT_SCORE,
            "{matching:?}"
        );
    }
}

#[test]
fn complete_event_frames_receive_their_fixed_scores() {
    let rules = fixture_rules();
    for (text, score, rule_id) in [
        ("I will kill you", DIRECT_THREAT_SCORE, RuleId::DirectThreat),
        ("I hope you die", HARM_WISH_SCORE, RuleId::HostileWish),
        (
            "kill yourself",
            SELF_HARM_COMMAND_SCORE,
            RuleId::SelfHarmCommand,
        ),
        (
            "you are idiot",
            DIRECTED_INSULT_SCORE,
            RuleId::SemanticDirectedHostility,
        ),
    ] {
        let result = analyze_with_rules(&rules, text, ReplyTarget::Unknown);

        assert_eq!(result.score, score, "{text}");
        assert!(result.should_nudge, "{text}");
        assert!(
            result.evidence.iter().any(|item| item.rule_id == rule_id),
            "{text}"
        );
    }
}

#[test]
fn negation_quote_report_and_counterspeech_suppress_the_linked_event() {
    let rules = fixture_rules();
    for (text, rule_id) in [
        ("I will not kill you", RuleId::NegatedEvidence),
        ("\"I will kill you,\" she wrote", RuleId::QuotedEvidence),
        ("She wrote I will kill you", RuleId::ReportedEvidence),
        (
            "It is wrong to say I will kill you",
            RuleId::CounterspeechEvidence,
        ),
    ] {
        let result = analyze_with_rules(&rules, text, ReplyTarget::Unknown);

        assert_eq!(result.score, 0, "{text}");
        assert!(!result.should_nudge, "{text}");
        assert!(
            result.evidence.iter().any(|item| item.rule_id == rule_id),
            "{text}"
        );
    }
}

#[test]
fn word_reports_and_counterspeech_link_across_commas_only() {
    let rules = fixture_rules();
    for (text, rule_id) in [
        ("She wrote, I will kill you", RuleId::ReportedEvidence),
        (
            "It is wrong to say, I will kill you",
            RuleId::CounterspeechEvidence,
        ),
    ] {
        let result = analyze_with_rules(&rules, text, ReplyTarget::Unknown);

        assert_eq!(result.score, 0, "{text}");
        assert!(
            result.evidence.iter().any(|item| item.rule_id == rule_id),
            "{text}"
        );
    }

    for text in [
        "She wrote. I will kill you",
        "It is wrong to say. I will kill you",
    ] {
        assert_eq!(
            analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
            DIRECT_THREAT_SCORE,
            "{text}"
        );
    }
}

#[test]
fn a_quoted_predicate_does_not_suppress_an_unquoted_complete_frame() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let result = analyze_with_rules(
            &with_profile(matching),
            "I will \"kill\" you",
            ReplyTarget::Unknown,
        );

        assert_eq!(result.score, DIRECT_THREAT_SCORE, "{matching:?}");
        assert!(result.should_nudge, "{matching:?}");
    }
}

#[test]
fn alternate_balanced_quotes_suppress_complete_events() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let rules = with_profile(matching);
        for text in ["『I will kill you』", "»I will kill you«"] {
            let result = analyze_with_rules(&rules, text, ReplyTarget::Unknown);

            assert_eq!(result.score, 0, "{matching:?}: {text}");
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|item| item.rule_id == RuleId::QuotedEvidence),
                "{matching:?}: {text}"
            );
        }
        for text in ["『I will kill you", "»I will kill you"] {
            assert_eq!(
                analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
                DIRECT_THREAT_SCORE,
                "{matching:?}: {text}"
            );
        }
    }
}

#[test]
fn a_target_in_another_proposition_does_not_complete_a_threat() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        for text in [
            "I will kill time, but you can help",
            "I will kill time but you can help",
            "I will kill time then later you can help",
            "I will kill time. You can help",
        ] {
            let result = analyze_with_rules(&with_profile(matching), text, ReplyTarget::Unknown);

            assert_eq!(result.score, 0, "{matching:?}: {text}");
        }
    }
}

#[test]
fn a_distant_target_does_not_complete_a_directed_insult() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        let result = analyze_with_rules(
            &with_profile(matching),
            "You discussed the game and they are idiot",
            ReplyTarget::Unknown,
        );

        assert_eq!(result.score, 0, "{matching:?}");
        assert_eq!(
            analyze_with_rules(
                &with_profile(matching),
                "you are idiot",
                ReplyTarget::Unknown,
            )
            .score,
            DIRECTED_INSULT_SCORE,
            "{matching:?}"
        );
    }
}

#[test]
fn an_exact_harm_wish_can_span_a_comma_and_keep_its_internal_negator() {
    let rules = LanguageRules {
        language: Language::De,
        targets: PhraseSet::empty(),
        harm_predicates: PhraseSet::empty(),
        intent_markers: PhraseSet::empty(),
        implicit_target_threats: PhraseSet::empty(),
        wish_markers: PhraseSet::empty(),
        harm_outcomes: PhraseSet::empty(),
        implicit_target_harm_wishes: PhraseSet::new(&["ich hoffe du wachst morgen nicht mehr auf"]),
        self_harm_commands: PhraseSet::empty(),
        strong_insults: PhraseSet::empty(),
        implicit_target_directed_insults: PhraseSet::empty(),
        negative_sentiment: PhraseSet::empty(),
        copulas_or_vocatives: PhraseSet::empty(),
        negators: PhraseSet::new(&["nicht"]),
        reports: PhraseSet::new(&["schrieb"]),
        counterspeech_markers: PhraseSet::new(&["sag das nicht"]),
        proposition_boundaries: PhraseSet::new(&["aber"]),
        matching: RuleMatchProfile::WordClauses,
        version: 1,
    };

    let result = analyze_with_rules(
        &rules,
        "Ich hoffe, du wachst morgen nicht mehr auf",
        ReplyTarget::Unknown,
    );

    assert_eq!(result.score, HARM_WISH_SCORE);
    assert!(result.should_nudge);
}

#[test]
fn compact_clause_normalization_maps_composed_events_to_decomposed_raw_bytes() {
    let cases = [
        (
            compact_exact_rules(
                Language::Ja,
                PhraseSet::new(&["死ねばいいが"]),
                PhraseSet::empty(),
            ),
            "死ねばいいか\u{3099}",
            RuleId::DirectThreat,
        ),
        (
            compact_exact_rules(
                Language::Ko,
                PhraseSet::empty(),
                PhraseSet::new(&["죽어가"]),
            ),
            "죽어\u{1100}\u{1161}",
            RuleId::HostileWish,
        ),
    ];

    for (rules, text, rule_id) in cases {
        let result = analyze_with_rules(&rules, text, ReplyTarget::Unknown);
        let evidence = result
            .evidence
            .iter()
            .find(|item| item.rule_id == rule_id)
            .expect("normalized compact event evidence");

        assert!(result.should_nudge, "{}", rules.language.code());
        assert_eq!(evidence.raw_start, Some(0));
        assert_eq!(evidence.raw_end, Some(text.len()));
        assert_eq!(evidence.matched_text, text);
    }
}

#[test]
fn a_suppressed_event_does_not_hide_separate_targeted_sentiment() {
    for matching in [
        RuleMatchProfile::WordClauses,
        RuleMatchProfile::CompactClauses,
    ] {
        for text in [
            "She wrote I will kill you, but you are awful",
            "She wrote I will kill you and you are awful",
        ] {
            let result = analyze_with_rules(&with_profile(matching), text, ReplyTarget::Unknown);

            assert_eq!(
                result.score, NEGATIVE_SENTIMENT_SCORE,
                "{matching:?}: {text}"
            );
            assert!(!result.should_nudge, "{matching:?}: {text}");
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|item| item.rule_id == RuleId::NegativeSentiment),
                "{matching:?}: {text}"
            );
        }
    }
}

#[test]
fn suppression_does_not_hide_a_separate_event() {
    let result = analyze_with_rules(
        &fixture_rules(),
        "She wrote I will kill you. I will kill you",
        ReplyTarget::Unknown,
    );

    assert_eq!(result.score, DIRECT_THREAT_SCORE);
    assert!(result.should_nudge);
    assert!(
        result
            .evidence
            .iter()
            .any(|item| item.rule_id == RuleId::ReportedEvidence)
    );
    assert!(
        result
            .evidence
            .iter()
            .any(|item| item.rule_id == RuleId::DirectThreat)
    );
}

#[test]
fn an_earlier_incomplete_or_suppressed_frame_does_not_hide_a_later_threat() {
    let rules = fixture_rules();
    for text in [
        "Did you kill him, but I will kill you",
        "She wrote I will kill him, but I will kill you",
    ] {
        let result = analyze_with_rules(&rules, text, ReplyTarget::Unknown);

        assert_eq!(result.score, DIRECT_THREAT_SCORE, "{text}");
        assert!(result.should_nudge, "{text}");
    }
}

#[test]
fn compact_matching_scores_a_later_complete_frame() {
    let rules = LanguageRules {
        matching: RuleMatchProfile::CompactClauses,
        ..fixture_rules()
    };
    let result = analyze_with_rules(
        &rules,
        "Did you kill him, but I will kill you",
        ReplyTarget::Unknown,
    );

    assert_eq!(result.score, DIRECT_THREAT_SCORE);
}

#[test]
fn reply_targets_supply_only_the_target_cue() {
    let rules = fixture_rules();
    for reply_target in [ReplyTarget::Person, ReplyTarget::ProtectedGroup] {
        let threat = analyze_with_rules(&rules, "I will kill", reply_target);
        assert_eq!(threat.score, DIRECT_THREAT_SCORE);

        let insult = analyze_with_rules(&rules, "idiot", reply_target);
        assert_eq!(insult.score, DIRECTED_INSULT_SCORE);

        let incomplete = analyze_with_rules(&rules, "kill", reply_target);
        assert_eq!(incomplete.score, 0);
    }

    let unknown = analyze_with_rules(&rules, "I will kill", ReplyTarget::Unknown);
    assert_eq!(unknown.score, 0);
}

#[test]
fn reply_targets_do_not_bypass_event_suppression() {
    let result = analyze_with_rules(
        &fixture_rules(),
        "It is wrong to say I will kill",
        ReplyTarget::Person,
    );

    assert_eq!(result.score, 0);
    assert!(!result.should_nudge);
    assert!(
        result
            .evidence
            .iter()
            .any(|item| item.rule_id == RuleId::CounterspeechEvidence)
    );
}

#[test]
fn targeted_negative_sentiment_stays_below_the_nudge_threshold() {
    let rules = fixture_rules();
    let targeted = analyze_with_rules(&rules, "you are awful", ReplyTarget::Unknown);
    let reply_targeted = analyze_with_rules(&rules, "awful", ReplyTarget::Person);
    let untargeted = analyze_with_rules(&rules, "an awful day", ReplyTarget::Unknown);

    assert_eq!(targeted.score, NEGATIVE_SENTIMENT_SCORE);
    assert_eq!(reply_targeted.score, NEGATIVE_SENTIMENT_SCORE);
    assert!(!targeted.should_nudge);
    assert!(!reply_targeted.should_nudge);
    assert_eq!(untargeted.score, 0);
}

#[test]
fn the_largest_unsuppressed_event_sets_the_score() {
    let result = analyze_with_rules(
        &fixture_rules(),
        "I hope you die. I will kill you",
        ReplyTarget::Unknown,
    );

    assert_eq!(result.score, DIRECT_THREAT_SCORE);
    assert_eq!(
        result
            .evidence
            .iter()
            .filter(|item| matches!(item.rule_id, RuleId::DirectThreat | RuleId::HostileWish))
            .count(),
        2
    );
}

#[test]
fn implicit_target_fields_match_only_complete_compact_clauses() {
    let rules = compact_fixture_rules();

    assert_eq!(
        analyze_with_rules(&rules, "殺 してやる", ReplyTarget::Unknown).score,
        DIRECT_THREAT_SCORE
    );
    assert_eq!(
        analyze_with_rules(&rules, "死ねばいい", ReplyTarget::Unknown).score,
        HARM_WISH_SCORE
    );
    assert_eq!(
        analyze_with_rules(&rules, "彼は死ねばいいと言った", ReplyTarget::Unknown).score,
        0
    );
}

#[test]
fn implicit_target_fields_match_only_complete_word_propositions() {
    let rules = word_implicit_rules();
    for (text, score) in [
        ("will kill", DIRECT_THREAT_SCORE),
        ("hope they die", HARM_WISH_SCORE),
        ("absolute idiot", DIRECTED_INSULT_SCORE),
    ] {
        assert_eq!(
            analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
            score,
            "{text}"
        );
    }

    for text in [
        "\"will kill\"",
        "She wrote will kill",
        "not will kill",
        "They think will kill",
        "will kill tomorrow",
    ] {
        assert_eq!(
            analyze_with_rules(&rules, text, ReplyTarget::Unknown).score,
            0,
            "{text}"
        );
    }
}

#[test]
fn canonical_identity_covers_field_ownership_and_counterspeech() {
    let original = canonical_rule_identity_for(&fixture_rules());
    let changed_counterspeech =
        canonical_rule_identity_for(&fixture_rules_with(PhraseSet::new(&["stop saying"])));
    let moved_phrase = canonical_rule_identity_for(&LanguageRules {
        harm_predicates: PhraseSet::empty(),
        negative_sentiment: PhraseSet::new(&["awful", "kill"]),
        ..fixture_rules()
    });
    let changed_boundary = canonical_rule_identity_for(&LanguageRules {
        proposition_boundaries: PhraseSet::new(&["however"]),
        ..fixture_rules()
    });

    assert_ne!(original, changed_counterspeech);
    assert_ne!(original, moved_phrase);
    assert_ne!(original, changed_boundary);
}

#[test]
fn canonical_identity_has_one_fixed_fixture_hash() {
    let identity = canonical_rule_identity_for(&fixture_rules());

    assert!(identity.starts_with(b"TOXRULE1EN"));
    assert_eq!(
        format!("{:x}", Sha256::digest(identity)),
        "d9a06ce3b641b7fb5b7cdd08a95f8455cc0e92bf0c7f61851333feadc5f99ba1"
    );
}

#[test]
fn cjk_boundaries_and_questions_do_not_complete_cross_scope_threats() {
    for (language, text) in [
        (Language::Zh, "杀光害虫但是你可以走了"),
        (Language::Zh, "杀光害虫。你可以走了"),
        (Language::Ja, "殴るのは木だしかしお前は帰っていい"),
        (Language::Ja, "殴るのは木だ。お前は帰っていい"),
        (Language::Ko, "죽여야 할 건 벌레 그러나 너는 가도 된다"),
        (Language::Ko, "죽여야 할 건 벌레. 너는 가도 된다"),
    ] {
        let rules = cjk_rules(language).expect("CJK rules");
        assert!(
            !analyze_with_rules(rules, text, ReplyTarget::Unknown).should_nudge,
            "{} {text}",
            language.code()
        );
    }

    for (language, question, command) in [
        (Language::Ja, "殴る、お前？", "殴る、お前"),
        (Language::Ko, "죽여버려, 너를?", "죽여버려, 너를"),
    ] {
        let rules = cjk_rules(language).expect("CJK rules");
        assert!(
            !analyze_with_rules(rules, question, ReplyTarget::Unknown).should_nudge,
            "{} {question}",
            language.code()
        );
        assert!(
            analyze_with_rules(rules, command, ReplyTarget::Unknown).should_nudge,
            "{} {command}",
            language.code()
        );
    }
}
