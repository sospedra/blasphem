use charabia::Tokenize;

const LEXICAL_COLLISION_EXCLUSIONS: &[(&str, &[&str])] = &[
    ("AR", &["فتاة", "عامل", "تجمع"]),
    ("DE", &["volk", "rock", "hund", "dose"]),
    ("EN", &["do", "people", "know", "love", "proud"]),
    (
        "ES",
        &[
            "gente",
            "pueblo",
            "policía",
            "agente",
            "bomba",
            "curioso",
            "gay",
            "gitano",
            "negro",
            "retrasado",
            "servicio",
        ],
    ),
    ("FR", &["gens", "peuple", "police"]),
    ("IT", &["gente", "popolo", "sito", "arte"]),
    (
        "JA",
        &[
            "女", "問題", "下手", "平凡", "初心", "刑事", "婦警", "大衆", "庶民", "平民",
        ],
    ),
    (
        "KO",
        &[
            "김", "질", "의사", "작가", "정부", "음식", "날짜", "완전", "낚시", "경찰", "경관",
            "여경", "대중", "민중", "서민",
        ],
    ),
    ("PT", &["gente", "povo", "polícia"]),
    ("RU", &["девушка", "член", "юбка"]),
    (
        "ZH",
        &[
            "和", "日", "警察", "警官", "民众", "群众", "刑事", "笔者", "困难", "同性", "暴力",
            "反派",
        ],
    ),
];

const LEXICAL_COLLISION_REACTIVATIONS: &[(&str, &str, &[&str])] =
    &[("DE", "hund", &["du", "hund"])];

pub(crate) struct RulePack {
    pub(crate) language: &'static str,
    pub(crate) targets: Vec<Vec<String>>,
    pub(crate) groups: Vec<Vec<String>>,
    pub(crate) identity_links: Vec<Vec<String>>,
    pub(crate) negators: Vec<Vec<String>>,
    pub(crate) threats: Vec<Vec<String>>,
    pub(crate) intent: Vec<Vec<String>>,
    pub(crate) reports: Vec<Vec<String>>,
    pub(crate) counterspeech: Vec<Vec<String>>,
    pub(crate) positive: Vec<Vec<String>>,
    pub(crate) negative: Vec<Vec<String>>,
    pub(crate) intensifiers: Vec<Vec<String>>,
    pub(crate) diminishers: Vec<Vec<String>>,
    pub(crate) semantic: SemanticPack,
}

#[derive(Default)]
pub(crate) struct SemanticPack {
    pub(crate) implicit_targets: Vec<Vec<String>>,
    pub(crate) implicit_threats: Vec<Vec<String>>,
    pub(crate) self_harm_commands: Vec<Vec<String>>,
    pub(crate) wish_markers: Vec<Vec<String>>,
    pub(crate) clause_initial_wish_markers: Vec<Vec<String>>,
    pub(crate) harm_outcomes: Vec<Vec<String>>,
    pub(crate) implicit_harm_outcomes: Vec<Vec<String>>,
    pub(crate) benign_harm_phrases: Vec<Vec<String>>,
    pub(crate) long_scope_negators: Vec<Vec<String>>,
    pub(crate) directed_hostility: Vec<Vec<String>>,
    pub(crate) group_hostility: Vec<Vec<String>>,
}

impl RulePack {
    #[allow(clippy::too_many_arguments)]
    fn new(
        language: &'static str,
        targets: &[&str],
        groups: &[&str],
        identity_links: &[&str],
        negators: &[&str],
        threats: &[&str],
        intent: &[&str],
        reports: &[&str],
        counterspeech: &[&str],
        positive: &[&str],
        negative: &[&str],
        intensifiers: &[&str],
        diminishers: &[&str],
    ) -> Self {
        Self {
            language,
            targets: normalize_phrases(targets),
            groups: normalize_phrases(groups),
            identity_links: normalize_phrases(identity_links),
            negators: normalize_phrases(negators),
            threats: normalize_phrases(threats),
            intent: normalize_phrases(intent),
            reports: normalize_phrases(reports),
            counterspeech: normalize_phrases(counterspeech),
            positive: normalize_phrases(positive),
            negative: normalize_phrases(negative),
            intensifiers: normalize_phrases(intensifiers),
            diminishers: normalize_phrases(diminishers),
            semantic: SemanticPack::default(),
        }
    }

    fn with_semantic(mut self, semantic: SemanticPack) -> Self {
        self.semantic = semantic;
        self
    }
}

fn spanish_semantic_pack() -> SemanticPack {
    SemanticPack {
        implicit_targets: normalize_phrases(&[
            "eres", "sois", "estas", "estais", "sos", "pareces", "pareceis",
        ]),
        implicit_threats: normalize_phrases(&[
            "romperte la cara",
            "romperte los dientes",
            "romperte todos los dientes",
            "partirte los dientes",
            "hacerte daño",
            "violarte",
            "apuñalarte",
            "dispararte",
            "pegarte",
            "golpearte",
        ]),
        self_harm_commands: normalize_phrases(&[
            "matate",
            "muerete",
            "suicidate",
            "tirate por un puente",
        ]),
        wish_markers: normalize_phrases(&[
            "ojala",
            "espero que",
            "deseo que",
            "quisiera que",
            "deberia",
        ]),
        clause_initial_wish_markers: normalize_phrases(&["que"]),
        harm_outcomes: normalize_phrases(&[
            "muera",
            "mueran",
            "se muera",
            "se mueran",
            "sufra",
            "sufran",
            "desaparezca",
            "desaparezcan",
            "pierda todo",
            "pierdan todo",
            "morir",
        ]),
        implicit_harm_outcomes: normalize_phrases(&[
            "mueras",
            "sufras",
            "desaparezcas",
            "pierdas todo",
        ]),
        benign_harm_phrases: normalize_phrases(&[
            "matar de risa",
            "matar el tiempo",
            "matar dos pajaros de un tiro",
        ]),
        long_scope_negators: normalize_phrases(&[
            "no quiero que",
            "no queremos que",
            "no deseo que",
            "no deseamos que",
        ]),
        directed_hostility: normalize_phrases(&[
            "das asco",
            "no vales nada",
            "nadie te quiere",
            "nadie te soporta",
            "no te soporto",
            "callate",
            "vete a la mierda",
            "largate",
        ]),
        group_hostility: normalize_phrases(&[
            "plaga",
            "inferior",
            "inferiores",
            "dan asco",
            "sobran",
        ]),
    }
}

fn focused_semantic_pack(
    implicit_threats: &[&str],
    wish_markers: &[&str],
    harm_outcomes: &[&str],
    implicit_harm_outcomes: &[&str],
) -> SemanticPack {
    SemanticPack {
        implicit_threats: normalize_phrases(implicit_threats),
        wish_markers: normalize_phrases(wish_markers),
        harm_outcomes: normalize_phrases(harm_outcomes),
        implicit_harm_outcomes: normalize_phrases(implicit_harm_outcomes),
        ..SemanticPack::default()
    }
}

fn english_semantic_pack() -> SemanticPack {
    let mut pack = focused_semantic_pack(&[], &[], &[], &[]);
    pack.benign_harm_phrases = normalize_phrases(&[
        "kill you with laughter",
        "kill you laughing",
        "kill this process",
        "kill that process",
        "kill the process",
        "kill a process",
        "kill your process",
    ]);
    pack
}

fn normalize_phrases(phrases: &[&str]) -> Vec<Vec<String>> {
    phrases
        .iter()
        .map(|phrase| {
            phrase
                .tokenize()
                .filter(|token| token.is_word() || token.is_stopword())
                .map(|token| token.lemma().to_owned())
                .collect()
        })
        .collect()
}

pub(crate) fn lexical_collision_excluded(language: &str, lemma: &str) -> bool {
    let exclusions = LEXICAL_COLLISION_EXCLUSIONS
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(language.trim()))
        .map(|(_, exclusions)| *exclusions);
    let (Some(exclusions), Some(normalized)) = (exclusions, normalize_exact_lemma(lemma)) else {
        return false;
    };

    exclusions
        .iter()
        .any(|exclusion| normalize_exact_lemma(exclusion).as_deref() == Some(normalized.as_str()))
}

pub(crate) fn lexical_collision_exclusions(language: &str) -> &'static [&'static str] {
    LEXICAL_COLLISION_EXCLUSIONS
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(language.trim()))
        .map_or(&[], |(_, exclusions)| *exclusions)
}

pub(crate) fn lexical_collision_reactivation_phrase(
    language: &str,
    lemma: &str,
) -> Option<&'static [&'static str]> {
    if !LEXICAL_COLLISION_REACTIVATIONS
        .iter()
        .any(|(candidate, _, _)| candidate.eq_ignore_ascii_case(language.trim()))
    {
        return None;
    }
    let normalized = normalize_exact_lemma(lemma)?;

    LEXICAL_COLLISION_REACTIVATIONS
        .iter()
        .find(|(candidate_language, candidate_lemma, _)| {
            candidate_language.eq_ignore_ascii_case(language.trim())
                && normalize_exact_lemma(candidate_lemma).as_deref() == Some(normalized.as_str())
        })
        .map(|(_, _, phrase)| *phrase)
}

fn normalize_exact_lemma(lemma: &str) -> Option<String> {
    let normalized = lemma
        .tokenize()
        .filter(|token| token.is_word() || token.is_stopword())
        .map(|token| token.lemma().to_owned())
        .collect::<Vec<_>>();
    let [normalized] = normalized.as_slice() else {
        return None;
    };
    Some(normalized.clone())
}

macro_rules! pack {
    ($language:literal, $targets:expr, $groups:expr, $identity_links:expr, $negators:expr,
     $threats:expr, $intent:expr, $reports:expr, $counterspeech:expr, $positive:expr,
     $negative:expr, $intensifiers:expr, $diminishers:expr) => {
        RulePack::new(
            $language,
            $targets,
            $groups,
            $identity_links,
            $negators,
            $threats,
            $intent,
            $reports,
            $counterspeech,
            $positive,
            $negative,
            $intensifiers,
            $diminishers,
        )
    };
}

pub(crate) fn for_language(language: &str) -> Option<RulePack> {
    Some(match language.trim().to_ascii_uppercase().as_str() {
        "EN" => pack!(
            "EN",
            &["you", "your", "yours", "yourself"],
            &[
                "immigrants",
                "women",
                "men",
                "muslims",
                "jews",
                "christians",
                "gays",
                "lesbians",
                "trans",
                "blacks",
                "whites",
                "disabled"
            ],
            &[
                "is", "are", "was", "were", "is a", "are all", "was a", "were all"
            ],
            &["no", "not", "never", "dont", "don't", "do not"],
            &[
                "kill",
                "hurt",
                "attack",
                "murder",
                "shoot",
                "beat",
                "break every bone",
                "break your bones",
                "break your hand",
                "break your legs",
                "break your jaw",
                "break your teeth",
                "smash your face",
                "stab",
            ],
            &["will", "gonna", "going"],
            &["said", "says", "called", "reported", "quoted", "explained",],
            &["wrong", "stop", "condemn", "condemns", "against"],
            &["good", "great", "love", "kind", "beautiful"],
            &["awful", "horrible", "disgusting", "hate", "evil"],
            &["very", "really", "so", "extremely"],
            &["slightly", "somewhat", "maybe"]
        )
        .with_semantic(english_semantic_pack()),
        "ES" => pack!(
            "ES",
            &[
                "tú",
                "tu",
                "usted",
                "ustedes",
                "vosotros",
                "te",
                "ti",
                "tu familia",
                "toda tu familia",
                "tu hijo",
                "tu hija",
                "tus hijos",
                "tu madre",
                "tu padre"
            ],
            &[
                "inmigrantes",
                "mujeres",
                "hombres",
                "musulmanes",
                "judíos",
                "gais",
                "lesbianas",
                "trans",
                "discapacitados"
            ],
            &[
                "es", "son", "era", "eran", "es un", "es una", "son unos", "son unas"
            ],
            &["no", "nunca", "jamás"],
            &[
                "matar",
                "mato",
                "mataré",
                "herir",
                "atacar",
                "golpear",
                "violar",
                "rompo la cara",
                "pego un tiro",
                "apuñalo",
                "destrozar",
                "partire los dientes",
                "manosear"
            ],
            &[
                "voy",
                "vas",
                "va",
                "vamos",
                "vais",
                "van",
                "rompo la cara",
                "pego un tiro",
                "apuñalo",
                "partire los dientes",
            ],
            &["dijo", "dice", "llamó", "reportó", "citó"],
            &[
                "mal", "para", "condeno", "condena", "contra", "falso", "falsa",
            ],
            &["bueno", "genial", "amor", "amable", "bonito"],
            &["horrible", "asqueroso", "odio", "malo", "malvado"],
            &["muy", "realmente", "tan"],
            &["algo", "quizá", "quizás"]
        )
        .with_semantic(spanish_semantic_pack()),
        "FR" => pack!(
            "FR",
            &["tu", "toi", "vous", "votre", "tes", "te"],
            &[
                "immigrés",
                "femmes",
                "hommes",
                "musulmans",
                "juifs",
                "gays",
                "lesbiennes",
                "trans",
                "handicapés"
            ],
            &[
                "est",
                "sont",
                "était",
                "étaient",
                "est un",
                "est une",
                "sont des",
                "est de la",
                "sont de la"
            ],
            &["ne", "pas", "jamais", "aucun"],
            &[
                "tuer", "tue", "tuerai", "blesser", "attaquer", "frapper", "briser", "briserai",
            ],
            &[
                "vais", "vas", "va", "allons", "allez", "vont", "tuerai", "briserai",
            ],
            &["dit", "appelé", "rapporté", "cité", "signalé", "menaçait",],
            &["mal", "arrête", "condamne", "contre"],
            &["bon", "génial", "amour", "gentil", "beau"],
            &["horrible", "dégoûtant", "haine", "mauvais"],
            &["très", "vraiment", "si"],
            &["légèrement", "peut-être"]
        ),
        "DE" => pack!(
            "DE",
            &["du", "dich", "ihr", "euch", "sie"],
            &[
                "immigranten",
                "frauen",
                "männer",
                "muslime",
                "juden",
                "schwule",
                "lesben",
                "trans",
                "behinderte"
            ],
            &["ist", "sind", "war", "waren"],
            &["nicht", "nie", "kein", "keine"],
            &["töten", "umbringen", "verletzen", "angreifen", "schlagen"],
            &["werde", "wirst", "wird", "werden"],
            &[
                "sagte",
                "sagt",
                "nannte",
                "berichtete",
                "zitierte",
                "meldete",
                "löschte",
            ],
            &["falsch", "stopp", "verurteile", "gegen"],
            &["gut", "toll", "liebe", "nett", "schön"],
            &["schrecklich", "ekelhaft", "hass", "böse"],
            &["sehr", "wirklich", "so"],
            &["etwas", "vielleicht"]
        )
        .with_semantic(focused_semantic_pack(
            &[],
            &["ich hoffe", "hoffentlich"],
            &[
                "wachst morgen nicht mehr auf",
                "wachst nicht mehr auf",
                "stirbst",
            ],
            &[],
        )),
        "IT" => pack!(
            "IT",
            &["tu", "te", "voi", "tua", "tuo", "ti"],
            &[
                "immigrati",
                "donne",
                "uomini",
                "musulmani",
                "ebrei",
                "gay",
                "lesbiche",
                "trans",
                "disabili"
            ],
            &[
                "è",
                "sono",
                "era",
                "erano",
                "è un",
                "è una",
                "sono dei",
                "sono degli",
                "sono delle"
            ],
            &["non", "mai", "nessun"],
            &[
                "uccidere",
                "ucciderò",
                "ammazzare",
                "ferire",
                "attaccare",
                "colpire",
                "rompo tutti i denti"
            ],
            &["voglio", "vai", "sta", "ucciderò", "rompo tutti i denti"],
            &[
                "disse",
                "dice",
                "chiamò",
                "riferì",
                "citò",
                "segnalato",
                "cancellato",
                "minacciava",
            ],
            &["sbagliato", "ferma", "condanno", "contro"],
            &["buono", "grande", "amore", "gentile", "bello"],
            &["orribile", "disgustoso", "odio", "cattivo"],
            &["molto", "davvero", "così"],
            &["poco", "forse"]
        ),
        "PT" => pack!(
            "PT",
            &["você", "vocês", "tu", "te", "ti", "seu", "sua"],
            &[
                "imigrantes",
                "mulheres",
                "homens",
                "muçulmanos",
                "judeus",
                "gays",
                "lésbicas",
                "trans",
                "deficientes"
            ],
            &[
                "é",
                "são",
                "era",
                "eram",
                "é um",
                "é uma",
                "são uns",
                "são umas"
            ],
            &["não", "nunca", "jamais", "nenhum"],
            &["matar", "mato", "matarei", "ferir", "atacar", "bater"],
            &["vou", "vai", "vamos", "vão"],
            &[
                "disse",
                "diz",
                "chamou",
                "relatou",
                "citou",
                "denunciei",
                "denunciou",
                "ameaçava",
            ],
            &["errado", "pare", "condeno", "contra"],
            &["bom", "ótimo", "amor", "gentil", "bonito"],
            &["horrível", "nojo", "ódio", "mau"],
            &["muito", "realmente", "tão"],
            &["pouco", "talvez"]
        )
        .with_semantic(focused_semantic_pack(
            &[],
            &["espero que", "tomara que", "desejo que"],
            &["morra", "nunca mais acorde", "não acorde mais"],
            &[],
        )),
        "RU" => pack!(
            "RU",
            &["ты", "тебя", "тебе", "вы", "вас", "вам"],
            &[
                "иммигранты",
                "женщины",
                "мужчины",
                "мусульмане",
                "евреи",
                "геи",
                "лесбиянки",
                "транс",
                "инвалиды"
            ],
            &["это", "является", "являются", "был", "были"],
            &["не", "нет", "никогда"],
            &["убить", "убью", "убьем", "ранить", "атаковать", "ударить"],
            &["буду", "будешь", "будет", "убью", "убьем"],
            &[
                "сказал",
                "говорит",
                "назвал",
                "сообщил",
                "цитировал",
                "удалил",
                "угрозой",
            ],
            &["неправильно", "стоп", "осуждаю", "против"],
            &["хорошо", "отлично", "любовь", "добрый", "красивый"],
            &["ужасный", "отвратительный", "ненавижу", "злой"],
            &["очень", "реально", "так"],
            &["немного", "может"]
        )
        .with_semantic(focused_semantic_pack(
            &[],
            &["надеюсь", "чтоб", "пусть"],
            &["больше не проснёшься", "не проснёшься", "сдох"],
            &[],
        )),
        "AR" => pack!(
            "AR",
            &["أنت", "انتم", "أنتم", "انت", "أنتِ"],
            &[
                "مهاجرون",
                "نساء",
                "رجال",
                "مسلمون",
                "يهود",
                "مثليون",
                "متحولون",
                "معاقون"
            ],
            &["هو", "هي", "هم", "هن", "كان", "كانت", "كانوا"],
            &["لا", "لن", "ليس", "أبدا"],
            &["سأقتلك", "أقتل", "اقتل", "أؤذي", "أهاجم", "أضرب"],
            &["سوف", "سأ"],
            &["قال", "يقول", "سمى", "أبلغ", "أبلغت", "اقتبس"],
            &["خطأ", "توقف", "أدين", "ضد"],
            &["جيد", "رائع", "حب", "لطيف", "جميل"],
            &["فظيع", "مقرف", "أكره", "شرير"],
            &["جدا", "حقا"],
            &["قليلا", "ربما"]
        )
        .with_semantic(focused_semantic_pack(
            &["سأقتلك", "سوف أقتلك", "راح أقتلك"],
            &["أتمنى أن", "أتمنى"],
            &[],
            &["تموت", "تموتوا"],
        )),
        "MS" | "ID" => pack!(
            "MS",
            &["kamu", "kau", "anda", "kalian"],
            &[],
            &[],
            &["tidak", "jangan", "bukan"],
            &["bunuh", "membunuh", "pukul", "serang"],
            &["akan", "mau", "bakal"],
            &["melaporkan", "dilaporkan", "laporan"],
            &["hentikan", "mengecam", "jangan"],
            &[],
            &[],
            &[],
            &[]
        )
        .with_semantic(focused_semantic_pack(
            &[
                "membunuhmu",
                "kupatahkan rahangmu",
                "patahkan rahangmu",
                "kupatahkan tulangmu",
            ],
            &[],
            &[],
            &[],
        )),
        "HI" => pack!(
            "HI",
            &["तुम", "तुम्हें", "तुम्हारी", "तू", "तुझे", "आप"],
            &[],
            &[],
            &["नहीं", "मत", "कभी नहीं"],
            &["मार दूंगा", "मार दूँगा", "तोड़ दूंगा", "तोड़ दूँगा", "मारना",],
            &["मैं", "करूंगा", "करूँगा"],
            &["रिपोर्ट", "रिपोर्ट किया"],
            &["रुको", "गलत", "निंदा"],
            &[],
            &[],
            &[],
            &[]
        )
        .with_semantic(focused_semantic_pack(
            &[
                "तुम्हें मार दूंगा",
                "तुम्हें मार दूँगा",
                "तुम्हारी हड्डियाँ तोड़ दूंगा",
                "तुम्हारी हड्डियाँ तोड़ दूँगा",
            ],
            &[],
            &[],
            &[],
        )),
        "TR" => pack!(
            "TR",
            &["sen", "seni", "sana", "siz", "sizi"],
            &[],
            &[],
            &["değil", "hayır", "asla", "sakın"],
            &["öldüreceğim", "kıracağım", "vuracağım", "döveceğim",],
            &[
                "ben",
                "bulunca",
                "görünce",
                "öldüreceğim",
                "kıracağım",
                "vuracağım",
                "döveceğim",
            ],
            &["bildirdi", "raporladı", "sildi"],
            &["dur", "yanlış", "kınıyorum"],
            &[],
            &[],
            &[],
            &[]
        ),
        "VI" => pack!(
            "VI",
            &["mày", "bạn", "mi", "ngươi", "chúng mày"],
            &[],
            &[],
            &["không", "đừng", "chẳng"],
            &["giết", "đập gãy", "đánh", "đâm"],
            &["sẽ tìm", "sẽ", "tao", "tôi"],
            &["báo cáo", "xóa"],
            &["dừng", "sai", "lên án"],
            &[],
            &[],
            &[],
            &[]
        ),
        "ZH" => pack!(
            "ZH",
            &["你", "你们", "你家人", "你全家"],
            &[],
            &[],
            &["不", "不会", "别"],
            &[],
            &[],
            &["删除", "举报", "报告", "封禁"],
            &["停止", "反对"],
            &[],
            &[],
            &[],
            &[]
        ),
        "JA" => pack!(
            "JA",
            &["お前", "あなた", "あんた", "君", "てめえ", "貴様"],
            &[],
            &[],
            &["ない", "しない"],
            &[],
            &[],
            &["削除", "通報", "報告", "凍結"],
            &["やめろ", "反対"],
            &[],
            &[],
            &[],
            &[]
        ),
        "KO" => pack!(
            "KO",
            &["너", "네가", "널", "너를", "니가", "당신", "너희"],
            &[],
            &[],
            &["않아", "아니다", "말아"],
            &[],
            &[],
            &["삭제", "신고", "차단"],
            &["하지 마", "반대"],
            &[],
            &[],
            &[],
            &[]
        ),
        _ => return None,
    })
}
