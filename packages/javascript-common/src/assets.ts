import { fail } from "./errors.js";

/** Separate bases for the code and the data, when they are not served together. */
export interface AssetBases {
  /** Serves `blasphem_bg.wasm`. */
  wasm: string;
  /** Serves `manifest.json`, `<code>.pack`, and `<code>.detect`. */
  packs: string;
}

/** `assets: "jsdelivr"` loads this build's exact version of both packages from the jsDelivr npm CDN. */
export const JSDELIVR = "jsdelivr";

/** `blasphem` and `@blasphem/packs` share one version, baked in at build time. */
export function jsdelivrBases(version: string): AssetBases {
  return {
    wasm: `https://cdn.jsdelivr.net/npm/blasphem@${version}/dist`,
    packs: `https://cdn.jsdelivr.net/npm/@blasphem/packs@${version}/dist`,
  };
}

function trimSlash(base: string): string {
  return base.endsWith("/") ? base.slice(0, -1) : base;
}

function isBases(value: unknown): value is AssetBases {
  return typeof value === "object" && value !== null
    && typeof (value as AssetBases).wasm === "string" && (value as AssetBases).wasm.trim() !== ""
    && typeof (value as AssetBases).packs === "string" && (value as AssetBases).packs.trim() !== "";
}

/**
 * The browser needs a place for the wasm and a place for the packs.
 * Omitted or `"jsdelivr"`: the npm CDN at this build's pinned versions.
 * A path serves both from your origin; an object splits them.
 */
export function resolveBrowserAssets(input: unknown, version: string): AssetBases {
  if (input === undefined || input === null || input === JSDELIVR) return jsdelivrBases(version);
  if (isBases(input)) return { wasm: trimSlash(input.wasm), packs: trimSlash(input.packs) };
  if (typeof input === "string" && input.trim() !== "") {
    const base = trimSlash(input);
    return { wasm: base, packs: base };
  }
  throw fail("BLASPHEM_ASSETS_REQUIRED", `assets must be a path that serves blasphem_bg.wasm and the packs, "${JSDELIVR}", or { wasm, packs }`);
}

/** Node reads packs from a directory. Returns null when the caller wants the installed @blasphem/packs. */
export function resolvePacksDirectory(input: unknown): string | null {
  if (input === undefined || input === null) return null;
  if (isBases(input)) return input.packs;
  if (typeof input === "string" && input.trim() !== "") {
    if (input === JSDELIVR) throw fail("BLASPHEM_ASSETS_REQUIRED", `"${JSDELIVR}" is a browser preset; on Node install @blasphem/packs or pass a directory`);
    return input;
  }
  return null;
}
