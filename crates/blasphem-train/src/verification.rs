use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use blasphem::{
    ConfusionMatrix, EvalLabel, Language, NudgeDetector, ReplyTarget, RuntimeInitError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    behavior_panel::{
        BehaviorPanelError, BehaviorRow, ControlKind, EventType, EvidenceKind, load_panel,
        validate_event_distribution,
    },
    calibration::{GateResult, gates},
    datasets::{DatasetSplit, PreparedManifest, PreparedRow},
    evidence::{Sha256Digest, sha256_digest},
    model_manifest::{
        ModelManifest, ModelManifestEntry, ModelSetError, parse_model_manifest, validate_model_set,
    },
    prepared_input::{
        load_prepared_validation, parse_prepared_manifest, validate_manifest_structure,
    },
};

const MINIMUM_CLASS_ROWS: usize = 300;
const EVIDENCE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceInputs {
    pub model_manifest: ModelManifest,
    pub prepared_manifest: PreparedManifest,
    pub model_manifest_sha256: Sha256Digest,
    pub prepared_manifest_sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    CalibrationEvidence,
    BehaviorContractEvidence,
    NativeCliSmokeEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationMetrics {
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub specificity: Option<f64>,
    pub f1: Option<f64>,
    pub false_warning_rate: Option<f64>,
    pub projected_precision_1_percent: Option<f64>,
    pub projected_precision_5_percent: Option<f64>,
}

impl VerificationMetrics {
    #[must_use]
    pub fn from_matrix(matrix: ConfusionMatrix) -> Self {
        let precision = ratio(
            matrix.true_positive,
            matrix.true_positive.saturating_add(matrix.false_positive),
        );
        let recall = ratio(
            matrix.true_positive,
            matrix.true_positive.saturating_add(matrix.false_negative),
        );
        let specificity = ratio(
            matrix.true_negative,
            matrix.true_negative.saturating_add(matrix.false_positive),
        );
        let false_warning_rate = ratio(
            matrix.false_positive,
            matrix.false_positive.saturating_add(matrix.true_negative),
        );
        Self {
            precision,
            recall,
            specificity,
            f1: f1(precision, recall),
            false_warning_rate,
            projected_precision_1_percent: projected_precision(0.01, recall, false_warning_rate),
            projected_precision_5_percent: projected_precision(0.05, recall, false_warning_rate),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageEvaluation {
    pub language: Language,
    pub split: DatasetSplit,
    pub matrix: ConfusionMatrix,
    pub metrics: VerificationMetrics,
    pub gates: Option<GateResult>,
}

impl LanguageEvaluation {
    #[must_use]
    pub fn from_matrix(language: Language, split: DatasetSplit, matrix: ConfusionMatrix) -> Self {
        Self {
            language,
            split,
            matrix,
            metrics: VerificationMetrics::from_matrix(matrix),
            gates: Some(gates(matrix)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationEvidence {
    pub schema_version: u16,
    pub evidence_status: EvidenceStatus,
    pub split: DatasetSplit,
    pub model_manifest_sha256: Sha256Digest,
    pub prepared_manifest_sha256: Sha256Digest,
    pub languages: BTreeMap<String, LanguageEvaluation>,
    pub pooled_matrix: ConfusionMatrix,
}

impl EvaluationEvidence {
    /// Builds one complete 15-language validation evidence record.
    ///
    /// # Errors
    ///
    /// Returns an error when the language set or split is wrong.
    pub fn validation(
        model_manifest_sha256: Sha256Digest,
        prepared_manifest_sha256: Sha256Digest,
        evaluations: Vec<LanguageEvaluation>,
    ) -> Result<Self, VerificationError> {
        let expected = Language::ALL
            .into_iter()
            .map(|language| language.code().to_owned())
            .collect::<BTreeSet<_>>();
        let mut languages = BTreeMap::new();
        let mut pooled_matrix = ConfusionMatrix::default();
        for evaluation in evaluations {
            if evaluation.split != DatasetSplit::Validation
                || languages
                    .insert(evaluation.language.code().to_owned(), evaluation.clone())
                    .is_some()
            {
                return Err(VerificationError::EvaluationLanguageSet);
            }
            add_matrix(&mut pooled_matrix, evaluation.matrix);
        }
        if languages.keys().cloned().collect::<BTreeSet<_>>() != expected {
            return Err(VerificationError::EvaluationLanguageSet);
        }
        for evaluation in languages.values() {
            let expected_gates = gates(evaluation.matrix);
            if evaluation.metrics != VerificationMetrics::from_matrix(evaluation.matrix)
                || evaluation.gates != Some(expected_gates)
            {
                return Err(VerificationError::EvaluationDerivedFields(
                    evaluation.language,
                ));
            }
            if !expected_gates.passed() {
                return Err(VerificationError::EvaluationGateFailure(
                    evaluation.language,
                ));
            }
        }
        Ok(Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            evidence_status: EvidenceStatus::CalibrationEvidence,
            split: DatasetSplit::Validation,
            model_manifest_sha256,
            prepared_manifest_sha256,
            languages,
            pooled_matrix,
        })
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.languages
            .values()
            .all(|evaluation| evaluation.gates.is_some_and(GateResult::passed))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorCaseResult {
    pub case_id: String,
    pub text: String,
    pub event_type: EventType,
    pub pair_id: String,
    pub control_kind: ControlKind,
    pub evidence_kind: EvidenceKind,
    pub evidence_ref: String,
    pub expected_nudge: bool,
    pub actual_nudge: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageBehaviorResult {
    pub language: Language,
    pub passed: bool,
    pub cases: Vec<BehaviorCaseResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorEvidence {
    pub schema_version: u16,
    pub evidence_status: EvidenceStatus,
    pub model_manifest_sha256: Sha256Digest,
    pub prepared_manifest_sha256: Sha256Digest,
    pub languages: BTreeMap<String, LanguageBehaviorResult>,
}

impl BehaviorEvidence {
    /// Builds a typed 15-language behavior contract record.
    ///
    /// # Errors
    ///
    /// Returns an error for a missing language, duplicate case, or invalid summary.
    pub fn new(
        model_manifest_sha256: Sha256Digest,
        prepared_manifest_sha256: Sha256Digest,
        results: Vec<LanguageBehaviorResult>,
    ) -> Result<Self, VerificationError> {
        let expected_languages = all_language_codes();
        let mut languages = BTreeMap::new();
        let mut case_ids = BTreeSet::new();
        for result in results {
            if result.cases.len() != 24 {
                return Err(VerificationError::BehaviorEvidenceShape);
            }
            for case in &result.cases {
                if !case_ids.insert(case.case_id.clone())
                    || case.passed != (case.actual_nudge == case.expected_nudge)
                {
                    return Err(VerificationError::BehaviorEvidenceShape);
                }
            }
            if result.passed != result.cases.iter().all(|case| case.passed) {
                return Err(VerificationError::EvidenceSummaryMismatch);
            }
            if languages
                .insert(result.language.code().to_owned(), result)
                .is_some()
            {
                return Err(VerificationError::BehaviorEvidenceShape);
            }
        }
        if languages.keys().cloned().collect::<BTreeSet<_>>() != expected_languages
            || case_ids.len() != 360
        {
            return Err(VerificationError::BehaviorEvidenceShape);
        }
        Ok(Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            evidence_status: EvidenceStatus::BehaviorContractEvidence,
            model_manifest_sha256,
            prepared_manifest_sha256,
            languages,
        })
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.languages.values().all(|language| language.passed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliSmokeSuite {
    Supplied,
    Context,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliSmokeCase {
    pub case_id: &'static str,
    pub suite: CliSmokeSuite,
    pub language: Language,
    pub text: &'static str,
    pub expected_nudge: bool,
}

#[must_use]
pub fn cli_smoke_cases() -> &'static [CliSmokeCase] {
    &CLI_SMOKE_CASES
}

const fn smoke_case(
    case_id: &'static str,
    suite: CliSmokeSuite,
    language: Language,
    text: &'static str,
    expected_nudge: bool,
) -> CliSmokeCase {
    CliSmokeCase {
        case_id,
        suite,
        language,
        text,
        expected_nudge,
    }
}

static CLI_SMOKE_CASES: [CliSmokeCase; 60] = [
    smoke_case(
        "supplied-en-toxic",
        CliSmokeSuite::Supplied,
        Language::En,
        "When I find you, I will break every bone in your hand",
        true,
    ),
    smoke_case(
        "supplied-en-clean",
        CliSmokeSuite::Supplied,
        Language::En,
        "The teacher explained why the threatening message should be reported",
        false,
    ),
    smoke_case(
        "supplied-zh-toxic",
        CliSmokeSuite::Supplied,
        Language::Zh,
        "等我找到你，我会打断你的双腿",
        true,
    ),
    smoke_case(
        "supplied-zh-clean",
        CliSmokeSuite::Supplied,
        Language::Zh,
        "管理员删除了那条威胁别人的消息",
        false,
    ),
    smoke_case(
        "supplied-es-toxic",
        CliSmokeSuite::Supplied,
        Language::Es,
        "Cuando salgas del edificio, voy a romperte todos los dientes",
        true,
    ),
    smoke_case(
        "supplied-es-clean",
        CliSmokeSuite::Supplied,
        Language::Es,
        "No quiero que nadie vuelva a hacerte daño",
        false,
    ),
    smoke_case(
        "supplied-ar-toxic",
        CliSmokeSuite::Supplied,
        Language::Ar,
        "أتمنى أن تموت وحيدًا هذه الليلة",
        true,
    ),
    smoke_case(
        "supplied-ar-clean",
        CliSmokeSuite::Supplied,
        Language::Ar,
        "أبلغت المشرف عن رسالة تهدد أحد الأعضاء",
        false,
    ),
    smoke_case(
        "supplied-id-toxic",
        CliSmokeSuite::Supplied,
        Language::Ms,
        "Kalau bertemu lagi, akan kupatahkan rahangmu",
        true,
    ),
    smoke_case(
        "supplied-id-clean",
        CliSmokeSuite::Supplied,
        Language::Ms,
        "Saya melaporkan pesan yang mengancam pemain lain",
        false,
    ),
    smoke_case(
        "supplied-pt-toxic",
        CliSmokeSuite::Supplied,
        Language::Pt,
        "Espero que você nunca mais acorde",
        true,
    ),
    smoke_case(
        "supplied-pt-clean",
        CliSmokeSuite::Supplied,
        Language::Pt,
        "Denunciei a mensagem que ameaçava outro jogador",
        false,
    ),
    smoke_case(
        "supplied-fr-toxic",
        CliSmokeSuite::Supplied,
        Language::Fr,
        "Quand je te trouverai, je te briserai les deux jambes",
        true,
    ),
    smoke_case(
        "supplied-fr-clean",
        CliSmokeSuite::Supplied,
        Language::Fr,
        "J'ai signalé le message qui menaçait un autre joueur",
        false,
    ),
    smoke_case(
        "supplied-hi-toxic",
        CliSmokeSuite::Supplied,
        Language::Hi,
        "मैं तुम्हें ढूँढकर तुम्हारी हड्डियाँ तोड़ दूँगा",
        true,
    ),
    smoke_case(
        "supplied-hi-clean",
        CliSmokeSuite::Supplied,
        Language::Hi,
        "मैंने दूसरे खिलाड़ी को धमकाने वाला संदेश रिपोर्ट किया",
        false,
    ),
    smoke_case(
        "supplied-ru-toxic",
        CliSmokeSuite::Supplied,
        Language::Ru,
        "Надеюсь, этой ночью ты больше не проснёшься",
        true,
    ),
    smoke_case(
        "supplied-ru-clean",
        CliSmokeSuite::Supplied,
        Language::Ru,
        "Модератор удалил сообщение с угрозой другому игроку",
        false,
    ),
    smoke_case(
        "supplied-ja-toxic",
        CliSmokeSuite::Supplied,
        Language::Ja,
        "今度会ったら顔が分からなくなるまで殴る",
        true,
    ),
    smoke_case(
        "supplied-ja-clean",
        CliSmokeSuite::Supplied,
        Language::Ja,
        "管理者は他の利用者を脅す投稿を削除した",
        false,
    ),
    smoke_case(
        "supplied-de-toxic",
        CliSmokeSuite::Supplied,
        Language::De,
        "Ich hoffe, du wachst morgen nicht mehr auf",
        true,
    ),
    smoke_case(
        "supplied-de-clean",
        CliSmokeSuite::Supplied,
        Language::De,
        "Die Moderatorin löschte die Nachricht mit der Drohung",
        false,
    ),
    smoke_case(
        "supplied-tr-toxic",
        CliSmokeSuite::Supplied,
        Language::Tr,
        "Seni bulunca bütün dişlerini kıracağım",
        true,
    ),
    smoke_case(
        "supplied-tr-clean",
        CliSmokeSuite::Supplied,
        Language::Tr,
        "Moderatör başka bir oyuncuyu tehdit eden mesajı sildi",
        false,
    ),
    smoke_case(
        "supplied-vi-toxic",
        CliSmokeSuite::Supplied,
        Language::Vi,
        "Tao sẽ tìm mày và đập gãy hết răng",
        true,
    ),
    smoke_case(
        "supplied-vi-clean",
        CliSmokeSuite::Supplied,
        Language::Vi,
        "Quản trị viên đã xóa tin nhắn đe dọa người chơi khác",
        false,
    ),
    smoke_case(
        "supplied-ko-toxic",
        CliSmokeSuite::Supplied,
        Language::Ko,
        "오늘 밤 네가 다시는 깨어나지 않기를 바란다",
        true,
    ),
    smoke_case(
        "supplied-ko-clean",
        CliSmokeSuite::Supplied,
        Language::Ko,
        "관리자가 다른 사용자를 협박한 메시지를 삭제했다",
        false,
    ),
    smoke_case(
        "supplied-it-toxic",
        CliSmokeSuite::Supplied,
        Language::It,
        "Quando ti vedo, ti rompo tutti i denti",
        true,
    ),
    smoke_case(
        "supplied-it-clean",
        CliSmokeSuite::Supplied,
        Language::It,
        "La moderatrice ha cancellato il messaggio che minacciava un altro giocatore",
        false,
    ),
    smoke_case(
        "context-en-toxic",
        CliSmokeSuite::Context,
        Language::En,
        "I will kill you",
        true,
    ),
    smoke_case(
        "context-en-clean",
        CliSmokeSuite::Context,
        Language::En,
        "The moderator reported a message saying \"I will kill you\"",
        false,
    ),
    smoke_case(
        "context-zh-toxic",
        CliSmokeSuite::Context,
        Language::Zh,
        "我要杀了你",
        true,
    ),
    smoke_case(
        "context-zh-clean",
        CliSmokeSuite::Context,
        Language::Zh,
        "管理员报告了“我要杀了你”这条消息",
        false,
    ),
    smoke_case(
        "context-es-toxic",
        CliSmokeSuite::Context,
        Language::Es,
        "Te voy a matar",
        true,
    ),
    smoke_case(
        "context-es-clean",
        CliSmokeSuite::Context,
        Language::Es,
        "La moderadora reportó el mensaje \"Te voy a matar\"",
        false,
    ),
    smoke_case(
        "context-ar-toxic",
        CliSmokeSuite::Context,
        Language::Ar,
        "سأقتلك",
        true,
    ),
    smoke_case(
        "context-ar-clean",
        CliSmokeSuite::Context,
        Language::Ar,
        "أبلغ المشرف عن رسالة تقول \"سأقتلك\"",
        false,
    ),
    smoke_case(
        "context-id-toxic",
        CliSmokeSuite::Context,
        Language::Ms,
        "aku akan membunuhmu",
        true,
    ),
    smoke_case(
        "context-id-clean",
        CliSmokeSuite::Context,
        Language::Ms,
        "Moderator melaporkan pesan \"aku akan membunuhmu\"",
        false,
    ),
    smoke_case(
        "context-pt-toxic",
        CliSmokeSuite::Context,
        Language::Pt,
        "vou te matar",
        true,
    ),
    smoke_case(
        "context-pt-clean",
        CliSmokeSuite::Context,
        Language::Pt,
        "A moderadora denunciou a mensagem \"vou te matar\"",
        false,
    ),
    smoke_case(
        "context-fr-toxic",
        CliSmokeSuite::Context,
        Language::Fr,
        "je vais te tuer",
        true,
    ),
    smoke_case(
        "context-fr-clean",
        CliSmokeSuite::Context,
        Language::Fr,
        "La modératrice a signalé le message « je vais te tuer »",
        false,
    ),
    smoke_case(
        "context-hi-toxic",
        CliSmokeSuite::Context,
        Language::Hi,
        "मैं तुम्हें मार दूंगा",
        true,
    ),
    smoke_case(
        "context-hi-clean",
        CliSmokeSuite::Context,
        Language::Hi,
        "मॉडरेटर ने \"मैं तुम्हें मार दूंगा\" संदेश रिपोर्ट किया",
        false,
    ),
    smoke_case(
        "context-ru-toxic",
        CliSmokeSuite::Context,
        Language::Ru,
        "я тебя убью",
        true,
    ),
    smoke_case(
        "context-ru-clean",
        CliSmokeSuite::Context,
        Language::Ru,
        "Модератор сообщил о сообщении «я тебя убью»",
        false,
    ),
    smoke_case(
        "context-ja-toxic",
        CliSmokeSuite::Context,
        Language::Ja,
        "お前を殺してやる",
        true,
    ),
    smoke_case(
        "context-ja-clean",
        CliSmokeSuite::Context,
        Language::Ja,
        "管理者は「お前を殺してやる」という投稿を報告した",
        false,
    ),
    smoke_case(
        "context-de-toxic",
        CliSmokeSuite::Context,
        Language::De,
        "ich werde dich töten",
        true,
    ),
    smoke_case(
        "context-de-clean",
        CliSmokeSuite::Context,
        Language::De,
        "Die Moderatorin meldete die Nachricht „ich werde dich töten“",
        false,
    ),
    smoke_case(
        "context-tr-toxic",
        CliSmokeSuite::Context,
        Language::Tr,
        "seni öldüreceğim",
        true,
    ),
    smoke_case(
        "context-tr-clean",
        CliSmokeSuite::Context,
        Language::Tr,
        "Moderatör \"seni öldüreceğim\" mesajını bildirdi",
        false,
    ),
    smoke_case(
        "context-vi-toxic",
        CliSmokeSuite::Context,
        Language::Vi,
        "tao sẽ giết mày",
        true,
    ),
    smoke_case(
        "context-vi-clean",
        CliSmokeSuite::Context,
        Language::Vi,
        "Quản trị viên báo cáo tin nhắn \"tao sẽ giết mày\"",
        false,
    ),
    smoke_case(
        "context-ko-toxic",
        CliSmokeSuite::Context,
        Language::Ko,
        "너를 죽이겠다",
        true,
    ),
    smoke_case(
        "context-ko-clean",
        CliSmokeSuite::Context,
        Language::Ko,
        "관리자가 \"너를 죽이겠다\"라는 메시지를 신고했다",
        false,
    ),
    smoke_case(
        "context-it-toxic",
        CliSmokeSuite::Context,
        Language::It,
        "ti ucciderò",
        true,
    ),
    smoke_case(
        "context-it-clean",
        CliSmokeSuite::Context,
        Language::It,
        "La moderatrice ha segnalato il messaggio \"ti ucciderò\"",
        false,
    ),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliSmokeCaseResult {
    pub case_id: String,
    pub suite: CliSmokeSuite,
    pub language: Language,
    pub text: String,
    pub expected_nudge: bool,
    pub ok: bool,
    pub score: u8,
    pub threshold: u8,
    pub should_nudge: bool,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageCliSmokeResult {
    pub language: Language,
    pub passed: bool,
    pub cases: Vec<CliSmokeCaseResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliSmokeEvidence {
    pub schema_version: u16,
    pub evidence_status: EvidenceStatus,
    pub model_manifest_sha256: Sha256Digest,
    pub languages: BTreeMap<String, LanguageCliSmokeResult>,
}

impl CliSmokeEvidence {
    /// Builds one typed native smoke record for all 60 fixed cases.
    ///
    /// # Errors
    ///
    /// Returns an error for a changed case, invalid result, or incomplete language set.
    pub fn new(
        model_manifest_sha256: Sha256Digest,
        results: Vec<LanguageCliSmokeResult>,
    ) -> Result<Self, VerificationError> {
        let expected_cases = cli_smoke_cases()
            .iter()
            .map(|case| (case.case_id, case))
            .collect::<BTreeMap<_, _>>();
        let mut case_ids = BTreeSet::new();
        let mut languages = BTreeMap::new();
        for result in results {
            if result.passed != result.cases.iter().all(|case| case.passed) {
                return Err(VerificationError::EvidenceSummaryMismatch);
            }
            if result.cases.len() != 4 {
                return Err(VerificationError::CliSmokeEvidenceShape);
            }
            for case in &result.cases {
                let Some(expected) = expected_cases.get(case.case_id.as_str()) else {
                    return Err(VerificationError::CliSmokeEvidenceShape);
                };
                if !case_ids.insert(case.case_id.clone())
                    || case.language != result.language
                    || case.language != expected.language
                    || case.suite != expected.suite
                    || case.text != expected.text
                    || case.expected_nudge != expected.expected_nudge
                {
                    return Err(VerificationError::CliSmokeEvidenceShape);
                }
                if case.threshold != 50
                    || case.ok == case.should_nudge
                    || case.should_nudge != (case.score >= case.threshold)
                {
                    return Err(VerificationError::PublicNudgeInvariant(
                        case.case_id.clone(),
                    ));
                }
                if case.passed != (case.should_nudge == case.expected_nudge) {
                    return Err(VerificationError::CliSmokeEvidenceShape);
                }
            }
            if languages
                .insert(result.language.code().to_owned(), result)
                .is_some()
            {
                return Err(VerificationError::CliSmokeEvidenceShape);
            }
        }
        let expected_languages = Language::ALL
            .into_iter()
            .map(|language| language.code().to_owned())
            .collect::<BTreeSet<_>>();
        if languages.keys().cloned().collect::<BTreeSet<_>>() != expected_languages
            || case_ids.len() != expected_cases.len()
        {
            return Err(VerificationError::CliSmokeEvidenceShape);
        }
        Ok(Self {
            schema_version: EVIDENCE_SCHEMA_VERSION,
            evidence_status: EvidenceStatus::NativeCliSmokeEvidence,
            model_manifest_sha256,
            languages,
        })
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.languages.values().all(|language| language.passed)
    }
}

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error(
        "{language} {split:?} has clean={clean_rows} and toxic={toxic_rows}; each class requires 300 rows"
    )]
    InsufficientClassRows {
        language: Language,
        split: DatasetSplit,
        clean_rows: usize,
        toxic_rows: usize,
    },
    #[error("validation evidence must contain each new language exactly once")]
    EvaluationLanguageSet,
    #[error("validation evidence has changed derived fields for {0}")]
    EvaluationDerivedFields(Language),
    #[error("validation evidence fails a quality gate for {0}")]
    EvaluationGateFailure(Language),
    #[error("prepared row {source_id} has language {actual}; expected {expected}")]
    RowLanguageMismatch {
        source_id: String,
        expected: Language,
        actual: Language,
    },
    #[error("cannot read behavior provenance: {0}")]
    ProvenanceIo(#[from] std::io::Error),
    #[error("cannot parse behavior provenance: {0}")]
    ProvenanceCsv(#[from] csv::Error),
    #[error("behavior provenance misses required column: {0}")]
    ProvenanceHeader(&'static str),
    #[error("behavior evidence reference is used for multiple languages: {0}")]
    BehaviorEvidenceLanguageConflict(String),
    #[error("behavior evidence reference does not map to one provenance row: {0}")]
    BehaviorEvidenceCount(String),
    #[error("behavior evidence reference is not a final audit-only exclusion: {0}")]
    BehaviorEvidenceNotAuditOnly(String),
    #[error("behavior evidence reference has the wrong language: {0}")]
    BehaviorEvidenceWrongLanguage(String),
    #[error("behavior evidence must contain 15 languages and 360 valid cases")]
    BehaviorEvidenceShape,
    #[error("native smoke evidence must contain 15 languages and the 60 fixed cases")]
    CliSmokeEvidenceShape,
    #[error("an evidence summary does not match its case results")]
    EvidenceSummaryMismatch,
    #[error("native smoke case violates the public nudge invariant: {0}")]
    PublicNudgeInvariant(String),
    #[error("cannot read evidence input {}: {source}", path.display())]
    EvidenceInputIo {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("the model manifest has no parent directory")]
    ModelManifestRoot,
    #[error("invalid model or prepared input: {0}")]
    ModelSet(#[from] ModelSetError),
    #[error("model manifest misses language {0}")]
    MissingModelLanguage(Language),
    #[error("model manifest misses a HurtLex digest for {0}")]
    MissingHurtlexDigest(Language),
    #[error("HurtLex digest mismatch for {0}")]
    HurtlexDigestMismatch(Language),
    #[error("cannot initialize the detector for {language}: {source}")]
    RuntimeInit {
        language: Language,
        #[source]
        source: RuntimeInitError,
    },
    #[error("final-path validation differs from the model manifest for {0}")]
    ValidationManifestMismatch(Language),
    #[error("invalid behavior panel: {0}")]
    BehaviorPanel(#[from] BehaviorPanelError),
}

/// Validates the per-class evidence floor for a prepared split.
///
/// # Errors
///
/// Returns an error when a new language has fewer than 300 rows in either class.
pub fn validate_class_counts(
    language: Language,
    split: DatasetSplit,
    rows: &[PreparedRow],
) -> Result<(), VerificationError> {
    let clean_rows = rows
        .iter()
        .filter(|row| row.label == EvalLabel::Clean)
        .count();
    let toxic_rows = rows.len().saturating_sub(clean_rows);
    if clean_rows < MINIMUM_CLASS_ROWS || toxic_rows < MINIMUM_CLASS_ROWS {
        return Err(VerificationError::InsufficientClassRows {
            language,
            split,
            clean_rows,
            toxic_rows,
        });
    }
    Ok(())
}

/// Evaluates one validation split through the product Boolean path.
///
/// # Errors
///
/// Returns an error for an invalid class count or a row from another language.
pub fn evaluate_language_validation(
    detector: &NudgeDetector,
    rows: &[PreparedRow],
) -> Result<LanguageEvaluation, VerificationError> {
    let language = detector.language();
    for row in rows {
        if row.detector_language != language {
            return Err(VerificationError::RowLanguageMismatch {
                source_id: row.source_id.clone(),
                expected: language,
                actual: row.detector_language,
            });
        }
    }
    validate_class_counts(language, DatasetSplit::Validation, rows)?;
    let mut matrix = ConfusionMatrix::default();
    for row in rows {
        let predicted = detector.check(&row.text, ReplyTarget::Unknown).should_nudge;
        match (row.label, predicted) {
            (EvalLabel::Toxic, true) => matrix.true_positive += 1,
            (EvalLabel::Clean, false) => matrix.true_negative += 1,
            (EvalLabel::Clean, true) => matrix.false_positive += 1,
            (EvalLabel::Toxic, false) => matrix.false_negative += 1,
        }
    }
    Ok(LanguageEvaluation::from_matrix(
        language,
        DatasetSplit::Validation,
        matrix,
    ))
}

/// Evaluates every new-language validation split through the product path.
///
/// # Errors
///
/// Returns an error for changed inputs, invalid splits, runtime failures, or parity drift.
pub fn evaluate_validation(
    prepared_root: &Path,
    model_manifest_path: &Path,
    hurtlex_root: &Path,
) -> Result<EvaluationEvidence, VerificationError> {
    let inputs = load_evidence_inputs(prepared_root, model_manifest_path)?;
    let mut evaluations = Vec::with_capacity(Language::ALL.len());
    for language in Language::ALL {
        let entry = manifest_entry(&inputs.model_manifest, language)?;
        let detector = load_detector(language, entry, hurtlex_root)?;
        let prepared = load_prepared_validation(prepared_root, language)?;
        let evaluation = evaluate_language_validation(&detector, &prepared.validation)?;
        if evaluation.matrix != entry.validation {
            return Err(VerificationError::ValidationManifestMismatch(language));
        }
        evaluations.push(evaluation);
    }
    EvaluationEvidence::validation(
        inputs.model_manifest_sha256,
        inputs.prepared_manifest_sha256,
        evaluations,
    )
}

/// Evaluates all 15 behavior panels through the product path.
///
/// # Errors
///
/// Returns an error for changed inputs, invalid panels, or runtime initialization failures.
pub fn evaluate_behavior(
    fixture_root: &Path,
    prepared_root: &Path,
    model_manifest_path: &Path,
    hurtlex_root: &Path,
) -> Result<BehaviorEvidence, VerificationError> {
    let inputs = load_evidence_inputs(prepared_root, model_manifest_path)?;
    let mut panels = BTreeMap::new();
    for language in Language::ALL {
        let rows = load_panel(fixture_root, language)?;
        validate_event_distribution(&rows)?;
        panels.insert(language, rows);
    }
    validate_behavior_provenance(&prepared_root.join("provenance.tsv"), &panels)?;

    let mut results = Vec::with_capacity(panels.len());
    for (language, rows) in panels {
        let entry = manifest_entry(&inputs.model_manifest, language)?;
        let detector = load_detector(language, entry, hurtlex_root)?;
        let cases = rows
            .into_iter()
            .map(|row| {
                let actual_nudge = detector.check(&row.text, ReplyTarget::Unknown).should_nudge;
                BehaviorCaseResult {
                    case_id: row.case_id,
                    text: row.text,
                    event_type: row.event_type,
                    pair_id: row.pair_id,
                    control_kind: row.control_kind,
                    evidence_kind: row.evidence_kind,
                    evidence_ref: row.evidence_ref,
                    expected_nudge: row.expected_nudge,
                    actual_nudge,
                    passed: actual_nudge == row.expected_nudge,
                }
            })
            .collect::<Vec<_>>();
        results.push(LanguageBehaviorResult {
            language,
            passed: cases.iter().all(|case| case.passed),
            cases,
        });
    }
    BehaviorEvidence::new(
        inputs.model_manifest_sha256,
        inputs.prepared_manifest_sha256,
        results,
    )
}

/// Evaluates all 60 fixed native CLI smoke cases through the product path.
///
/// # Errors
///
/// Returns an error for changed model inputs or runtime initialization failures.
pub fn evaluate_cli_smoke(
    model_manifest_path: &Path,
    hurtlex_root: &Path,
) -> Result<CliSmokeEvidence, VerificationError> {
    let (manifest, model_manifest_sha256) = load_model_evidence_input(model_manifest_path)?;
    let mut results = Vec::with_capacity(Language::ALL.len());

    for language in Language::ALL {
        let entry = manifest_entry(&manifest, language)?;
        let detector = load_detector(language, entry, hurtlex_root)?;
        let cases = cli_smoke_cases()
            .iter()
            .filter(|case| case.language == language)
            .map(|case| {
                let nudge = detector.check(case.text, ReplyTarget::Unknown);
                CliSmokeCaseResult {
                    case_id: case.case_id.to_owned(),
                    suite: case.suite,
                    language,
                    text: case.text.to_owned(),
                    expected_nudge: case.expected_nudge,
                    ok: !nudge.should_nudge,
                    score: nudge.score,
                    threshold: nudge.threshold,
                    should_nudge: nudge.should_nudge,
                    passed: nudge.should_nudge == case.expected_nudge,
                }
            })
            .collect::<Vec<_>>();
        results.push(LanguageCliSmokeResult {
            language,
            passed: cases.iter().all(|case| case.passed),
            cases,
        });
    }

    CliSmokeEvidence::new(model_manifest_sha256, results)
}

/// Loads typed model and prepared manifests and hashes their exact bytes.
///
/// # Errors
///
/// Returns an error for unreadable, malformed, incomplete, or changed inputs.
pub fn load_evidence_inputs(
    prepared_root: &Path,
    model_manifest_path: &Path,
) -> Result<EvidenceInputs, VerificationError> {
    let (model_manifest, model_manifest_sha256) = load_model_evidence_input(model_manifest_path)?;

    let prepared_path = prepared_root.join("manifest.json");
    let prepared_bytes = read_evidence_input(&prepared_path)?;
    let prepared_manifest = parse_prepared_manifest(prepared_bytes.as_slice())?;
    validate_manifest_structure(&prepared_manifest)?;

    Ok(EvidenceInputs {
        model_manifest,
        prepared_manifest,
        model_manifest_sha256,
        prepared_manifest_sha256: sha256_digest(&prepared_bytes),
    })
}

/// Verifies dataset-backed behavior references against the final provenance file.
///
/// # Errors
///
/// Returns an error when a reference lacks one matching audit-only exclusion.
pub fn validate_behavior_provenance(
    path: &Path,
    panels: &BTreeMap<Language, Vec<BehaviorRow>>,
) -> Result<(), VerificationError> {
    let mut required = BTreeMap::<String, (Language, usize)>::new();
    for (language, rows) in panels {
        for row in rows
            .iter()
            .filter(|row| row.evidence_kind == EvidenceKind::Dataset)
        {
            match required.get(&row.evidence_ref) {
                Some((actual, _)) if actual != language => {
                    return Err(VerificationError::BehaviorEvidenceLanguageConflict(
                        row.evidence_ref.clone(),
                    ));
                }
                Some(_) => {}
                None => {
                    required.insert(row.evidence_ref.clone(), (*language, 0));
                }
            }
        }
    }

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .flexible(false)
        .from_path(path)?;
    let headers = reader.headers()?.clone();
    let source_id_index = header_index(&headers, "source_id")?;
    let language_index = header_index(&headers, "detector_language_code")?;
    let inclusion_index = header_index(&headers, "inclusion_status")?;
    let reason_index = header_index(&headers, "exclusion_reason")?;

    for record in reader.records() {
        let record = record?;
        let source_id = record.get(source_id_index).unwrap_or_default();
        let Some((expected_language, count)) = required.get_mut(source_id) else {
            continue;
        };
        *count = count.saturating_add(1);
        if *count != 1 {
            return Err(VerificationError::BehaviorEvidenceCount(
                source_id.to_owned(),
            ));
        }
        if record.get(inclusion_index) != Some("excluded")
            || record.get(reason_index) != Some("audit_only")
        {
            return Err(VerificationError::BehaviorEvidenceNotAuditOnly(
                source_id.to_owned(),
            ));
        }
        if record.get(language_index) != Some(expected_language.storage_code()) {
            return Err(VerificationError::BehaviorEvidenceWrongLanguage(
                source_id.to_owned(),
            ));
        }
    }

    if let Some((source_id, _)) = required.into_iter().find(|(_, (_, count))| *count != 1) {
        return Err(VerificationError::BehaviorEvidenceCount(source_id));
    }
    Ok(())
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    (denominator != 0).then(|| numerator as f64 / denominator as f64)
}

fn f1(precision: Option<f64>, recall: Option<f64>) -> Option<f64> {
    let (precision, recall) = (precision?, recall?);
    let denominator = precision + recall;
    (denominator != 0.0).then(|| 2.0 * precision * recall / denominator)
}

fn projected_precision(
    prevalence: f64,
    recall: Option<f64>,
    false_warning_rate: Option<f64>,
) -> Option<f64> {
    let numerator = prevalence * recall?;
    let denominator = numerator + (1.0 - prevalence) * false_warning_rate?;
    (denominator != 0.0).then(|| numerator / denominator)
}

fn add_matrix(total: &mut ConfusionMatrix, matrix: ConfusionMatrix) {
    total.true_positive = total.true_positive.saturating_add(matrix.true_positive);
    total.true_negative = total.true_negative.saturating_add(matrix.true_negative);
    total.false_positive = total.false_positive.saturating_add(matrix.false_positive);
    total.false_negative = total.false_negative.saturating_add(matrix.false_negative);
}

fn header_index(
    headers: &csv::StringRecord,
    name: &'static str,
) -> Result<usize, VerificationError> {
    headers
        .iter()
        .position(|header| header == name)
        .ok_or(VerificationError::ProvenanceHeader(name))
}

fn all_language_codes() -> BTreeSet<String> {
    Language::ALL
        .into_iter()
        .map(|language| language.code().to_owned())
        .collect()
}

fn read_evidence_input(path: &Path) -> Result<Vec<u8>, VerificationError> {
    fs::read(path).map_err(|source| VerificationError::EvidenceInputIo {
        path: path.to_owned(),
        source,
    })
}

fn load_model_evidence_input(
    model_manifest_path: &Path,
) -> Result<(ModelManifest, Sha256Digest), VerificationError> {
    let bytes = read_evidence_input(model_manifest_path)?;
    let manifest = parse_model_manifest(bytes.as_slice())?;
    let model_root = model_manifest_path
        .parent()
        .ok_or(VerificationError::ModelManifestRoot)?;
    validate_model_set(model_root, &manifest)?;
    Ok((manifest, sha256_digest(&bytes)))
}

fn manifest_entry(
    manifest: &ModelManifest,
    language: Language,
) -> Result<&ModelManifestEntry, VerificationError> {
    manifest
        .entries
        .iter()
        .find(|entry| entry.language == language)
        .ok_or(VerificationError::MissingModelLanguage(language))
}

fn load_detector(
    language: Language,
    entry: &ModelManifestEntry,
    hurtlex_root: &Path,
) -> Result<NudgeDetector, VerificationError> {
    let path = hurtlex_root
        .join(language.storage_code())
        .join("1.2")
        .join(format!("hurtlex_{}.tsv", language.storage_code()));
    let bytes = read_evidence_input(&path)?;
    let expected = entry
        .hurtlex_sha256
        .as_ref()
        .ok_or(VerificationError::MissingHurtlexDigest(language))?;
    if &sha256_digest(&bytes) != expected {
        return Err(VerificationError::HurtlexDigestMismatch(language));
    }
    NudgeDetector::from_hurtlex_bytes(language, Some(&bytes))
        .map_err(|source| VerificationError::RuntimeInit { language, source })
}
