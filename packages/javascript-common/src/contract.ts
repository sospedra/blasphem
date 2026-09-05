import type { AssetBases } from "./assets.js";

/** Options for one judge. `locales` is required. */
export interface JudgeOptions {
  /** Locale codes or every locale in this engine release. Empty arrays throw. */
  locales: string[] | "all";
  /**
   * Bundled by default. Remote clients accept "remote" and its "jsdelivr" alias.
   * Advanced browser instances accept asset bases. Node accepts a local directory.
   */
  assets?: string | AssetBases;
  /** Route by detected language. Native bundled assets inherit the bundle selection; otherwise defaults to true. */
  detectLanguage?: boolean;
  /** Populate `grawlix` for unsafe verdicts. Defaults to false. */
  grawlix?: boolean;
}

/** Default initialization reads the application's build configuration. */
export type InitOptions = Partial<JudgeOptions>;

/** One verdict for one message. Safe verdicts never include masked text. */
export type Judgement = {
  /** Ordinal risk from 0 through 1. Not a probability. */
  score: number;
  /** The locale that produced the score, or null. */
  locale: string | null;
} & (
  | { safe: true; grawlix: null }
  | { safe: false; grawlix: string | null }
);

/** A judge built once and called on every keystroke. */
export interface Judge {
  /** The loaded locales, in registry order. */
  readonly locales: readonly string[];
  /** Which engine answered: the wasm or a native binary. */
  readonly transport: "wasm" | "native";
  /** Scores one message. Synchronous. Never throws while the judge is open. */
  judge(text: string): Judgement;
  /** Releases the packs. Later `judge()` calls throw BLASPHEM_CLOSED. */
  close(): void;
}
