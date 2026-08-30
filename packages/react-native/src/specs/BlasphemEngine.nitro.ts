import type { HybridObject } from "react-native-nitro-modules";

/** One verdict as the native engine returns it. `locale` and `grawlix` are absent when null. */
export interface NativeJudgement {
  safe: boolean;
  score: number;
  locale?: string;
  grawlix?: string;
}

/** Collects one locale at a time, then builds the engine. Every error message starts with a contract code. */
export interface BlasphemEngineBuilder extends HybridObject<{ ios: "c++"; android: "c++" }> {
  configure(detectLanguage: boolean, grawlix: boolean): void;
  add(locale: string, pack: ArrayBuffer, packSha256?: string, detect?: ArrayBuffer, detectSha256?: string): void;
  build(): BlasphemEngine;
}

/** The native engine. `judge` is synchronous over JSI. */
export interface BlasphemEngine extends HybridObject<{ ios: "c++"; android: "c++" }> {
  readonly locales: string[];
  judge(text: string): NativeJudgement;
  close(): void;
}

/** Reads a pack file the app bundle carries: `manifest.json`, `<code>.pack`, `<code>.detect`. */
export interface BlasphemAssets extends HybridObject<{ ios: "swift"; android: "kotlin" }> {
  readBundled(name: string): Promise<ArrayBuffer>;
}
