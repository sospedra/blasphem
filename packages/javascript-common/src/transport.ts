import type { Judgement } from "./contract.js";

/** One locale's bytes plus the digests the manifest promised. */
export interface Entry {
  locale: string;
  pack: Uint8Array;
  packSha256: string;
  detect: Uint8Array | null;
  detectSha256: string | null;
}

/** What the wasm or the native binary hands back. */
export interface EngineHandle {
  readonly locales: readonly string[];
  judge(text: string): Judgement;
  free(): void;
}

/** A runtime's way to read a file by name and to start an engine. */
export interface Transport {
  readonly name: "wasm" | "native";
  read(name: string): Promise<Uint8Array>;
  engine(entries: Entry[], detectLanguage: boolean, grawlix: boolean): Promise<EngineHandle>;
}
