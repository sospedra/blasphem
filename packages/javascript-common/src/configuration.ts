import { fail } from "./errors.js";
import { normalizeLocales, selectedFiles } from "./locales.js";
import type { ManifestFile } from "./manifest.js";

export interface BundleConfiguration {
  formatVersion: 1;
  engineVersion: string;
  dataVersion: string;
  locales: ReturnType<typeof normalizeLocales>;
  assets: "bundled" | "remote";
  detectLanguage: boolean;
  files: string[];
  assetUrls?: Record<string, string>;
  wasm?: ManifestFile & { url: string };
  manifest?: ManifestFile;
}

function configurationObject(input: unknown): Record<string, unknown> {
  if (typeof input !== "object" || input === null || Array.isArray(input)) {
    throw fail("BLASPHEM_ASSETS_REQUIRED", "Declare blasphem.locales in the application configuration");
  }
  return input as Record<string, unknown>;
}

function delivery(input: unknown): BundleConfiguration["assets"] {
  if (input === undefined || input === "bundled") return "bundled";
  if (input === "remote" || input === "jsdelivr") return "remote";
  throw fail("BLASPHEM_ASSETS_REQUIRED", "Configured assets must be bundled or remote");
}

export function resolveConfiguration(input: unknown, version: string): BundleConfiguration {
  const config = configurationObject(input);
  const locales = normalizeLocales(config.locales);
  const assets = delivery(config.assets);
  const detectLanguage = config.detectLanguage ?? true;
  if (typeof detectLanguage !== "boolean") throw fail("BLASPHEM_PACK_INVALID", "detectLanguage must be a boolean");
  return {
    formatVersion: 1, engineVersion: version, dataVersion: version,
    locales, assets, detectLanguage, files: selectedFiles(locales, detectLanguage),
  };
}

export function parseBundle(input: unknown, version: string): BundleConfiguration {
  const config = configurationObject(input);
  if (config.formatVersion !== 1 || config.engineVersion !== version || config.dataVersion !== version) {
    throw fail("BLASPHEM_FORMAT_VERSION", "bundle.json must match this engine release and format");
  }
  const resolved = resolveConfiguration(config, version);
  if (JSON.stringify(config.files) !== JSON.stringify(resolved.files)) {
    throw fail("BLASPHEM_PACK_INVALID", "bundle.json files do not match its locale selection");
  }
  return { ...resolved, assetUrls: assetUrls(config.assetUrls), wasm: wasmRecord(config.wasm), manifest: integrityRecord(config.manifest) };
}

function assetUrls(input: unknown): Record<string, string> | undefined {
  if (input === undefined) return undefined;
  const entries = Object.entries(configurationObject(input));
  if (entries.some(([, url]) => typeof url !== "string" || url.length === 0)) {
    throw fail("BLASPHEM_PACK_INVALID", "bundle.json asset URLs must be nonempty strings");
  }
  return Object.fromEntries(entries) as Record<string, string>;
}

function integrityRecord(input: unknown): ManifestFile | undefined {
  if (input === undefined) return undefined;
  const record = configurationObject(input);
  if (!Number.isSafeInteger(record.bytes) || (record.bytes as number) <= 0 || typeof record.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(record.sha256)) {
    throw fail("BLASPHEM_PACK_INVALID", "bundle.json integrity metadata is invalid");
  }
  return { bytes: record.bytes as number, sha256: record.sha256 };
}

function wasmRecord(input: unknown): BundleConfiguration["wasm"] {
  const record = integrityRecord(input);
  if (record === undefined) return undefined;
  const { url } = configurationObject(input);
  if (typeof url !== "string" || url.length === 0) throw fail("BLASPHEM_PACK_INVALID", "bundle.json WASM URL is invalid");
  return { ...record, url };
}
