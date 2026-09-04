import { createHash } from "node:crypto";
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const canonicalPacks = fileURLToPath(new URL("../resources/packs/", import.meta.url));

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function packName(name) {
  if (basename(name) !== name) throw new Error(`Invalid pack name: ${name}`);
  const [code, kind, extra] = name.split(".");
  const validCode = code.length === 2 && [...code].every((letter) => letter >= "a" && letter <= "z");
  if (!validCode || extra !== undefined || !["pack", "detect"].includes(kind)) {
    throw new Error(`Invalid pack name: ${name}`);
  }
}

function readPack(source, name, record) {
  packName(name);
  const path = resolve(source, name);
  if (!lstatSync(path).isFile()) throw new Error(`Expected a regular file: ${path}`);
  const bytes = readFileSync(path);
  if (!Number.isSafeInteger(record?.bytes) || record.bytes <= 0) throw new Error(`Invalid size: ${name}`);
  if (bytes.length !== record.bytes || digest(bytes) !== record.sha256) {
    throw new Error(`Pack integrity mismatch: ${path}`);
  }
  return { name, bytes, sha256: record.sha256 };
}

export function loadPacks(source = canonicalPacks) {
  const manifestBytes = readFileSync(resolve(source, "manifest.json"));
  const manifest = JSON.parse(manifestBytes);
  if (manifest.formatVersion !== 1 || !manifest.files || Array.isArray(manifest.files)) {
    throw new Error(`Invalid packs manifest: ${source}`);
  }
  const names = Object.keys(manifest.files).sort();
  if (names.length === 0) throw new Error(`Empty packs manifest: ${source}`);
  const files = names.map((name) => readPack(source, name, manifest.files[name]));
  const unlisted = readdirSync(source).filter((name) => name.endsWith(".pack") || name.endsWith(".detect"))
    .filter((name) => !Object.hasOwn(manifest.files, name));
  if (unlisted.length) throw new Error(`Unlisted packs: ${unlisted.join(", ")}`);
  return { manifest, manifestBytes, files };
}

export function copyPack(file, destination) {
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, file.bytes);
  if (digest(readFileSync(destination)) !== file.sha256) throw new Error(`Export integrity mismatch: ${destination}`);
}

export function replaceDirectory(staged, destination) {
  const backup = `${staged}.previous`;
  const previous = existsSync(destination);
  if (previous) renameSync(destination, backup);
  try {
    renameSync(staged, destination);
  } catch (error) {
    if (previous) renameSync(backup, destination);
    throw error;
  }
  if (previous) rmSync(backup, { recursive: true });
}

export function exportPacks(destination, packs = loadPacks()) {
  mkdirSync(dirname(destination), { recursive: true });
  const staged = mkdtempSync(resolve(dirname(destination), ".packs-export-"));
  try {
    for (const file of packs.files) copyPack(file, resolve(staged, file.name));
    writeFileSync(resolve(staged, "manifest.json"), packs.manifestBytes);
    loadPacks(staged);
    replaceDirectory(staged, destination);
  } finally {
    rmSync(staged, { recursive: true, force: true });
  }
}
