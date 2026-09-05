import { BlasphemEngineBuilder } from "./blasphem.js";
import type { EngineHandle, Entry, Judgement } from "./core/index.js";

/** Feeds the entries to the wasm builder and wraps the engine it returns. */
export function buildWasmEngine(entries: Entry[], detectLanguage: boolean, grawlix: boolean): EngineHandle {
  const builder = new BlasphemEngineBuilder(detectLanguage, grawlix);
  let built = false;
  try {
    for (const entry of entries) {
      builder.add(entry.locale, entry.pack, entry.packSha256, entry.detect ?? undefined, entry.detectSha256 ?? undefined);
    }
    built = true;
    const engine = builder.build();
    return {
      locales: engine.locales,
      judge: (text: string): Judgement => engine.judge(text),
      free: (): void => engine.free(),
    };
  } finally {
    // build() consumes the builder. Only an add() failure leaves it alive.
    if (!built) builder.free();
  }
}
