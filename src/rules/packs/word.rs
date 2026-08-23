use crate::Language;

use super::super::{LanguageRules, PhraseSet, RuleMatchProfile};

const EMPTY: PhraseSet = PhraseSet::empty();

const EN_RULES: LanguageRules = LanguageRules {
    language: Language::En,
    version: 1,
    targets: PhraseSet::new(&["you", "your hand"]),
    harm_predicates: PhraseSet::new(&["kill", "break every bone", "beat the shit out of"]),
    intent_markers: PhraseSet::new(&["i will", "i ll"]),
    implicit_target_threats: EMPTY,
    wish_markers: PhraseSet::new(&["i hope"]),
    harm_outcomes: PhraseSet::new(&["die"]),
    implicit_target_harm_wishes: EMPTY,
    self_harm_commands: PhraseSet::new(&["kill yourself"]),
    strong_insults: PhraseSet::new(&["piece of shit", "moron"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["awful"]),
    copulas_or_vocatives: PhraseSet::new(&["are"]),
    negators: PhraseSet::new(&["not"]),
    reports: PhraseSet::new(&["wrote"]),
    counterspeech_markers: PhraseSet::new(&["do not say", "stop saying"]),
    proposition_boundaries: PhraseSet::new(&["but"]),
    matching: RuleMatchProfile::WordClauses,
};

const MS_RULES: LanguageRules = LanguageRules {
    language: Language::Ms,
    version: 1,
    targets: PhraseSet::new(&["kamu", "kau", "lo", "lu", "rahangmu"]),
    harm_predicates: PhraseSet::new(&["kupatahkan"]),
    intent_markers: PhraseSet::new(&["akan"]),
    implicit_target_threats: PhraseSet::new(&[
        "aku akan membunuhmu",
        "user gaya bgt lo kampang mau gue tendang gak",
    ]),
    wish_markers: PhraseSet::new(&["semoga"]),
    harm_outcomes: PhraseSet::new(&["mati"]),
    implicit_target_harm_wishes: PhraseSet::new(&[
        "perempuan kaya lo mending mati aja deh jelek aja gausa sok jadi make up artist",
    ]),
    self_harm_commands: PhraseSet::new(&["mati saja kau", "bunuh diri saja"]),
    strong_insults: PhraseSet::new(&["bajingan", "tolol"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["buruk"]),
    copulas_or_vocatives: PhraseSet::new(&["adalah"]),
    negators: PhraseSet::new(&["tidak akan", "tidak"]),
    reports: PhraseSet::new(&["menulis"]),
    counterspeech_markers: PhraseSet::new(&["jangan katakan", "berhenti mengatakan"]),
    proposition_boundaries: PhraseSet::new(&["tapi"]),
    matching: RuleMatchProfile::WordClauses,
};

const PT_RULES: LanguageRules = LanguageRules {
    language: Language::Pt,
    version: 1,
    targets: PhraseSet::new(&["te", "você", "voce", "user"]),
    harm_predicates: PhraseSet::new(&["matar"]),
    intent_markers: PhraseSet::new(&["vou"]),
    implicit_target_threats: PhraseSet::new(&[
        "user porra porto te bato se fizer qualquer tatuagem",
    ]),
    wish_markers: PhraseSet::new(&["espero que", "tomara que"]),
    harm_outcomes: PhraseSet::new(&["nunca mais acorde", "morra"]),
    implicit_target_harm_wishes: PhraseSet::new(&["fale uma musica ruim do 21 pilots e morra fdp"]),
    self_harm_commands: PhraseSet::new(&["mate se", "se mata"]),
    strong_insults: PhraseSet::new(&["filho da puta", "burro"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["ruim"]),
    copulas_or_vocatives: PhraseSet::new(&["é", "e"]),
    negators: PhraseSet::new(&["não vou te", "não", "nao"]),
    reports: PhraseSet::new(&["escreveu"]),
    counterspeech_markers: PhraseSet::new(&["não diga", "pare de dizer"]),
    proposition_boundaries: PhraseSet::new(&["mas"]),
    matching: RuleMatchProfile::WordClauses,
};

const FR_RULES: LanguageRules = LanguageRules {
    language: Language::Fr,
    version: 1,
    targets: PhraseSet::new(&["te", "tu"]),
    harm_predicates: PhraseSet::new(&["briserai", "tuer", "tuerais"]),
    intent_markers: PhraseSet::new(&["je vais", "je te"]),
    implicit_target_threats: EMPTY,
    wish_markers: PhraseSet::new(&["j espere que", "je pense que"]),
    harm_outcomes: PhraseSet::new(&["creves", "crever"]),
    implicit_target_harm_wishes: PhraseSet::new(&["je pense que tu ferais mieux d aller crever"]),
    self_harm_commands: PhraseSet::new(&["tue toi", "va te suicider"]),
    strong_insults: PhraseSet::new(&["ordure", "imbecile"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["mauvais"]),
    copulas_or_vocatives: PhraseSet::new(&["es"]),
    negators: PhraseSet::new(&["ne", "pas"]),
    reports: PhraseSet::new(&["a ecrit"]),
    counterspeech_markers: PhraseSet::new(&["ne dis pas", "arrete de dire"]),
    proposition_boundaries: PhraseSet::new(&["mais"]),
    matching: RuleMatchProfile::WordClauses,
};

const RU_RULES: LanguageRules = LanguageRules {
    language: Language::Ru,
    version: 1,
    targets: PhraseSet::new(&["тебя", "тебе", "ты", "себя"]),
    harm_predicates: PhraseSet::new(&["убью", "сломаю"]),
    intent_markers: PhraseSet::new(&["я"]),
    implicit_target_threats: EMPTY,
    wish_markers: PhraseSet::new(&["надеюсь", "чтоб"]),
    harm_outcomes: PhraseSet::new(&["больше не просне\u{308}шься", "сдох"]),
    implicit_target_harm_wishes: PhraseSet::new(&[
        "надеюсь этои\u{306} ночью ты больше не просне\u{308}шься",
    ]),
    self_harm_commands: PhraseSet::new(&["убеи\u{306} себя", "застрелись"]),
    strong_insults: PhraseSet::new(&["мразь", "идиотка"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["ужасный"]),
    copulas_or_vocatives: EMPTY,
    negators: PhraseSet::new(&["тебя не", "ты не"]),
    reports: PhraseSet::new(&["написал"]),
    counterspeech_markers: PhraseSet::new(&["не говори", "перестань говорить"]),
    proposition_boundaries: PhraseSet::new(&["но"]),
    matching: RuleMatchProfile::WordClauses,
};

const DE_RULES: LanguageRules = LanguageRules {
    language: Language::De,
    version: 1,
    targets: PhraseSet::new(&["dich", "dir", "du", "ihr"]),
    harm_predicates: PhraseSet::new(&["toten", "breche ich"]),
    intent_markers: PhraseSet::new(&["ich werde", "dann"]),
    implicit_target_threats: EMPTY,
    wish_markers: PhraseSet::new(&["ich hoffe", "hoffentlich"]),
    harm_outcomes: PhraseSet::new(&["wachst morgen nicht mehr auf", "stirbst"]),
    implicit_target_harm_wishes: PhraseSet::new(&["verrecke elendig"]),
    self_harm_commands: PhraseSet::new(&["bring dich um", "tote dich selbst"]),
    strong_insults: PhraseSet::new(&["arschloch", "verlogenen arschlocher"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["schlecht"]),
    copulas_or_vocatives: PhraseSet::new(&["bist", "seit"]),
    negators: PhraseSet::new(&["werde dich nicht", "du nicht"]),
    reports: PhraseSet::new(&["schrieb"]),
    counterspeech_markers: PhraseSet::new(&["sag nicht", "hor auf zu sagen"]),
    proposition_boundaries: PhraseSet::new(&["aber"]),
    matching: RuleMatchProfile::WordClauses,
};

const TR_RULES: LanguageRules = LanguageRules {
    language: Language::Tr,
    version: 1,
    targets: PhraseSet::new(&["seni", "dişlerini", "senin", "orospu", "sen", "user"]),
    harm_predicates: PhraseSet::new(&["kıracağım", "kafanı kırarım"]),
    intent_markers: PhraseSet::new(&["seni bulunca"]),
    implicit_target_threats: PhraseSet::new(&[
        "seni öldüreceğim",
        "seni bulunca bütün dişlerini kıracağım",
    ]),
    wish_markers: PhraseSet::new(&["en yakın zamanda"]),
    harm_outcomes: PhraseSet::new(&["geber"]),
    implicit_target_harm_wishes: PhraseSet::new(&["umarım ölürsün"]),
    self_harm_commands: PhraseSet::new(&["kendini öldür", "intihar ederim nolcak et hadi madem"]),
    strong_insults: PhraseSet::new(&["şerefsizsin", "geri zekalı"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["kötü"]),
    copulas_or_vocatives: EMPTY,
    negators: EMPTY,
    reports: PhraseSet::new(&["yazdı"]),
    counterspeech_markers: PhraseSet::new(&["bunu söyleme", "şunu söylemeyi bırak"]),
    proposition_boundaries: PhraseSet::new(&["ama", "fakat"]),
    matching: RuleMatchProfile::WordClauses,
};

const VI_RULES: LanguageRules = LanguageRules {
    language: Language::Vi,
    version: 1,
    targets: PhraseSet::new(&["mày"]),
    harm_predicates: PhraseSet::new(&["đập gãy", "giết"]),
    intent_markers: PhraseSet::new(&["tao sẽ", "t đi tìm ra"]),
    implicit_target_threats: EMPTY,
    wish_markers: PhraseSet::new(&["mong"]),
    harm_outcomes: PhraseSet::new(&["chết"]),
    implicit_target_harm_wishes: PhraseSet::new(&["đcmm đi chết đi"]),
    self_harm_commands: PhraseSet::new(&["tự tử đi", "mày tự sát đi"]),
    strong_insults: PhraseSet::new(&["đồ chó", "ngu"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: PhraseSet::new(&["tệ"]),
    copulas_or_vocatives: PhraseSet::new(&["là"]),
    negators: PhraseSet::new(&["sẽ không", "không mong", "không"]),
    reports: PhraseSet::new(&["viết"]),
    counterspeech_markers: PhraseSet::new(&["đừng nói", "đừng gọi"]),
    proposition_boundaries: PhraseSet::new(&["nhưng"]),
    matching: RuleMatchProfile::WordClauses,
};

const IT_RULES: LanguageRules = LanguageRules {
    language: Language::It,
    version: 2,
    targets: PhraseSet::new(&["ti", "tu"]),
    harm_predicates: PhraseSet::new(&["rompo", "ammazzo"]),
    intent_markers: PhraseSet::new(&["quando ti vedo", "se ti becco a san siro"]),
    implicit_target_threats: PhraseSet::new(&["ti uccidero"]),
    wish_markers: PhraseSet::new(&["spero che"]),
    harm_outcomes: PhraseSet::new(&["muoia"]),
    implicit_target_harm_wishes: PhraseSet::new(&["muori pezzente"]),
    self_harm_commands: PhraseSet::new(&["ammazzati"]),
    strong_insults: EMPTY,
    implicit_target_directed_insults: PhraseSet::new(&["sei una merda", "hellis sei un cretino"]),
    negative_sentiment: PhraseSet::new(&["orribile"]),
    copulas_or_vocatives: PhraseSet::new(&["sei"]),
    negators: PhraseSet::new(&["non ti", "tu non"]),
    reports: PhraseSet::new(&["ha scritto"]),
    counterspeech_markers: PhraseSet::new(&["non dire", "smetti di dire"]),
    proposition_boundaries: PhraseSet::new(&["ma", "pero"]),
    matching: RuleMatchProfile::WordClauses,
};

/// Returns the static semantic rules for a word-script language.
#[must_use]
pub fn word_rules(language: Language) -> Option<&'static LanguageRules> {
    match language {
        Language::En => Some(&EN_RULES),
        Language::Ms => Some(&MS_RULES),
        Language::Pt => Some(&PT_RULES),
        Language::Fr => Some(&FR_RULES),
        Language::Ru => Some(&RU_RULES),
        Language::De => Some(&DE_RULES),
        Language::Tr => Some(&TR_RULES),
        Language::Vi => Some(&VI_RULES),
        Language::It => Some(&IT_RULES),
        Language::Zh | Language::Es | Language::Ar | Language::Hi | Language::Ja | Language::Ko => {
            None
        }
    }
}
