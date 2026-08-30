import type { AssetBases } from "./assets.js";

/** Options for one judge. `locales` is required. */
export interface JudgeOptions {
  /** Lowercase locale codes to load, such as ["en", "es"]. Empty throws. */
  locales: string[];
  /**
   * Where the bytes come from. Browser: omitted means the jsDelivr npm CDN at
   * this build's exact versions; a path serves the wasm and the packs from
   * your origin; `{ wasm, packs }` splits them. Node: omitted means the
   * installed `@blasphem/packs`; a string is a packs directory. React Native:
   * ignored, packs come from the app bundle.
   */
  assets?: string | AssetBases;
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
