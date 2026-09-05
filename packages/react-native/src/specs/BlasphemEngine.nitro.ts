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

export interface DownloadIntegrity {
  bytes: number;
  sha256: string;
}

/** Native bundle reads and persistent, verified CDN downloads. */
export interface BlasphemAssets extends HybridObject<{ ios: "swift"; android: "kotlin" }> {
  readBundled(name: string): Promise<ArrayBuffer>;
  readManifest(url: string, refresh: boolean): Promise<ArrayBuffer>;
  /** Called only after JavaScript validates the manifest schema. */
  commitManifest(url: string, bytes: ArrayBuffer): Promise<void>;
  readDownloaded(url: string, expected: DownloadIntegrity): Promise<ArrayBuffer>;
}
