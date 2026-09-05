import { fail } from "./errors.js";

export const MANIFEST_FORMAT_VERSION = 1;
export const MANIFEST_FILE = "manifest.json";

export interface ManifestFile {
  bytes: number;
  sha256: string;
}

export interface Manifest {
  formatVersion: typeof MANIFEST_FORMAT_VERSION;
  files: Record<string, ManifestFile>;
}

const HEX_64 = /^[0-9a-f]{64}$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Parses `manifest.json` from `@blasphem/packs`. Rejects a version this core does not read. */
export function parseManifest(bytes: Uint8Array): Manifest {
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder().decode(bytes));
  } catch (error) {
    throw fail("BLASPHEM_PACK_INVALID", `${MANIFEST_FILE} is not JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
  if (!isRecord(parsed)) throw fail("BLASPHEM_PACK_INVALID", `${MANIFEST_FILE} must be an object`);
  if (parsed.formatVersion !== MANIFEST_FORMAT_VERSION) {
    throw fail("BLASPHEM_FORMAT_VERSION", `${MANIFEST_FILE} has format version ${String(parsed.formatVersion)}, this build accepts ${MANIFEST_FORMAT_VERSION}`);
  }
  if (!isRecord(parsed.files)) throw fail("BLASPHEM_PACK_INVALID", `${MANIFEST_FILE} lacks a files map`);
  const files: Record<string, ManifestFile> = {};
  for (const [name, record] of Object.entries(parsed.files)) {
    if (!validFile(name, record)) {
      throw fail("BLASPHEM_PACK_INVALID", `${MANIFEST_FILE} entry ${JSON.stringify(name)} needs bytes and a 64-character sha256`);
    }
    files[name] = { bytes: record.bytes, sha256: record.sha256 };
  }
  return { formatVersion: MANIFEST_FORMAT_VERSION, files };
}

function validFile(name: string, record: unknown): record is ManifestFile {
  if (!/^[a-z]{2,3}\.(pack|detect)$/.test(name) || !isRecord(record)) return false;
  const validLength = typeof record.bytes === "number" && Number.isSafeInteger(record.bytes) && record.bytes > 0;
  return validLength && typeof record.sha256 === "string" && HEX_64.test(record.sha256);
}
