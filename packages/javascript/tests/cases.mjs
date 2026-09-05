// Environment neutral: no DOM, no Node APIs. `createJudge(options)` resolves to a Judge.
// Rows are [case id, language, text, expected nudge or score out of 100].

export const SUPPLIED_CASES = [
  ["supplied-en-toxic", "EN", "When I find you, I will break every bone in your hand", true],
  ["supplied-en-clean", "EN", "The teacher explained why the threatening message should be reported", false],
  ["supplied-zh-toxic", "ZH", "等我找到你，我会打断你的双腿", true],
  ["supplied-zh-clean", "ZH", "管理员删除了那条威胁别人的消息", false],
  ["supplied-es-toxic", "ES", "Cuando salgas del edificio, voy a romperte todos los dientes", true],
  ["supplied-es-clean", "ES", "No quiero que nadie vuelva a hacerte daño", false],
  ["supplied-ar-toxic", "AR", "أتمنى أن تموت وحيدًا هذه الليلة", true],
  ["supplied-ar-clean", "AR", "أبلغت المشرف عن رسالة تهدد أحد الأعضاء", false],
  ["supplied-id-toxic", "ID", "Kalau bertemu lagi, akan kupatahkan rahangmu", true],
  ["supplied-id-clean", "ID", "Saya melaporkan pesan yang mengancam pemain lain", false],
  ["supplied-pt-toxic", "PT", "Espero que você nunca mais acorde", true],
  ["supplied-pt-clean", "PT", "Denunciei a mensagem que ameaçava outro jogador", false],
  ["supplied-fr-toxic", "FR", "Quand je te trouverai, je te briserai les deux jambes", true],
  ["supplied-fr-clean", "FR", "J'ai signalé le message qui menaçait un autre joueur", false],
  ["supplied-hi-toxic", "HI", "मैं तुम्हें ढूँढकर तुम्हारी हड्डियाँ तोड़ दूँगा", true],
  ["supplied-hi-clean", "HI", "मैंने दूसरे खिलाड़ी को धमकाने वाला संदेश रिपोर्ट किया", false],
  ["supplied-ru-toxic", "RU", "Надеюсь, этой ночью ты больше не проснёшься", true],
  ["supplied-ru-clean", "RU", "Модератор удалил сообщение с угрозой другому игроку", false],
  ["supplied-ja-toxic", "JA", "今度会ったら顔が分からなくなるまで殴る", true],
  ["supplied-ja-clean", "JA", "管理者は他の利用者を脅す投稿を削除した", false],
  ["supplied-de-toxic", "DE", "Ich hoffe, du wachst morgen nicht mehr auf", true],
  ["supplied-de-clean", "DE", "Die Moderatorin löschte die Nachricht mit der Drohung", false],
  ["supplied-tr-toxic", "TR", "Seni bulunca bütün dişlerini kıracağım", true],
  ["supplied-tr-clean", "TR", "Moderatör başka bir oyuncuyu tehdit eden mesajı sildi", false],
  ["supplied-vi-toxic", "VI", "Tao sẽ tìm mày và đập gãy hết răng", true],
  ["supplied-vi-clean", "VI", "Quản trị viên đã xóa tin nhắn đe dọa người chơi khác", false],
  ["supplied-ko-toxic", "KO", "오늘 밤 네가 다시는 깨어나지 않기를 바란다", true],
  ["supplied-ko-clean", "KO", "관리자가 다른 사용자를 협박한 메시지를 삭제했다", false],
  ["supplied-it-toxic", "IT", "Quando ti vedo, ti rompo tutti i denti", true],
  ["supplied-it-clean", "IT", "La moderatrice ha cancellato il messaggio che minacciava un altro giocatore", false],
];

export const AUTO_CASES = [
  ["auto-en", "EN", "I never should've bought that.", 48],
  ["auto-zh", "ZH", "我想要确定什么都没有发生在汤姆身上。", 0],
  ["auto-es", "ES", "Was ist das? A ship. Todo bien en la costa.", 17],
  ["auto-ar", "AR", "هل تحب الكتب؟", 5],
  ["auto-ms", "MS", "Dia memberitahu saya yang dia benar-benar letih.", 37],
  ["auto-pt", "PT", "Não vou chegar em casa até segunda.", 5],
  ["auto-fr", "FR", "Bonjour le monde", 42],
  ["auto-hi", "HI", "वह मेरे पिताजी की माँ है। वह मेरी दादी है।", 37],
  ["auto-ru", "RU", "Они были здесь.", 49],
  ["auto-ja", "JA", "私は２日間忙しくありません。", 0],
  ["auto-de", "DE", "Was ist das?", 0],
  ["auto-tr", "TR", "Çok büyük bir musibet.", 7],
  ["auto-vi", "VI", "Đây là 1 lời nói đùa cợt", 9],
  ["auto-ko", "KO", "물이 별로 없다.", 18],
  ["auto-it", "IT", "La incontrerai domani sera.", 0],
];

export const UNKNOWN_CASES = ["", "!@#$%^&*()", "😀🚀🧪❤️", "Hello"];

export const ALL_LOCALES = ["ar", "de", "en", "es", "fr", "hi", "id", "it", "ja", "ko", "pt", "ru", "tr", "vi", "zh"];

export const README_EXAMPLE = {
  text: "you are a stupid loser",
  options: { locales: ["en", "es"], detectLanguage: true, grawlix: true },
  verdict: { safe: false, score: 0.95, locale: "en", grawlix: "you are a @#$%&! @#$%&" },
};

export const INVALID_LOCALES = ["", "xx", "en-US"];

const ALIAS_TEXT = "Dia memberitahu saya yang dia benar-benar letih.";
const VERDICT_KEYS = "grawlix,locale,safe,score";
const EPSILON = 1e-9;

function canonicalLocale(code) {
  return (code === "ID" ? "MS" : code).toLowerCase();
}

function nearlyEqual(left, right) {
  return Math.abs(left - right) < EPSILON;
}

/** The shape and arithmetic every verdict must satisfy, whatever the text. */
export function invariantsHold(verdict, grawlixRequested) {
  if (verdict === null || typeof verdict !== "object") return false;
  if (Object.keys(verdict).sort().join(",") !== VERDICT_KEYS) return false;
  const { safe, score, locale, grawlix } = verdict;
  const hundredths = nearlyEqual(score * 100, Math.round(score * 100));
  const grawlixMatches = grawlixRequested && !safe ? typeof grawlix === "string" : grawlix === null;
  return typeof safe === "boolean"
    && Number.isFinite(score) && score >= 0 && score <= 1 && hundredths
    && safe === (score < 0.5)
    && (locale === null || /^[a-z]{2}$/.test(locale))
    && grawlixMatches;
}

async function rejection(promise) {
  try {
    const value = await promise;
    value?.close?.();
    return null;
  } catch (caught) {
    return { code: caught?.code ?? null, message: String(caught?.message ?? caught) };
  }
}

function failsOpen(verdict) {
  return invariantsHold(verdict, false) && verdict.safe === true && verdict.score === 0 && verdict.locale === null;
}

/** The module-level `init` and `judge`: the recommended API. */
async function singletonCases(api, withAssets) {
  const results = [];
  const record = (caseId, passed, detail) => results.push({ case_id: caseId, passed, ...detail });
  const { init, judge, ready, close } = api;
  record("singleton-exports", [init, judge, ready, close].every((value) => typeof value === "function"), {});

  close();
  const idle = judge(README_EXAMPLE.text);
  record("judge-before-init-fails-open", failsOpen(idle) && ready() === false, { verdict: idle });

  await init(withAssets(README_EXAMPLE.options));
  const verdict = judge(README_EXAMPLE.text);
  const expected = README_EXAMPLE.verdict;
  record("init-then-judge-readme", ready() && invariantsHold(verdict, true) && verdict.safe === expected.safe && nearlyEqual(verdict.score, expected.score) && verdict.locale === expected.locale && verdict.grawlix === expected.grawlix, { verdict });

  const again = init(withAssets(README_EXAMPLE.options));
  record("init-same-options-is-idempotent", again instanceof Promise && ready(), {});
  await again;

  const rejected = await rejection(init(withAssets({ locales: ["xx"] })));
  record("init-rejects-and-keeps-the-judge", rejected?.code === "BLASPHEM_LOCALE_UNSUPPORTED" && ready() && judge(README_EXAMPLE.text).locale === "en", { rejected });

  await init(withAssets({ locales: ["ko"], detectLanguage: true }));
  record("init-other-locales-replaces-the-judge", ready() && failsOpen(judge(README_EXAMPLE.text)), { verdict: judge(README_EXAMPLE.text) });

  close();
  record("close-then-judge-fails-open", ready() === false && failsOpen(judge("x")), {});
  return results;
}

function supplied(judge, [caseId, language, text, expectedNudge]) {
  const verdict = judge.judge(text);
  const invariantPassed = invariantsHold(verdict, true);
  const passed = invariantPassed && verdict.locale === canonicalLocale(language) && verdict.safe === !expectedNudge;
  return { case_id: caseId, language, expected_nudge: expectedNudge, actual_nudge: !verdict.safe, safe: verdict.safe, score: verdict.score, locale: verdict.locale, grawlix: verdict.grawlix, invariant_passed: invariantPassed, passed };
}

function automatic(autoJudge, explicitJudge, [caseId, language, text, expectedScore]) {
  const verdict = autoJudge.judge(text);
  const explicit = explicitJudge.judge(text);
  const invariantPassed = invariantsHold(verdict, false) && invariantsHold(explicit, true);
  const passed = invariantPassed
    && verdict.locale === canonicalLocale(language)
    && nearlyEqual(verdict.score, expectedScore / 100)
    && verdict.safe === explicit.safe
    && nearlyEqual(verdict.score, explicit.score);
  return { case_id: caseId, expected_language: language, expected_score: expectedScore / 100, locale: verdict.locale, safe: verdict.safe, score: verdict.score, explicit_score: explicit.score, invariant_passed: invariantPassed, passed };
}

function unknown(autoJudge, text) {
  const verdict = autoJudge.judge(text);
  const passed = invariantsHold(verdict, false) && verdict.safe === true && verdict.score === 0 && verdict.locale === null;
  return { text, safe: verdict.safe, score: verdict.score, locale: verdict.locale, passed };
}

async function packageCases(createJudge, withAssets, autoJudge) {
  const results = [];
  const record = (caseId, passed, detail) => results.push({ case_id: caseId, passed, ...detail });

  record("exports", typeof createJudge === "function", {});

  const readmeJudge = await createJudge(withAssets(README_EXAMPLE.options));
  const readme = readmeJudge.judge(README_EXAMPLE.text);
  const expected = README_EXAMPLE.verdict;
  record("readme-example", invariantsHold(readme, true) && readme.safe === expected.safe && nearlyEqual(readme.score, expected.score) && readme.locale === expected.locale && readme.grawlix === expected.grawlix, { verdict: readme });
  record("locales-in-registry-order", readmeJudge.locales.join(",") === "en,es" && Object.isFrozen(readmeJudge.locales), { locales: [...readmeJudge.locales] });
  record("transport-named", readmeJudge.transport === "wasm" || readmeJudge.transport === "native", { transport: readmeJudge.transport });

  const masked = await createJudge(withAssets({ locales: ["en"], detectLanguage: false, grawlix: true }));
  const unmasked = await createJudge(withAssets({ locales: ["en"], detectLanguage: false }));
  const maskedVerdict = masked.judge(README_EXAMPLE.text);
  const unmaskedVerdict = unmasked.judge(README_EXAMPLE.text);
  record("grawlix-only-when-requested", invariantsHold(maskedVerdict, true) && invariantsHold(unmaskedVerdict, false) && maskedVerdict.grawlix !== README_EXAMPLE.text && maskedVerdict.score === unmaskedVerdict.score, { masked: maskedVerdict.grawlix, unmasked: unmaskedVerdict.grawlix });
  for (const [caseId, text] of [["clean", "good morning everyone"], ["safe-profanity", "this damn printer is broken again"], ["empty", ""]]) {
    const verdict = masked.judge(text);
    const withoutGrawlix = unmasked.judge(text);
    record(`grawlix-safe-${caseId}`, invariantsHold(verdict, true) && verdict.safe === true && verdict.grawlix === null && verdict.score === withoutGrawlix.score && verdict.locale === withoutGrawlix.locale, { verdict });
  }
  const empty = unmasked.judge("");
  record("empty-text-explicit-locale", invariantsHold(empty, false) && empty.safe === true, { verdict: empty });
  record("default-detects-language", invariantsHold(autoJudge.judge("hello there"), false), {});

  masked.close();
  masked.close();
  let closedError = null;
  try {
    masked.judge("x");
  } catch (caught) {
    closedError = caught?.code ?? String(caught);
  }
  record("close-then-judge-throws", closedError === "BLASPHEM_CLOSED", { error: closedError });

  const emptyLocales = await rejection(createJudge(withAssets({ locales: [] })));
  const noLocales = await rejection(createJudge(withAssets({})));
  record("empty-locales-reject", emptyLocales?.code === "BLASPHEM_LOCALES_EMPTY" && noLocales?.code === "BLASPHEM_LOCALES_EMPTY", { emptyLocales, noLocales });

  readmeJudge.close();
  unmasked.close();
  return results;
}

async function aliasPassed(createJudge, withAssets) {
  const options = (code) => withAssets({ locales: [code], detectLanguage: false, grawlix: true });
  const ms = await createJudge(options("ms"));
  const id = await createJudge(options("id"));
  const left = ms.judge(ALIAS_TEXT);
  const right = id.judge(ALIAS_TEXT);
  const passed = left.locale === "ms" && right.locale === "ms" && ms.locales.join() === "ms" && id.locales.join() === "ms"
    && left.safe === right.safe && left.score === right.score && left.grawlix === right.grawlix;
  ms.close();
  id.close();
  return passed;
}

async function invalidLocales(createJudge, withAssets) {
  const results = [];
  for (const code of INVALID_LOCALES) {
    const error = await rejection(createJudge(withAssets({ locales: [code] })));
    results.push({ locale: code, rejected: error?.code === "BLASPHEM_LOCALE_UNSUPPORTED", error });
  }
  return results;
}

const passedCount = (list) => list.filter((entry) => entry.passed).length;

/**
 * Runs every case through a runtime module: `createJudge`, `init`, `judge`,
 * `ready`, `close`. `assets` is the base the transport reads from, or
 * undefined for the runtime's default.
 */
export async function runCases(api, assets) {
  const { createJudge } = api;
  const withAssets = (options) => (assets === undefined ? options : { ...options, assets });
  const autoJudge = await createJudge(withAssets({ locales: ALL_LOCALES, detectLanguage: true }));
  const explicit = new Map();
  for (const language of new Set(SUPPLIED_CASES.map(([, code]) => code))) {
    explicit.set(language, await createJudge(withAssets({ locales: [language.toLowerCase()], detectLanguage: false, grawlix: true })));
  }
  const explicitFor = (language) => explicit.get(language === "MS" ? "ID" : language);

  const cases = SUPPLIED_CASES.map((entry) => supplied(explicitFor(entry[1]), entry));
  const autoCases = AUTO_CASES.map((entry) => automatic(autoJudge, explicitFor(entry[1]), entry));
  const unknownCases = UNKNOWN_CASES.map((text) => unknown(autoJudge, text));
  const packageResults = [...(await packageCases(createJudge, withAssets, autoJudge)), ...(await singletonCases(api, withAssets))];
  const alias = await aliasPassed(createJudge, withAssets);
  const invalid = await invalidLocales(createJudge, withAssets);
  const transport = autoJudge.transport;
  for (const judge of explicit.values()) judge.close();
  autoJudge.close();

  const fragment = {
    transport,
    supplied_case_count: cases.length,
    passed_case_count: passedCount(cases),
    cases,
    auto_case_count: autoCases.length,
    passed_auto_case_count: passedCount(autoCases),
    auto_cases: autoCases,
    unknown_case_count: unknownCases.length,
    passed_unknown_case_count: passedCount(unknownCases),
    unknown_cases: unknownCases,
    package_case_count: packageResults.length,
    passed_package_case_count: passedCount(packageResults),
    package_cases: packageResults,
    ms_id_alias_passed: alias,
    invalid_locales: invalid,
    score_invariants_passed: [...cases, ...autoCases].every((entry) => entry.invariant_passed),
  };
  fragment.passed = fragment.passed_case_count === cases.length
    && fragment.passed_auto_case_count === autoCases.length
    && fragment.passed_unknown_case_count === unknownCases.length
    && fragment.passed_package_case_count === packageResults.length
    && fragment.ms_id_alias_passed
    && invalid.every((entry) => entry.rejected)
    && fragment.score_invariants_passed;
  return fragment;
}

/** The case count a fragment covers, across every table. */
export function caseTotal(fragment) {
  return fragment.supplied_case_count + fragment.auto_case_count + fragment.unknown_case_count + fragment.package_case_count;
}

/** Every failing entry of a fragment, for a terse failure log. */
export function failures(fragment) {
  const lists = [fragment.cases, fragment.auto_cases, fragment.unknown_cases, fragment.package_cases];
  const failed = lists.flat().filter((entry) => entry && !entry.passed);
  if (fragment.ms_id_alias_passed === false) failed.push({ case_id: "ms-id-alias", passed: false });
  for (const entry of fragment.invalid_locales ?? []) {
    if (!entry.rejected) failed.push({ case_id: `invalid-locale:${entry.locale}`, passed: false, error: entry.error });
  }
  return failed;
}

/** Verdict tables only, for comparing two transports. */
export function verdictSignature(fragment) {
  const strip = (entry) => ({ ...entry, invariant_passed: undefined, passed: undefined });
  return JSON.stringify([fragment.cases.map(strip), fragment.auto_cases.map(strip), fragment.unknown_cases.map(strip)]);
}
