#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { randomUUID } from "node:crypto";
import { parseArgs } from "node:util";
import { browserAssets } from "../integrations/assets.mjs";

const { values, positionals } = parseArgs({ allowPositionals: true, options: { base: { type: "string", default: "/blasphem/" } } });
const target = positionals[0];
if (!target) {
  console.error("usage: blasphem-assets <directory> [--base /application/blasphem/]");
  process.exit(2);
}

const { entries } = browserAssets(process.cwd(), values.base);
const destination = resolve(process.cwd(), target);
mkdirSync(destination, { recursive: true });
const marker = resolve(destination, ".blasphem-files.json");
const previous = existsSync(marker) ? JSON.parse(readFileSync(marker, "utf8")) : [];
const names = entries.map(([name]) => name);
const generatedName = (name) => typeof name === "string" && basename(name) === name &&
  (["NOTICE", "manifest.json", "bundle.json", "config.js", "blasphem_bg.wasm"].includes(name) || /^[a-z]{2,3}\.(pack|detect)$/.test(name));
if (!Array.isArray(previous) || previous.some((name) => !generatedName(name))) {
  throw new Error("Invalid generated asset ownership file");
}
for (const [name, bytes] of entries) {
  const temporary = resolve(destination, `.${name}.${randomUUID()}.tmp`);
  writeFileSync(temporary, bytes);
  renameSync(temporary, resolve(destination, name));
}
for (const name of previous.filter((name) => !names.includes(name))) {
  if (existsSync(resolve(destination, name))) unlinkSync(resolve(destination, name));
}
writeFileSync(marker, `${JSON.stringify(names)}\n`);
const bytes = entries.reduce((total, entry) => total + entry[1].length, 0);
console.log(`status=copied files=${names.length} bytes=${bytes} to=${target}`);
