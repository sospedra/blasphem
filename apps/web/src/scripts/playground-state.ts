import type { Judgement } from "blasphem";

export type Phase =
  | { status: "idle" }
  | { status: "unavailable" }
  | { status: "loading" }
  | { status: "ready" }
  | { status: "error"; message: string };

export type PhaseEvent =
  | { type: "LOAD" }
  | { type: "LOADED" }
  | { type: "FAILED"; message: string }
  | { type: "RETRY" }
  | { type: "UNAVAILABLE" };

export function transition(phase: Phase, event: PhaseEvent): Phase {
  switch (event.type) {
    case "LOAD":
      return phase.status === "idle" || phase.status === "ready" ? { status: "loading" } : phase;
    case "LOADED":
      return phase.status === "loading" ? { status: "ready" } : phase;
    case "FAILED":
      return phase.status === "loading" || phase.status === "ready" ? { status: "error", message: event.message } : phase;
    case "RETRY":
      return phase.status === "error" ? { status: "idle" } : phase;
    case "UNAVAILABLE":
      return { status: "unavailable" };
  }
}

export type Snapshot = Judgement;

export type Tone = "waiting" | "clean" | "hit" | "unknown";
export type Verdict = { word: string; tone: Tone; note: string };

export const WAITING: Verdict = { word: "Awaiting", tone: "waiting", note: "write a message, or load a sample" };
const UNHEARD: Verdict = { word: "Unheard", tone: "unknown", note: "no locale routed this text, the nudge fails open" };
const CONDEMNED: Verdict = { word: "Condemned", tone: "hit", note: "safe is false, the pre-send nudge fires" };
const ABSOLVED: Verdict = { word: "Absolved", tone: "clean", note: "safe is true, no nudge" };

export function verdictFor(snapshot: Pick<Snapshot, "safe" | "locale">): Verdict {
  if (snapshot.locale === null) return UNHEARD;
  return snapshot.safe ? ABSOLVED : CONDEMNED;
}

export function statusCopy(phase: Phase, megabytes: string): string {
  const copy: Record<Phase["status"], string> = {
    idle: `The module sleeps until you type. Your first keystroke fetches ${megabytes} once. Nothing you write leaves this page.`,
    loading: `Fetching ${megabytes} and waking the judge. Keep typing; the verdict lands when it does.`,
    ready: "Awake. Every keystroke is judged in this page and timed.",
    error: "The judge failed to wake.",
    unavailable: "The browser package is not built. Run pnpm --filter blasphem run build, then rebuild the site.",
  };
  return copy[phase.status];
}
