use crate::Language;

use super::super::{LanguageRules, PhraseSet, RuleMatchProfile};

const EMPTY: PhraseSet = PhraseSet::empty();

const ZH_RULES: LanguageRules = LanguageRules {
    language: Language::Zh,
    version: 1,
    targets: PhraseSet::new(&["你", "你们", "你家人", "你全家", "女权"]),
    harm_predicates: PhraseSet::new(&["杀光"]),
    intent_markers: EMPTY,
    implicit_target_threats: PhraseSet::new(&["我要杀了你", "等我找到你我会打断你的双腿"]),
    wish_markers: PhraseSet::new(&["希望"]),
    harm_outcomes: PhraseSet::new(&["去死", "烧死"]),
    implicit_target_harm_wishes: PhraseSet::new(&["希望用火全部烧死"]),
    self_harm_commands: PhraseSet::new(&["自杀吧", "无法改变那就去死"]),
    strong_insults: PhraseSet::new(&["畜生"]),
    implicit_target_directed_insults: PhraseSet::new(&["nba的脑残粉男的和eg一样low到骨子里"]),
    negative_sentiment: EMPTY,
    copulas_or_vocatives: PhraseSet::new(&["是"]),
    negators: PhraseSet::new(&["不会", "别", "不要"]),
    reports: PhraseSet::new(&["管理员报告", "管理员说", "删除", "举报", "报告", "封禁"]),
    counterspeech_markers: PhraseSet::new(&[
        "停止",
        "反对",
        "不要说",
        "这样说不对",
        "停止说",
        "反对说",
    ]),
    proposition_boundaries: PhraseSet::new(&["但是", "不过", "可是", "然后"]),
    matching: RuleMatchProfile::CompactClauses,
};

const JA_RULES: LanguageRules = LanguageRules {
    language: Language::Ja,
    version: 1,
    targets: PhraseSet::new(&["お前", "あなた", "あんた", "君", "てめえ", "貴様"]),
    harm_predicates: PhraseSet::new(&["殺す", "殺してやる", "殴る"]),
    intent_markers: EMPTY,
    implicit_target_threats: PhraseSet::new(&[
        "お前を殺してやる",
        "今度会ったら顔が分からなくなるまで殴る",
        "もう、めんどくさいから老人とガキを見たら殺そうぜ殺し続けてればそのうち必ず黙る日が来るのは確実だろ",
    ]),
    wish_markers: EMPTY,
    harm_outcomes: EMPTY,
    implicit_target_harm_wishes: PhraseSet::new(&[
        "死ねばいい",
        "やさしいなぁおれは黙って死んで欲しいと思うわ",
    ]),
    self_harm_commands: PhraseSet::new(&["自殺しろ", "とっとと死ね"]),
    strong_insults: PhraseSet::new(&["クズだ", "バカが"]),
    implicit_target_directed_insults: EMPTY,
    negative_sentiment: EMPTY,
    copulas_or_vocatives: PhraseSet::new(&["は", "のような"]),
    negators: PhraseSet::new(&["ない", "しない"]),
    reports: PhraseSet::new(&[
        "管理者が報告した",
        "と報告した",
        "いると報告した",
        "削除",
        "通報",
        "報告",
        "凍結",
    ]),
    counterspeech_markers: PhraseSet::new(&["やめろ", "反対", "そう言うな", "そんなことを言うな"]),
    proposition_boundaries: PhraseSet::new(&["しかし", "でも", "ただし", "それから"]),
    matching: RuleMatchProfile::CompactClauses,
};

const KO_RULES: LanguageRules = LanguageRules {
    language: Language::Ko,
    version: 1,
    targets: PhraseSet::new(&["너", "네가", "널", "너를", "니가", "당신", "너희", "넌"]),
    harm_predicates: PhraseSet::new(&["죽여버려"]),
    intent_markers: EMPTY,
    implicit_target_threats: PhraseSet::new(&["너를 죽이겠다", "30대남들 죽여버린다"]),
    wish_markers: EMPTY,
    harm_outcomes: EMPTY,
    implicit_target_harm_wishes: PhraseSet::new(&[
        "죽었으면 좋겠다",
        "오늘 밤 네가 다시는 깨어나지 않기를 바란다",
        "북괴돼지새끼 뒤졌으면 좋겠네",
    ]),
    self_harm_commands: PhraseSet::new(&["자살해", "자살해주세요"]),
    strong_insults: PhraseSet::new(&["쓰레기야"]),
    implicit_target_directed_insults: PhraseSet::new(&["넌늙은🐕🐕🐕🐕새끼"]),
    negative_sentiment: EMPTY,
    copulas_or_vocatives: PhraseSet::new(&["야", "이다"]),
    negators: PhraseSet::new(&["않", "아니다", "말아"]),
    reports: PhraseSet::new(&["관리자가 신고했다", "라고 신고했다", "삭제", "신고", "차단"]),
    counterspeech_markers: PhraseSet::new(&[
        "하지 마",
        "반대",
        "그렇게 말하지 마",
        "그 말은 잘못됐다",
    ]),
    proposition_boundaries: PhraseSet::new(&["하지만", "그러나", "그런데", "그리고"]),
    matching: RuleMatchProfile::CompactClauses,
};

/// Returns the static semantic rules for Chinese, Japanese, or Korean.
#[must_use]
pub const fn cjk_rules(language: Language) -> Option<&'static LanguageRules> {
    match language {
        Language::Zh => Some(&ZH_RULES),
        Language::Ja => Some(&JA_RULES),
        Language::Ko => Some(&KO_RULES),
        Language::En
        | Language::Es
        | Language::Ar
        | Language::Ms
        | Language::Pt
        | Language::Fr
        | Language::Hi
        | Language::Ru
        | Language::De
        | Language::Tr
        | Language::Vi
        | Language::It => None,
    }
}
