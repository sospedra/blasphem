import { BlasphemJudge } from "./load.js";

/** Every option is optional. */
export interface JudgeOptions {
  /** Locale codes to load, such as ["en", "es"]. Defaults to all 15. */
  locales?: string[];
  /** Route by detected language. Defaults to true. */
  detectLanguage?: boolean;
  /** Populate `grawlix` on the result. Defaults to false. */
  grawlix?: boolean;
}

/** One verdict for one message. */
export interface Judgement {
  /** True when no nudge is due. Unroutable text is safe; the nudge fails open. */
  safe: boolean;
  /** Ordinal risk from 0 through 1. Not a probability. */
  score: number;
  /** The locale that produced the score, or null. */
  locale: string | null;
  /** The masked text when `grawlix` was requested, otherwise null. */
  grawlix: string | null;
}

const judges = new Map<string, BlasphemJudge>();

function keyFor(locales: string[], detectLanguage: boolean, grawlix: boolean): string {
  return `${locales.join(",")}|${detectLanguage}|${grawlix}`;
}

function judgeFor(locales: string[], detectLanguage: boolean, grawlix: boolean): BlasphemJudge {
  const key = keyFor(locales, detectLanguage, grawlix);
  const found = judges.get(key);
  if (found !== undefined) return found;

  const created = new BlasphemJudge(locales, detectLanguage, grawlix);
  judges.set(key, created);
  return created;
}

/**
 * Scores one message.
 *
 * Each distinct option set builds one judge and reuses it, so repeated
 * calls do not reload the lexica.
 */
export function judge(text: string, options: JudgeOptions = {}): Judgement {
  const locales = [...(options.locales ?? [])].sort();
  const instance = judgeFor(locales, options.detectLanguage ?? true, options.grawlix ?? false);
  return instance.judge(text) as Judgement;
}
