use blasphem::{
    HURTLEX_SCORE, Language, ReplyTarget, RuleChannel, RuleId, canonical_rule_identity,
};
use sha2::{Digest, Sha256};
use std::path::Path;

fn hurtlex(language: Language, rows: &[(&str, &str)]) -> Vec<u8> {
    let mut contents = String::from("id\tpos\tcategory\tstereotype\tlemma\tlevel\n");
    for (index, (lemma, level)) in rows.iter().enumerate() {
        contents.push_str(&format!(
            "{}{index}\tn\tcds\tno\t{lemma}\t{level}\n",
            language.code()
        ));
    }
    contents.into_bytes()
}

#[test]
fn every_language_resolves_to_one_static_rule_path() {
    for language in Language::ALL {
        RuleChannel::from_hurtlex_bytes(language, None)
            .unwrap_or_else(|error| panic!("{}: {error}", language.code()));
    }
}

#[test]
fn every_locked_hurtlex_file_builds_its_matching_language_channel() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/raw-v1/hurtlex");

    for language in Language::ALL {
        let path = root
            .join(language.storage_code())
            .join("1.2")
            .join(format!("hurtlex_{}.tsv", language.storage_code()));
        let bytes =
            std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        RuleChannel::from_hurtlex_bytes(language, Some(&bytes))
            .unwrap_or_else(|error| panic!("{}: {error}", language.code()));
    }
}

#[test]
fn supplied_cjk_messages_pass_through_the_shared_channel() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/raw-v1/hurtlex");
    let cases = [
        (
            Language::Zh,
            "等我找到你，我会打断你的双腿",
            "管理员删除了那条威胁别人的消息",
        ),
        (
            Language::Ja,
            "今度会ったら顔が分からなくなるまで殴る",
            "管理者は他の利用者を脅す投稿を削除した",
        ),
        (
            Language::Ko,
            "오늘 밤 네가 다시는 깨어나지 않기를 바란다",
            "관리자가 다른 사용자를 협박한 메시지를 삭제했다",
        ),
    ];

    for (language, toxic, clean) in cases {
        let path = root
            .join(language.storage_code())
            .join("1.2")
            .join(format!("hurtlex_{}.tsv", language.storage_code()));
        let bytes =
            std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let channel =
            RuleChannel::from_hurtlex_bytes(language, Some(&bytes)).expect("rule channel");

        assert!(
            channel.analyze(toxic, ReplyTarget::Unknown).should_nudge,
            "{} {toxic}",
            language.code()
        );
        assert!(
            !channel.analyze(clean, ReplyTarget::Unknown).should_nudge,
            "{} {clean}",
            language.code()
        );
    }
}

#[test]
fn inclusive_hurtlex_rows_cannot_create_rule_events() {
    let bytes = hurtlex(Language::En, &[("buffoon", "inclusive")]);
    let channel =
        RuleChannel::from_hurtlex_bytes(Language::En, Some(&bytes)).expect("rule channel");

    let result = channel.analyze("buffoon", ReplyTarget::Unknown);

    assert_eq!(result.score, 0);
    assert!(!result.should_nudge);
    assert!(result.evidence.is_empty());
}

#[test]
fn a_hurtlex_only_match_scores_below_the_nudge_threshold() {
    let bytes = hurtlex(Language::En, &[("buffoon", "conservative")]);
    let channel =
        RuleChannel::from_hurtlex_bytes(Language::En, Some(&bytes)).expect("rule channel");

    let result = channel.analyze("buffoon", ReplyTarget::Unknown);

    assert_eq!(result.score, HURTLEX_SCORE);
    assert!(!result.should_nudge);
    assert!(
        result
            .evidence
            .iter()
            .any(|item| item.rule_id == RuleId::LexicalMatch && item.points == HURTLEX_SCORE)
    );
}

#[test]
fn semantic_and_hurtlex_scores_compose_with_maximum() {
    let bytes = hurtlex(Language::En, &[("kill", "conservative")]);
    let channel =
        RuleChannel::from_hurtlex_bytes(Language::En, Some(&bytes)).expect("rule channel");

    let result = channel.analyze("I will kill you", ReplyTarget::Unknown);

    assert_eq!(result.score, 95);
    assert!(result.should_nudge);
    assert!(result.evidence.iter().any(|item| item.points == 95));
    assert!(result.evidence.iter().any(|item| item.points == 30));
}

#[test]
fn reply_target_completes_a_bare_strong_insult() {
    let channel = RuleChannel::from_hurtlex_bytes(Language::En, None).expect("rule channel");

    let result = channel.analyze("moron", ReplyTarget::Person);

    assert_eq!(result.score, 70);
    assert!(result.should_nudge);
}

#[test]
fn a_wrong_language_hurtlex_file_is_rejected() {
    let bytes = hurtlex(Language::Ja, &[("問題", "conservative")]);

    assert!(RuleChannel::from_hurtlex_bytes(Language::Ko, Some(&bytes)).is_err());
}

#[test]
fn korean_collisions_are_zero_point_evidence() {
    let bytes = hurtlex(
        Language::Ko,
        &[
            ("질", "conservative"),
            ("김", "conservative"),
            ("협박", "conservative"),
        ],
    );
    let channel =
        RuleChannel::from_hurtlex_bytes(Language::Ko, Some(&bytes)).expect("rule channel");

    let unrelated = channel.analyze("메시지를 보냈다", ReplyTarget::Unknown);
    assert_eq!(unrelated.score, 0, "{:?}", unrelated.evidence);
    assert!(
        unrelated.evidence.is_empty(),
        "no lemma is a whole word in this text: {:?}",
        unrelated.evidence
    );

    let result = channel.analyze("김 씨가 왔다", ReplyTarget::Unknown);
    assert_eq!(result.score, 0, "{:?}", result.evidence);
    assert!(!result.should_nudge, "{:?}", result.evidence);
    assert!(
        result
            .evidence
            .iter()
            .any(|item| { item.rule_id == RuleId::LexicalCollisionExcluded && item.points == 0 }),
        "{:?}",
        result.evidence
    );

    let report = channel.analyze("뉴스가 협박 메시지 삭제를 보도했다", ReplyTarget::Unknown);
    assert_eq!(report.score, HURTLEX_SCORE);
    assert!(!report.should_nudge);
}

#[test]
fn every_reviewed_cjk_collision_is_excluded() {
    let cases: [(Language, &[&str]); 3] = [
        (
            Language::Zh,
            &[
                "和", "日", "警察", "警官", "民众", "群众", "刑事", "笔者", "困难", "同性", "暴力",
                "反派",
            ],
        ),
        (
            Language::Ja,
            &[
                "女", "問題", "下手", "平凡", "初心", "刑事", "婦警", "大衆", "庶民", "平民",
            ],
        ),
        (
            Language::Ko,
            &[
                "김", "질", "의사", "작가", "정부", "음식", "날짜", "완전", "낚시", "경찰", "경관",
                "여경", "대중", "민중", "서민",
            ],
        ),
    ];

    for (language, lemmas) in cases {
        let rows = lemmas
            .iter()
            .map(|lemma| (*lemma, "conservative"))
            .collect::<Vec<_>>();
        let bytes = hurtlex(language, &rows);
        let channel =
            RuleChannel::from_hurtlex_bytes(language, Some(&bytes)).expect("rule channel");
        for lemma in lemmas {
            let result = channel.analyze(lemma, ReplyTarget::Unknown);
            assert_eq!(
                result.score,
                0,
                "{} {lemma}: {:?}",
                language.code(),
                result.evidence
            );
            assert!(
                result
                    .evidence
                    .iter()
                    .any(|item| item.rule_id == RuleId::LexicalCollisionExcluded),
                "{} {lemma}: {:?}",
                language.code(),
                result.evidence
            );
        }
    }
}

#[test]
fn the_audited_german_phrase_reactivates_only_its_lexical_score() {
    let bytes = hurtlex(Language::De, &[("hund", "conservative")]);
    let channel =
        RuleChannel::from_hurtlex_bytes(Language::De, Some(&bytes)).expect("rule channel");

    let neutral = channel.analyze("Der Hund schläft", ReplyTarget::Unknown);
    assert_eq!(neutral.score, 0);
    assert!(!neutral.should_nudge);

    let directed = channel.analyze("du hund", ReplyTarget::Unknown);
    assert_eq!(directed.score, HURTLEX_SCORE);
    assert!(!directed.should_nudge);
    assert!(
        directed
            .evidence
            .iter()
            .any(|item| item.rule_id == RuleId::LexicalMatch && item.points == HURTLEX_SCORE)
    );
    assert!(!directed.evidence.iter().any(|item| {
        item.rule_id == RuleId::SemanticDirectedHostility || item.points > HURTLEX_SCORE
    }));
}

#[test]
fn repeated_german_collision_matches_use_one_bounded_analysis() {
    let bytes = hurtlex(Language::De, &[("hund", "conservative")]);
    let channel =
        RuleChannel::from_hurtlex_bytes(Language::De, Some(&bytes)).expect("rule channel");
    let text = std::iter::repeat_n("du hund.", 512).collect::<String>();

    let result = channel.analyze(&text, ReplyTarget::Unknown);

    assert_eq!(result.score, HURTLEX_SCORE);
    assert!(!result.should_nudge);
    assert_eq!(
        result
            .evidence
            .iter()
            .filter(|item| item.rule_id == RuleId::LexicalMatch)
            .count(),
        512
    );
}

#[test]
fn every_complete_rule_identity_is_stable_and_distinct() {
    let expected = [
        (
            Language::En,
            "83f12c208705486045927869c1adc40d5987064de60cc5665bb24e5ee20f1bd3",
        ),
        (
            Language::Zh,
            "6faedfcb637f60f23a58ff24e3473023a2484707774fe90277cb217c9b3d7941",
        ),
        (
            Language::Es,
            "8bb5ad315f8abe69611cb192bfdf3712d8005cd331565547ec87573720a48246",
        ),
        (
            Language::Ar,
            "a882fa77392de6d327db51fc15a97729d63378df59b9ae564360e9b86aaff7ef",
        ),
        (
            Language::Ms,
            "32b05ecb070b353590ed6b2f29e4d6a13023ec4b33cfa7f48679f16475182861",
        ),
        (
            Language::Pt,
            "76b8c7927042582bee11d3c4444e1cf61c199782a223d1f95e60ea536c2a69aa",
        ),
        (
            Language::Fr,
            "d944d890212aefb86324d2d6dd4518a724f0e6de6daefee2706fb7be0e3fabc6",
        ),
        (
            Language::Hi,
            "eed30552d88d22ecc5cb33da64aaa4786299b78f046c3ff5fd0a1f744f6fb275",
        ),
        (
            Language::Ru,
            "2d3f7288619d2e801eeb2a52ea7243207752c08ee49c54098c70fb294c059b01",
        ),
        (
            Language::Ja,
            "8474720d2e8e0e85ba97e3a08f949dfa9b429b1538bff6a7b593bd32d6c9b2a1",
        ),
        (
            Language::De,
            "b48d46b1d9a84a9cf4781cd32d16f08acbeef426811919bdb6f870d52b837f96",
        ),
        (
            Language::Tr,
            "1ead20c976e68da4fdb683cdebbe680e40d7561bed08b807f595925fe663a932",
        ),
        (
            Language::Vi,
            "13fcb5ae0b82f081bd942acd347469290b369e39a159287a646ebcb4a835ff64",
        ),
        (
            Language::Ko,
            "51f02655fed2ffe57b000f4f557cf88ff03ee15fd5cb1e9b356b0fffae879a35",
        ),
        (
            Language::It,
            "c1e8cf9f3612964383114fe3108d5ae08c2d98c3662dceb24f56219c9dc8129e",
        ),
    ];

    for (language, expected_hash) in expected {
        let identity = canonical_rule_identity(language);
        assert!(identity.starts_with(b"TOXCHANNEL1"), "{}", language.code());
        assert_eq!(
            format!("{:x}", Sha256::digest(identity)),
            expected_hash,
            "{}",
            language.code()
        );
    }
}
