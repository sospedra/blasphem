import type { Judge, JudgeOptions, Judgement } from "blasphem";
import { LANGUAGES, normalizeSelection, type Selection } from "../lib/languages";
import { statusCopy, transition, verdictFor, WAITING, type Phase, type PhaseEvent, type Snapshot } from "./playground-state";
import { copyText, resetCopy } from "./clipboard";

type Module = typeof import("blasphem");

const BASE = __BLASPHEM_BASE__;
const MEGABYTES = `${(__BLASPHEM_TOTAL_BYTES__ / 1_048_576).toFixed(2)} MB`;
/** The first call in a burst runs cold and Safari rounds performance.now() to 1 ms, so the mean covers at least five runs and stops once the clock has moved 3 ms. */
const TIMING_MIN_RUNS = 5;
const TIMING_BUDGET_MS = 3;
const TIMING_MAX_RUNS = 300;
const FIELD_IDS = ["f-safe", "f-score", "f-locale", "f-grawlix"] as const;
const ALL_LOCALES = LANGUAGES.map((language) => language.tag);

type FieldId = (typeof FIELD_IDS)[number];
type SampleKind = "toxic" | "clean";
type Sample = { code: string; tag: string; kind: SampleKind; name: string; text: string };
type JudgeEntry = { status: "loading"; promise: Promise<Judge> } | { status: "ready"; judge: Judge };

type Elements = {
  root: HTMLElement;
  message: HTMLTextAreaElement;
  language: HTMLSelectElement;
  samples: NodeListOf<HTMLButtonElement>;
  sampleNote: HTMLElement;
  copy: HTMLButtonElement;
  copyStatus: HTMLElement;
  status: HTMLElement;
  verdict: HTMLElement;
  ruling: HTMLElement;
  note: HTMLElement;
  announce: HTMLElement;
  clock: HTMLElement;
  bar: HTMLElement;
  failure: HTMLElement;
  failureMessage: HTMLElement;
  retry: HTMLButtonElement;
  fields: Record<FieldId, HTMLElement>;
};

type Session = {
  phase: Phase;
  module: Promise<Module> | null;
  moduleAttempt: number;
  judges: Map<Selection, JudgeEntry>;
  sampleIndex: Record<SampleKind, number>;
  sampleText: string | null;
  revision: number;
};

type Playground = { elements: Elements; samples: readonly Sample[]; session: Session; lifetime: AbortController };
type CheckRequest = { text: string; selection: Selection; revision: number };

function required<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelector<T>(selector);
  if (!found) throw new Error(`playground markup lacks ${selector}`);
  return found;
}

function collect(root: HTMLElement): Elements {
  const fields = Object.fromEntries(FIELD_IDS.map((id) => [id, required<HTMLElement>(root, `#${id}`)])) as Record<FieldId, HTMLElement>;
  return {
    root,
    message: required(root, "#message"),
    language: required(root, "#language"),
    samples: root.querySelectorAll<HTMLButtonElement>(".sample"),
    sampleNote: required(root, "#sample-note"),
    copy: required(root, "#copy-message"),
    copyStatus: required(root, "#copy-status"),
    status: required(root, "#status"),
    verdict: required(root, "#verdict"),
    ruling: required(root, "#ruling"),
    note: required(root, "#note"),
    announce: required(root, "#announce"),
    clock: required(root, "#clock"),
    bar: required(root, "#bar"),
    failure: required(root, "#failure"),
    failureMessage: required(root, "#failure-message"),
    retry: required(root, "#retry"),
    fields,
  };
}

function readSamples(root: ParentNode): readonly Sample[] {
  const holder = root.querySelector("#sample-data");
  return holder?.textContent ? (JSON.parse(holder.textContent) as Sample[]) : [];
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** The package's browser entry, served beside the wasm and the packs under BASE. */
async function loadModule(attempt: number): Promise<Module> {
  const retry = attempt > 0 ? `?retry=${attempt}` : "";
  return (await import(/* @vite-ignore */ `${BASE}/browser.js${retry}`)) as Module;
}

function optionsFor(selection: Selection): JudgeOptions {
  return { locales: [selection.toLowerCase()], detectLanguage: false, grawlix: true };
}

function snapshot(verdict: Judgement): Snapshot {
  return { ...verdict };
}

function timeJudge(judge: () => Judgement): { verdict: Judgement; perCallMs: number } {
  const started = performance.now();
  const verdict = judge();
  let runs = 1;
  const withinBudget = (): boolean => performance.now() - started < TIMING_BUDGET_MS && runs < TIMING_MAX_RUNS;
  while (runs < TIMING_MIN_RUNS || withinBudget()) {
    judge();
    runs += 1;
  }
  return { verdict, perCallMs: (performance.now() - started) / runs };
}

function renderPhase(elements: Elements, phase: Phase): void {
  elements.root.dataset.state = phase.status;
  elements.status.textContent = statusCopy(phase, MEGABYTES);
  elements.failure.hidden = phase.status !== "error";
  elements.failureMessage.textContent = phase.status === "error" ? phase.message : "";
  elements.message.disabled = phase.status === "unavailable";
  for (const sample of elements.samples) sample.disabled = phase.status === "unavailable";
  elements.clock.textContent = phase.status === "loading" ? "waking" : "—";
}

function renderWaiting(elements: Elements): void {
  elements.verdict.dataset.tone = WAITING.tone;
  elements.ruling.textContent = WAITING.word;
  elements.note.textContent = WAITING.note;
  elements.clock.textContent = "—";
  elements.bar.style.transform = "scaleX(0)";
  elements.copy.disabled = true;
  resetCopy(elements.copy, elements.copyStatus);
  for (const id of FIELD_IDS) elements.fields[id].textContent = "—";
}

function announceChange(elements: Elements, word: string, note: string): void {
  const summary = `${word}. ${note}`;
  if (elements.announce.textContent === summary) return;
  elements.announce.textContent = summary;
}

function renderResult(elements: Elements, taken: Snapshot, elapsedMs: number): void {
  const verdict = verdictFor(taken);
  elements.verdict.dataset.tone = verdict.tone;
  elements.ruling.textContent = verdict.word;
  elements.note.textContent = verdict.note;
  announceChange(elements, verdict.word, verdict.note);
  elements.clock.textContent = elapsedMs > 0 ? `${elapsedMs.toFixed(2)} ms per judge()` : "under 0.01 ms per judge()";
  elements.bar.style.transform = `scaleX(${taken.score})`;
  elements.fields["f-safe"].textContent = String(taken.safe);
  elements.fields["f-score"].textContent = taken.score.toFixed(2);
  elements.fields["f-locale"].textContent = taken.locale ?? "null";
  elements.fields["f-grawlix"].textContent = taken.grawlix ?? "null";
  elements.copy.disabled = taken.grawlix === null;
}

function applyQuerySelection(select: HTMLSelectElement): void {
  const requested = new URLSearchParams(location.search).get("lang");
  if (!requested) return;
  const selection = normalizeSelection(requested);
  if (selection) select.value = selection;
}

function currentSelection(elements: Elements): Selection {
  return normalizeSelection(elements.language.value) ?? "AUTO";
}

function dispatch(playground: Playground, event: PhaseEvent): void {
  playground.session.phase = transition(playground.session.phase, event);
  renderPhase(playground.elements, playground.session.phase);
}

function moduleFor(session: Session): Promise<Module> {
  if (session.module) return session.module;
  session.module = loadModule(session.moduleAttempt++).catch((error: unknown) => {
    session.module = null;
    throw error;
  });
  return session.module;
}

async function loadJudge(session: Session, selection: Selection): Promise<Judge> {
  const module = await moduleFor(session);
  const judge = selection === "AUTO" ? await defaultJudge(module) : await module.createJudge(optionsFor(selection));
  session.judges.set(selection, { status: "ready", judge });
  return judge;
}

async function defaultJudge(module: Module): Promise<Judge> {
  await module.init({ grawlix: true });
  return { locales: ALL_LOCALES, transport: "wasm", judge: module.judge, close: module.close };
}

function judgeFor(session: Session, selection: Selection): Promise<Judge> {
  const existing = session.judges.get(selection);
  if (existing?.status === "ready") return Promise.resolve(existing.judge);
  if (existing?.status === "loading") return existing.promise;
  const promise = loadJudge(session, selection).catch((error: unknown) => {
    session.judges.delete(selection);
    throw error;
  });
  session.judges.set(selection, { status: "loading", promise });
  return promise;
}

function isCurrent(playground: Playground, request: CheckRequest): boolean {
  return !playground.lifetime.signal.aborted && request.revision === playground.session.revision;
}

function stampSample(playground: Playground, taken: Snapshot): void {
  const { elements, session } = playground;
  const firstSample = taken.locale !== null && session.sampleText === elements.message.value && !elements.verdict.hasAttribute("data-stamped");
  if (firstSample) elements.verdict.setAttribute("data-stamped", "");
}

async function evaluate(playground: Playground, request: CheckRequest): Promise<void> {
  try {
    const judge = await judgeFor(playground.session, request.selection);
    if (!isCurrent(playground, request)) return;
    if (playground.elements.message.value.trim() === "") {
      dispatch(playground, { type: "LOADED" });
      renderWaiting(playground.elements);
      return;
    }
    const { verdict, perCallMs } = timeJudge(() => judge.judge(request.text));
    dispatch(playground, { type: "LOADED" });
    const taken = snapshot(verdict);
    renderResult(playground.elements, taken, perCallMs);
    stampSample(playground, taken);
  } catch (error) {
    if (!isCurrent(playground, request)) return;
    dispatch(playground, { type: "FAILED", message: describe(error) });
  }
}

function check(playground: Playground): void {
  const { elements, session } = playground;
  resetCopy(elements.copy, elements.copyStatus);
  if (elements.message.value.trim() === "") {
    renderWaiting(elements);
    return;
  }
  const request = { text: elements.message.value, selection: currentSelection(elements), revision: ++session.revision };
  if (BASE === "") {
    dispatch(playground, { type: "UNAVAILABLE" });
    return;
  }
  if (session.phase.status === "error") return;
  const needsLoading = session.judges.get(request.selection)?.status !== "ready" || session.phase.status === "idle";
  if (needsLoading) {
    renderWaiting(elements);
    dispatch(playground, { type: "LOAD" });
  }
  void evaluate(playground, request);
}

function loadSample(playground: Playground, kind: SampleKind): void {
  const { elements, session } = playground;
  const selection = currentSelection(elements);
  const pool = playground.samples.filter((sample) => sample.kind === kind && (selection === "AUTO" || sample.code === selection));
  if (pool.length === 0) return;
  const index = (session.sampleIndex[kind] + 1) % pool.length;
  session.sampleIndex[kind] = index;
  const sample = pool[index];
  session.sampleText = sample.text;
  elements.message.value = sample.text;
  elements.message.lang = sample.tag;
  elements.sampleNote.textContent = `${sample.name} · ${kind === "toxic" ? "hostile" : "clean"} example`;
  check(playground);
}

function editMessage(playground: Playground): void {
  playground.session.sampleText = null;
  playground.elements.message.lang = "";
  if (playground.elements.sampleNote.textContent !== "Your message") playground.elements.sampleNote.textContent = "Your message";
  check(playground);
}

function bindInputs(playground: Playground): void {
  const { elements, session, lifetime } = playground;
  const options = { signal: lifetime.signal };
  elements.message.addEventListener("input", () => editMessage(playground), options);
  elements.language.addEventListener("change", () => {
    session.sampleIndex = { toxic: -1, clean: -1 };
    check(playground);
  }, options);
  for (const button of elements.samples) {
    button.addEventListener("click", () => loadSample(playground, button.dataset.kind === "clean" ? "clean" : "toxic"), options);
  }
  elements.retry.addEventListener("click", () => {
    dispatch(playground, { type: "RETRY" });
    check(playground);
  }, options);
  elements.copy.addEventListener("click", () => {
    void copyText(elements.copy, elements.copyStatus, () => elements.copy.disabled ? "" : elements.fields["f-grawlix"].textContent ?? "");
  }, options);
}

function closePlayground(playground: Playground): void {
  playground.lifetime.abort();
  for (const entry of playground.session.judges.values()) {
    if (entry.status === "ready") entry.judge.close();
    if (entry.status === "loading") void entry.promise.then((judge) => judge.close(), () => undefined);
  }
}

export function mountPlayground(root: HTMLElement): () => void {
  const elements = collect(root);
  const playground: Playground = {
    elements,
    samples: readSamples(document),
    session: {
      phase: { status: "idle" },
      module: null,
      moduleAttempt: 0,
      judges: new Map(),
      sampleIndex: { toxic: -1, clean: -1 },
      sampleText: null,
      revision: 0,
    },
    lifetime: new AbortController(),
  };
  bindInputs(playground);
  applyQuerySelection(elements.language);
  renderWaiting(elements);
  renderPhase(elements, playground.session.phase);
  return () => closePlayground(playground);
}
