import type { JudgeOptions, Judgement } from "blasphem";
import { normalizeSelection, type Selection } from "../lib/languages";
import { statusCopy, transition, verdictFor, WAITING, type Phase, type PhaseEvent, type Snapshot } from "./playground-state";

type Module = typeof import("blasphem");

const BASE = __BLASPHEM_BASE__;
const MEGABYTES = `${(__BLASPHEM_WASM_BYTES__ / 1_000_000).toFixed(1)} MB`;
/** The nudge boundary is fixed at 50 of 100, so 0.5 on the returned score. */
const NUDGE_THRESHOLD = 0.5;
/** The first call in a burst runs cold and Safari rounds performance.now() to 1 ms, so the mean covers at least five runs and stops once the clock has moved 3 ms. */
const TIMING_MIN_RUNS = 5;
const TIMING_BUDGET_MS = 3;
const TIMING_MAX_RUNS = 300;
const FIELD_IDS = ["f-safe", "f-score", "f-locale", "f-grawlix"] as const;

type FieldId = (typeof FIELD_IDS)[number];
type Sample = { code: string; tag: string; text: string };

type Elements = {
  root: HTMLElement;
  message: HTMLTextAreaElement;
  language: HTMLSelectElement;
  sample: HTMLButtonElement;
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
  module: Module | null;
  sampleIndex: number;
};

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
    sample: required(root, "#sample"),
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

/**
 * The package entry instantiates the WebAssembly module through a top-level
 * await, so judge() is callable as soon as this import resolves.
 */
async function loadModule(): Promise<Module> {
  return (await import(/* @vite-ignore */ `${BASE}/index.js`)) as Module;
}

function optionsFor(selection: Selection): JudgeOptions {
  if (selection === "AUTO") return { detectLanguage: true, grawlix: true };
  return { locales: [selection.toLowerCase()], detectLanguage: false, grawlix: true };
}

function snapshot(verdict: Judgement): Snapshot {
  return { safe: verdict.safe, score: verdict.score, locale: verdict.locale, grawlix: verdict.grawlix };
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
  elements.sample.disabled = phase.status === "unavailable";
  if (phase.status === "loading") elements.clock.textContent = "waking";
}

function renderWaiting(elements: Elements): void {
  elements.verdict.dataset.tone = WAITING.tone;
  elements.ruling.textContent = WAITING.word;
  elements.note.textContent = WAITING.note;
  elements.clock.textContent = "—";
  elements.bar.style.transform = "scaleX(0)";
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
}

function applyQuerySelection(select: HTMLSelectElement): void {
  const requested = new URLSearchParams(location.search).get("lang");
  if (!requested) return;
  const selection = normalizeSelection(requested);
  if (selection) select.value = selection;
}

export function mountPlayground(root: HTMLElement): void {
  const elements = collect(root);
  const samples = readSamples(document);
  const session: Session = { phase: { status: "idle" }, module: null, sampleIndex: -1 };

  const dispatch = (event: PhaseEvent): void => {
    session.phase = transition(session.phase, event);
    renderPhase(elements, session.phase);
  };

  const currentSelection = (): Selection => normalizeSelection(elements.language.value) ?? "AUTO";

  const evaluate = (module: Module): void => {
    const text = elements.message.value;
    if (text.trim() === "") {
      renderWaiting(elements);
      return;
    }
    const options = optionsFor(currentSelection());
    const { verdict, perCallMs } = timeJudge(() => module.judge(text, options));
    renderResult(elements, snapshot(verdict), perCallMs);
  };

  const ensureModule = async (): Promise<Module | null> => {
    if (session.module) return session.module;
    if (session.phase.status !== "idle") return null;
    dispatch({ type: "LOAD" });
    try {
      session.module = await loadModule();
    } catch (error) {
      dispatch({ type: "FAILED", message: describe(error) });
      return null;
    }
    dispatch({ type: "LOADED" });
    return session.module;
  };

  const check = async (): Promise<void> => {
    if (BASE === "") {
      dispatch({ type: "UNAVAILABLE" });
      return;
    }
    const module = await ensureModule();
    if (!module) return;
    warm(module, currentSelection());
    evaluate(module);
  };

  const warmed = new Set<Selection>();
  function warm(module: Module, selection: Selection): void {
    if (warmed.has(selection)) return;
    warmed.add(selection);
    module.judge("blasphem", optionsFor(selection));
  }

  const samplesFor = (selection: Selection): readonly Sample[] => {
    if (selection === "AUTO") return samples;
    return samples.filter((sample) => sample.code === selection);
  };

  const loadSample = (): void => {
    const pool = samplesFor(currentSelection());
    if (pool.length === 0) return;
    session.sampleIndex = (session.sampleIndex + 1) % pool.length;
    const sample = pool[session.sampleIndex];
    elements.message.value = sample.text;
    elements.message.lang = sample.tag;
    elements.message.focus();
    void check();
  };

  elements.message.addEventListener("input", () => {
    elements.message.lang = "";
    void check();
  });
  elements.language.addEventListener("change", () => {
    session.sampleIndex = -1;
    void check();
  });
  elements.sample.addEventListener("click", loadSample);
  elements.retry.addEventListener("click", () => {
    dispatch({ type: "RETRY" });
    void check();
  });

  applyQuerySelection(elements.language);
  renderWaiting(elements);
  renderPhase(elements, session.phase);
}
