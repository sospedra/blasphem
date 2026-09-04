use std::process::Command;

struct MessageCase {
    language: &'static str,
    text: &'static str,
    expected_nudge: bool,
}

const CASES: &[MessageCase] = &[
    MessageCase {
        language: "EN",
        text: "When I find you, I will break every bone in your hand",
        expected_nudge: true,
    },
    MessageCase {
        language: "EN",
        text: "The teacher explained why the threatening message should be reported",
        expected_nudge: false,
    },
    MessageCase {
        language: "ZH",
        text: "等我找到你，我会打断你的双腿",
        expected_nudge: true,
    },
    MessageCase {
        language: "ZH",
        text: "管理员删除了那条威胁别人的消息",
        expected_nudge: false,
    },
    MessageCase {
        language: "ES",
        text: "Cuando salgas del edificio, voy a romperte todos los dientes",
        expected_nudge: true,
    },
    MessageCase {
        language: "ES",
        text: "No quiero que nadie vuelva a hacerte daño",
        expected_nudge: false,
    },
    MessageCase {
        language: "AR",
        text: "أتمنى أن تموت وحيدًا هذه الليلة",
        expected_nudge: true,
    },
    MessageCase {
        language: "AR",
        text: "أبلغت المشرف عن رسالة تهدد أحد الأعضاء",
        expected_nudge: false,
    },
    MessageCase {
        language: "ID",
        text: "Kalau bertemu lagi, akan kupatahkan rahangmu",
        expected_nudge: true,
    },
    MessageCase {
        language: "ID",
        text: "Saya melaporkan pesan yang mengancam pemain lain",
        expected_nudge: false,
    },
    MessageCase {
        language: "PT",
        text: "Espero que você nunca mais acorde",
        expected_nudge: true,
    },
    MessageCase {
        language: "PT",
        text: "Denunciei a mensagem que ameaçava outro jogador",
        expected_nudge: false,
    },
    MessageCase {
        language: "FR",
        text: "Quand je te trouverai, je te briserai les deux jambes",
        expected_nudge: true,
    },
    MessageCase {
        language: "FR",
        text: "J'ai signalé le message qui menaçait un autre joueur",
        expected_nudge: false,
    },
    MessageCase {
        language: "HI",
        text: "मैं तुम्हें ढूँढकर तुम्हारी हड्डियाँ तोड़ दूँगा",
        expected_nudge: true,
    },
    MessageCase {
        language: "HI",
        text: "मैंने दूसरे खिलाड़ी को धमकाने वाला संदेश रिपोर्ट किया",
        expected_nudge: false,
    },
    MessageCase {
        language: "RU",
        text: "Надеюсь, этой ночью ты больше не проснёшься",
        expected_nudge: true,
    },
    MessageCase {
        language: "RU",
        text: "Модератор удалил сообщение с угрозой другому игроку",
        expected_nudge: false,
    },
    MessageCase {
        language: "JA",
        text: "今度会ったら顔が分からなくなるまで殴る",
        expected_nudge: true,
    },
    MessageCase {
        language: "JA",
        text: "管理者は他の利用者を脅す投稿を削除した",
        expected_nudge: false,
    },
    MessageCase {
        language: "DE",
        text: "Ich hoffe, du wachst morgen nicht mehr auf",
        expected_nudge: true,
    },
    MessageCase {
        language: "DE",
        text: "Die Moderatorin löschte die Nachricht mit der Drohung",
        expected_nudge: false,
    },
    MessageCase {
        language: "TR",
        text: "Seni bulunca bütün dişlerini kıracağım",
        expected_nudge: true,
    },
    MessageCase {
        language: "TR",
        text: "Moderatör başka bir oyuncuyu tehdit eden mesajı sildi",
        expected_nudge: false,
    },
    MessageCase {
        language: "VI",
        text: "Tao sẽ tìm mày và đập gãy hết răng",
        expected_nudge: true,
    },
    MessageCase {
        language: "VI",
        text: "Quản trị viên đã xóa tin nhắn đe dọa người chơi khác",
        expected_nudge: false,
    },
    MessageCase {
        language: "KO",
        text: "오늘 밤 네가 다시는 깨어나지 않기를 바란다",
        expected_nudge: true,
    },
    MessageCase {
        language: "KO",
        text: "관리자가 다른 사용자를 협박한 메시지를 삭제했다",
        expected_nudge: false,
    },
    MessageCase {
        language: "IT",
        text: "Quando ti vedo, ti rompo tutti i denti",
        expected_nudge: true,
    },
    MessageCase {
        language: "IT",
        text: "La moderatrice ha cancellato il messaggio che minacciava un altro giocatore",
        expected_nudge: false,
    },
];

const GENERALIZATION_CASES: &[MessageCase] = &[
    MessageCase {
        language: "EN",
        text: "I will kill you",
        expected_nudge: true,
    },
    MessageCase {
        language: "EN",
        text: "The moderator reported a message saying \"I will kill you\"",
        expected_nudge: false,
    },
    MessageCase {
        language: "ZH",
        text: "我要杀了你",
        expected_nudge: true,
    },
    MessageCase {
        language: "ZH",
        text: "管理员报告了“我要杀了你”这条消息",
        expected_nudge: false,
    },
    MessageCase {
        language: "ES",
        text: "Te voy a matar",
        expected_nudge: true,
    },
    MessageCase {
        language: "ES",
        text: "La moderadora reportó el mensaje \"Te voy a matar\"",
        expected_nudge: false,
    },
    MessageCase {
        language: "AR",
        text: "سأقتلك",
        expected_nudge: true,
    },
    MessageCase {
        language: "AR",
        text: "أبلغ المشرف عن رسالة تقول \"سأقتلك\"",
        expected_nudge: false,
    },
    MessageCase {
        language: "ID",
        text: "aku akan membunuhmu",
        expected_nudge: true,
    },
    MessageCase {
        language: "ID",
        text: "Moderator melaporkan pesan \"aku akan membunuhmu\"",
        expected_nudge: false,
    },
    MessageCase {
        language: "PT",
        text: "vou te matar",
        expected_nudge: true,
    },
    MessageCase {
        language: "PT",
        text: "A moderadora denunciou a mensagem \"vou te matar\"",
        expected_nudge: false,
    },
    MessageCase {
        language: "FR",
        text: "je vais te tuer",
        expected_nudge: true,
    },
    MessageCase {
        language: "FR",
        text: "La modératrice a signalé le message « je vais te tuer »",
        expected_nudge: false,
    },
    MessageCase {
        language: "HI",
        text: "मैं तुम्हें मार दूंगा",
        expected_nudge: true,
    },
    MessageCase {
        language: "HI",
        text: "मॉडरेटर ने \"मैं तुम्हें मार दूंगा\" संदेश रिपोर्ट किया",
        expected_nudge: false,
    },
    MessageCase {
        language: "RU",
        text: "я тебя убью",
        expected_nudge: true,
    },
    MessageCase {
        language: "RU",
        text: "Модератор сообщил о сообщении «я тебя убью»",
        expected_nudge: false,
    },
    MessageCase {
        language: "JA",
        text: "お前を殺してやる",
        expected_nudge: true,
    },
    MessageCase {
        language: "JA",
        text: "管理者は「お前を殺してやる」という投稿を報告した",
        expected_nudge: false,
    },
    MessageCase {
        language: "DE",
        text: "ich werde dich töten",
        expected_nudge: true,
    },
    MessageCase {
        language: "DE",
        text: "Die Moderatorin meldete die Nachricht „ich werde dich töten“",
        expected_nudge: false,
    },
    MessageCase {
        language: "TR",
        text: "seni öldüreceğim",
        expected_nudge: true,
    },
    MessageCase {
        language: "TR",
        text: "Moderatör \"seni öldüreceğim\" mesajını bildirdi",
        expected_nudge: false,
    },
    MessageCase {
        language: "VI",
        text: "tao sẽ giết mày",
        expected_nudge: true,
    },
    MessageCase {
        language: "VI",
        text: "Quản trị viên báo cáo tin nhắn \"tao sẽ giết mày\"",
        expected_nudge: false,
    },
    MessageCase {
        language: "KO",
        text: "너를 죽이겠다",
        expected_nudge: true,
    },
    MessageCase {
        language: "KO",
        text: "관리자가 \"너를 죽이겠다\"라는 메시지를 신고했다",
        expected_nudge: false,
    },
    MessageCase {
        language: "IT",
        text: "ti ucciderò",
        expected_nudge: true,
    },
    MessageCase {
        language: "IT",
        text: "La moderatrice ha segnalato il messaggio \"ti ucciderò\"",
        expected_nudge: false,
    },
];

#[cfg(feature = "language-detection")]
const AUTO_LANGUAGE_CASES: &[(&str, &str, &str)] = &[
    (
        "EN",
        "I never should've bought that.",
        "ok=true score=48 threshold=50 should_nudge=false",
    ),
    (
        "ZH",
        "我想要确定什么都没有发生在汤姆身上。",
        "ok=true score=0 threshold=50 should_nudge=false",
    ),
    (
        "ES",
        "Was ist das? A ship. Todo bien en la costa.",
        "ok=true score=17 threshold=50 should_nudge=false",
    ),
    (
        "AR",
        "هل تحب الكتب؟",
        "ok=true score=5 threshold=50 should_nudge=false",
    ),
    (
        "MS",
        "Dia memberitahu saya yang dia benar-benar letih.",
        "ok=true score=37 threshold=50 should_nudge=false",
    ),
    (
        "PT",
        "Não vou chegar em casa até segunda.",
        "ok=true score=5 threshold=50 should_nudge=false",
    ),
    (
        "FR",
        "Bonjour le monde",
        "ok=true score=42 threshold=50 should_nudge=false",
    ),
    (
        "HI",
        "वह मेरे पिताजी की माँ है। वह मेरी दादी है।",
        "ok=true score=37 threshold=50 should_nudge=false",
    ),
    (
        "RU",
        "Они были здесь.",
        "ok=true score=49 threshold=50 should_nudge=false",
    ),
    (
        "JA",
        "私は２日間忙しくありません。",
        "ok=true score=0 threshold=50 should_nudge=false",
    ),
    (
        "DE",
        "Was ist das?",
        "ok=true score=0 threshold=50 should_nudge=false",
    ),
    (
        "TR",
        "Çok büyük bir musibet.",
        "ok=true score=7 threshold=50 should_nudge=false",
    ),
    (
        "VI",
        "Đây là 1 lời nói đùa cợt",
        "ok=true score=9 threshold=50 should_nudge=false",
    ),
    (
        "KO",
        "물이 별로 없다.",
        "ok=true score=18 threshold=50 should_nudge=false",
    ),
    (
        "IT",
        "La incontrerai domani sera.",
        "ok=true score=0 threshold=50 should_nudge=false",
    ),
];

#[test]
fn the_release_cli_warns_for_each_threat_and_allows_each_context_control() {
    run_cases(CASES);
}

#[test]
fn multilingual_rules_generalize_to_separate_threat_and_report_pairs() {
    run_cases(GENERALIZATION_CASES);
}

#[test]
#[cfg(feature = "language-detection")]
fn automatic_routing_resolves_all_canonical_languages() {
    for (language, text, expected_first_line) in AUTO_LANGUAGE_CASES {
        let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["check", "--language", "AUTO", "--text", text])
            .output()
            .expect("run blasphem");

        assert!(
            output.status.success(),
            "{language} {text:?} failed: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
        assert_known_auto_output(&stdout, expected_first_line, language);
    }
}

#[test]
#[cfg(feature = "language-detection")]
fn automatic_routing_fails_open_for_ambiguous_inputs() {
    for text in ["", "!@#$%^&*()", "😀🚀🧪❤️", "Hello"] {
        let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["check", "--language", "AUTO", "--text", text])
            .output()
            .expect("run blasphem");

        assert!(
            output.status.success(),
            "{text:?}: {}",
            String::from_utf8_lossy(&output.stderr),
        );
        assert_eq!(
            String::from_utf8(output.stdout).expect("UTF-8 output"),
            "ok=true score=0 threshold=50 should_nudge=false\nlanguage_mode=auto route=unknown detected_language=unknown reliable=false language_score=none evaluated=false\n",
            "{text:?}",
        );
    }
}

#[cfg(feature = "language-detection")]
fn assert_known_auto_output(stdout: &str, expected_first_line: &str, language: &str) {
    assert_eq!(stdout.lines().next(), Some(expected_first_line));

    let route = stdout.lines().nth(1).expect("routing line");
    let prefix = format!(
        "language_mode=auto route=known detected_language={language} reliable=true language_score="
    );
    let score = route
        .strip_prefix(&prefix)
        .and_then(|value| value.strip_suffix(" evaluated=true"))
        .expect("exact automatic known routing fields");
    let (whole, fractional) = score.split_once('.').expect("decimal language score");

    assert!(!whole.is_empty() && whole.chars().all(|character| character.is_ascii_digit()));
    assert_eq!(
        fractional.len(),
        4,
        "language score requires four decimal digits"
    );
    assert!(
        fractional
            .chars()
            .all(|character| character.is_ascii_digit()),
        "language score requires decimal digits"
    );
}

fn run_cases(cases: &[MessageCase]) {
    let mut failures = Vec::new();
    for case in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_blasphem"))
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["check", "--language", case.language, "--text", case.text])
            .output()
            .expect("run blasphem");

        if !output.status.success() {
            failures.push(format!(
                "{} {:?} failed: {}",
                case.language,
                case.text,
                String::from_utf8_lossy(&output.stderr),
            ));
            continue;
        }
        let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
        let expected = if case.expected_nudge {
            "ok=false"
        } else {
            "ok=true"
        };
        if !stdout.starts_with(expected) {
            failures.push(format!("{} {:?}: {}", case.language, case.text, stdout,));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
