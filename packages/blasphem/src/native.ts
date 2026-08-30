import { createRequire } from "node:module";
import type { EngineHandle, Entry, Judgement } from "./core/index.js";

const require = createRequire(import.meta.url);

interface NativeBuilder {
  add(locale: string, pack: Uint8Array, packSha256?: string, detect?: Uint8Array, detectSha256?: string): void;
  build(): NativeEngine;
}

interface NativeEngine {
  readonly locales: string[];
  judge(text: string): { safe: boolean; score: number; locale?: string | null; grawlix?: string | null };
  close(): void;
}

export interface NativeModule {
  EngineBuilder: new (detectLanguage: boolean, grawlix: boolean) => NativeBuilder;
}

// One literal require per platform package. Deployment tracers such as
// @vercel/nft follow literals, so the installed binary ships with the app;
// a computed package name would be invisible to them.
const LOADERS: Record<string, () => NativeModule> = {
  "darwin-arm64": () => require("@blasphem/node-darwin-arm64") as NativeModule,
  "darwin-x64": () => require("@blasphem/node-darwin-x64") as NativeModule,
  "linux-arm64-gnu": () => require("@blasphem/node-linux-arm64-gnu") as NativeModule,
  "linux-arm64-musl": () => require("@blasphem/node-linux-arm64-musl") as NativeModule,
  "linux-x64-gnu": () => require("@blasphem/node-linux-x64-gnu") as NativeModule,
  "linux-x64-musl": () => require("@blasphem/node-linux-x64-musl") as NativeModule,
  "win32-x64-msvc": () => require("@blasphem/node-win32-x64-msvc") as NativeModule,
};

function libc(): string {
  const report = process.report?.getReport?.() as { header?: { glibcVersionRuntime?: unknown } } | undefined;
  return typeof report?.header?.glibcVersionRuntime === "string" ? "gnu" : "musl";
}

/** The platform key this machine's binary is published under, or null when none exists. */
export function nativeTarget(): string | null {
  const { platform, arch } = process;
  if (platform === "darwin" && (arch === "arm64" || arch === "x64")) return `darwin-${arch}`;
  if (platform === "linux" && (arch === "arm64" || arch === "x64")) return `linux-${arch}-${libc()}`;
  if (platform === "win32" && arch === "x64") return "win32-x64-msvc";
  return null;
}

/** Loads the native binary, or returns null so the caller falls back to wasm. `BLASPHEM_FORCE_WASM=1` skips it. */
export function loadNative(): NativeModule | null {
  if (process.env.BLASPHEM_FORCE_WASM === "1") return null;
  const target = nativeTarget();
  const load = target === null ? undefined : LOADERS[target];
  if (load === undefined) return null;
  try {
    return load();
  } catch {
    return null;
  }
}

export function buildNativeEngine(native: NativeModule, entries: Entry[], detectLanguage: boolean, grawlix: boolean): EngineHandle {
  const builder = new native.EngineBuilder(detectLanguage, grawlix);
  for (const entry of entries) {
    builder.add(entry.locale, entry.pack, entry.packSha256, entry.detect ?? undefined, entry.detectSha256 ?? undefined);
  }
  const engine = builder.build();
  return {
    locales: engine.locales,
    // napi drops absent optionals; the contract promises null.
    judge: (text: string): Judgement => {
      const verdict = engine.judge(text);
      return { safe: verdict.safe, score: verdict.score, locale: verdict.locale ?? null, grawlix: verdict.grawlix ?? null };
    },
    free: (): void => engine.close(),
  };
}
