# Blasphem Web Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pnpm + Turborepo JavaScript workspace with the private `blasphem` browser package and a single-page Astro landing site whose playground loads the WASM detector only after the first user action.

**Architecture:** The Cargo workspace stays the Rust task runner. Turborepo runs two JavaScript packages: `packages/blasphem` wraps `cargo build` + `wasm-bindgen` into `dist/`, and `apps/web` is a static Astro 7 site. An Astro integration copies the package `dist` into the site output under a content-hashed path; the playground imports it at runtime with a `@vite-ignore` dynamic import, so the initial page never references WASM. All benchmark figures are read from `reports/*.json` at build time.

**Tech Stack:** pnpm 11.13.0, Node 24.18.0, Turborepo 2.10.12, Astro 7.2.10, TypeScript 5.9.3, `@astrojs/sitemap` 3.7.4, `@resvg/resvg-js` 2.6.2, `sharp` 0.35.4, `@fontsource/*` 5.3.0, wasm-bindgen 0.2.127, Rust 1.97.0.

**Spec:** `docs/superpowers/specs/2026-09-02-blasphem-public-package-and-corpus-design.md` (sections "Npm package", "Web workspace", "Tests", "Acceptance criteria").

## Global Constraints

- Work only inside the worktree `/Users/sospedra/labs/blasphem-wt/development` on branch `development`. Never run `git -C` against another checkout. Never use bare `git stash`.
- Commit subjects only. No bodies. No trailers.
- The branch merged the renamed `development` (`09d1bca`) as `71f2ccb`. `cargo metadata` resolves `blasphem`, `blasphem-language`, `blasphem-wasm`, `blasphem-bench`, `blasphem-train`. Do not edit any Cargo manifest or Rust file; the other agent (`blasphem-98`) owns them. Its Task 3 already exports `BlasphemDetector` and `BlasphemResult`.
- Ownership rulings from the user on 2026-09-03: pnpm + Turborepo replace npm for both agents; this branch owns `packages/blasphem` (the other plan's Task 14 and Task 11 step 4); the other agent adds the Playwright browser smoke to the package after the user merges `development` into `development`, which happens before its Task 12.
- Public names: package `blasphem`, classes `BlasphemDetector` and `BlasphemResult`, files `dist/blasphem.js` and `dist/blasphem_bg.wasm`, types at `packages/blasphem/index.d.ts`.
- Retired names must not appear in any `.rs .toml .md .json .mjs .js .ts .sh .yml .html` file under `apps/` or `packages/`, except `packages/blasphem/NOTICE`: `toxcheck`, `toxtrain`, `toxbench`, `toxcheck-wasm`, `toxcheck_wasm`, `eldc`, `ELDC`, `import-eldc`. `tests/rename_contract.rs` enforces this on disk, so never write them into source, never hardcode `reports/eldc-auto-validation.json`, and never write the crate name into a file.
- Pins: `packageManager` `pnpm@11.13.0`, Node `24.18.0`, `turbo` `2.10.12`, `astro` `7.2.10`, `wasm-bindgen` CLI `0.2.127` read from `crates/blasphem-wasm/Cargo.toml`.
- Canonical site URL `https://blasphem.sospedra.me`; `SITE_URL` overrides it at build.
- The site has one page `/` with anchored chapters `#detector`, `#rite`, `#vows`, `#reckoning`, `#colophon`. No React. No server runtime. No third-party request (fonts self-hosted). No automated website test files. No npm publish. No deploy.
- The playground supports `AUTO` and the 15 codes `EN ZH ES AR MS PT FR HI RU JA DE TR VI KO IT`, accepts `ID` as the Malay alias, shows `ok`, `score`, `shouldNudge`, `resolvedLanguage`, imports the package only after the first action, never preloads WASM, shows loading and failure states, frees every result and detector, and keeps text in the browser.
- Benchmark values come from `reports/*.json` at build time. No number from a report is typed into a component. The page states that scores are ordinal and not probabilities, states the experimental limitations, and labels each figure with its `evidence_status`.
- Code style (project rules): guard clauses first, no `else` after `return`, no `forEach`, combinators over loops, lookup tables over `else if` chains, discriminated unions for phases, three parameters max, `const` by default, no what-comments.
- Astro 7 uses the Rust compiler: every tag must close. `compressHTML: true` keeps classic whitespace handling.
- Turbo caches only `blasphem#build` and `web#build`. `test`, `dev`, `check`, `//#reproduce`, `//#regenerate` set `cache: false`.
- Design skills: invoke `frontend-design:frontend-design` and `emil-design-eng` before writing CSS in Task 6, and `find-animation-opportunities` at the start of Task 12.

---

## File Structure

| Path | Responsibility |
| --- | --- |
| `package.json` | Workspace root: `packageManager`, root scripts, `turbo` dev dependency. |
| `pnpm-workspace.yaml` | Workspace members and allowed build scripts. |
| `turbo.json` | Task graph, inputs, outputs, cache policy. |
| `.gitignore` | Adds JavaScript build output and caches. |
| `crates/blasphem-wasm/src/lib.rs` | Two `js_name` and two `js_class` attributes (Task 2 only). |
| `packages/blasphem/package.json` | Private package manifest, exports, files, scripts. |
| `packages/blasphem/index.d.ts` | Public TypeScript declarations. |
| `packages/blasphem/scripts/crate.mjs` | Reads crate name and `wasm-bindgen` pin from the crate manifest. |
| `packages/blasphem/scripts/build.mjs` | `cargo build` + `wasm-bindgen` into `dist/`, asserts class names. |
| `packages/blasphem/scripts/pack-check.mjs` | `pnpm pack --dry-run --json` archive assertions. |
| `packages/blasphem/README.md`, `NOTICE`, `LICENSE`, `TOOLCHAIN.md` | Package documents shipped in the archive. |
| `apps/web/package.json`, `astro.config.ts`, `tsconfig.json`, `vercel.json` | Site manifest and configuration. |
| `apps/web/integrations/blasphem-assets.ts` | Hashes and copies the package `dist` into the site output; serves it in dev; defines `__BLASPHEM_BASE__`. |
| `apps/web/src/env.d.ts` | Declares the two build-time constants. |
| `apps/web/src/lib/reports.ts` | Loads `reports/*.json` and selects each report by content signature. |
| `apps/web/src/lib/metrics.ts` | Pure aggregates over reports (medians, totals, transfer). |
| `apps/web/src/lib/languages.ts` | The 15 languages, `AUTO`, alias normalization. Shared by server and client. |
| `apps/web/src/lib/format.ts` | Number, percent, bytes, milliseconds formatting. |
| `apps/web/src/lib/samples.ts` | Sample messages from the CLI smoke report. |
| `apps/web/src/scripts/playground-state.ts` | Pure phase machine and verdict mapping. |
| `apps/web/src/scripts/playground.ts` | DOM wiring, lazy module load, detector lifecycle. |
| `apps/web/src/styles/tokens.css`, `base.css`, `codex.css` | Palette, base document, shared codex layout classes. |
| `apps/web/src/layouts/Codex.astro` | Document shell, fonts, frame, rails, skip link. |
| `apps/web/src/components/*.astro` | `Head`, `Glyphs`, `Frame`, `RailRight`, `Spread`, `Numeral`, `Frontispiece`, `Seal`, `Advisory`, `Badges`, `Detector`, `Rite`, `Vows`, `Reckoning`, `Ledger`, `Colophon`. |
| `apps/web/src/pages/index.astro` | The one page, composed from chapters. |
| `apps/web/src/pages/robots.txt.ts`, `og.png.ts` | Generated robots file and Open Graph image. |
| `apps/web/src/assets/bust.jpg`, `fonts/PirataOne-Regular.ttf`, `fonts/OFL.txt` | Committed CC0 image and OFL font for the OG render. |
| `apps/web/public/favicon.svg` | Favicon. |

---

## Phases

1. Tasks 1 to 3: workspace root, crate attributes, package.
2. Tasks 4 to 11: site scaffold, data, shell, chapters.
3. Task 12: motion and polish.
4. Task 13: merge `development`, full verification, report.

---

### Task 1: Workspace root and Turborepo

**Files:**
- Create: `package.json`
- Create: `pnpm-workspace.yaml`
- Create: `turbo.json`
- Modify: `.gitignore`

**Interfaces:**
- Produces: root scripts `build`, `dev`, `test`, `check`, `reproduce`, `regenerate`; Turbo tasks `build`, `test`, `check`, `dev`, `//#reproduce`, `//#regenerate`. Tasks 3 and 4 add `blasphem#build`, `blasphem#test`, `web#build`, `web#check`.

- [ ] **Step 1: Write the root manifest**

Create `package.json`:

```json
{
  "name": "blasphem-workspace",
  "private": true,
  "packageManager": "pnpm@11.13.0",
  "engines": {
    "node": "24.18.0"
  },
  "scripts": {
    "build": "turbo run build",
    "dev": "turbo run dev",
    "test": "turbo run test",
    "check": "turbo run check",
    "reproduce": "cargo run --release --locked -p blasphem-train -- reproduce",
    "regenerate": "cargo run --release --locked -p blasphem-train -- regenerate"
  },
  "devDependencies": {
    "turbo": "2.10.12"
  }
}
```

- [ ] **Step 2: Declare the workspace**

Create `pnpm-workspace.yaml`:

```yaml
packages:
  - apps/*
  - packages/*
onlyBuiltDependencies:
  - esbuild
  - sharp
```

- [ ] **Step 3: Write the task graph**

Create `turbo.json`:

```json
{
  "$schema": "https://turborepo.dev/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**"]
    },
    "test": {
      "dependsOn": ["build"],
      "cache": false
    },
    "check": {
      "cache": false
    },
    "dev": {
      "cache": false,
      "persistent": true
    },
    "//#reproduce": {
      "cache": false
    },
    "//#regenerate": {
      "cache": false
    }
  }
}
```

- [ ] **Step 4: Ignore JavaScript output**

Append to `.gitignore`:

```text
node_modules/
.turbo/
/apps/web/dist/
/apps/web/.astro/
```

- [ ] **Step 5: Install and verify**

Run: `pnpm install`
Expected: the output ends with `Done in` and `pnpm-lock.yaml` exists.

Run: `pnpm turbo --version`
Expected: `2.10.12`

Run: `pnpm turbo run build --dry-run`
Expected: exit code 0. The `Tasks to Run` section is empty because no workspace package exists yet.

- [ ] **Step 6: Commit**

```bash
git add package.json pnpm-workspace.yaml pnpm-lock.yaml turbo.json .gitignore
git commit -m "Add the pnpm workspace root and Turborepo"
```

---

### Task 2: Confirm the merged Rust baseline (done)

**Files:** none. Merge commit `71f2ccb` already exists on `development`.

**Interfaces:**
- Consumes: `development` at `09d1bca` (crate rename, `BlasphemDetector`/`BlasphemResult` exports, renamed reports).
- Produces: a workspace where `cargo build -p blasphem-wasm --target wasm32-unknown-unknown` can run.

- [x] **Step 1: Verify the baseline**

Run: `cargo metadata --no-deps --format-version 1 | python3 -c "import json,sys; print(sorted(p['name'] for p in json.load(sys.stdin)['packages']))"`
Expected: `['blasphem', 'blasphem-bench', 'blasphem-language', 'blasphem-train', 'blasphem-wasm']`

Run: `grep -n "js_name = Blasphem\|js_class = \"Blasphem" crates/blasphem-wasm/src/lib.rs`
Expected: four lines (`BlasphemDetector` struct and impl, `BlasphemResult` struct and impl).

Run: `ls reports`
Expected: seven files, none containing a retired name. `language-auto-validation.json` replaced the old routing report; `reports.ts` selects it by content, not by name.

- [x] **Step 2: Nothing to commit**

The merge commit is the deliverable.

---

### Task 3: The private blasphem package

**Files:**
- Create: `packages/blasphem/package.json`
- Create: `packages/blasphem/index.d.ts`
- Create: `packages/blasphem/scripts/crate.mjs`
- Create: `packages/blasphem/scripts/build.mjs`
- Create: `packages/blasphem/scripts/pack-check.mjs`
- Create: `packages/blasphem/README.md`
- Create: `packages/blasphem/NOTICE`
- Create: `packages/blasphem/LICENSE`
- Create: `packages/blasphem/TOOLCHAIN.md`
- Modify: `turbo.json`

**Interfaces:**
- Consumes: `crates/blasphem-wasm/Cargo.toml` lines `name = "..."` and `wasm-bindgen = "=0.2.127"`.
- Produces: `dist/blasphem.js` (ESM glue, default export `init`, classes `BlasphemDetector`, `BlasphemResult`), `dist/blasphem_bg.wasm`, scripts `pnpm --filter blasphem run build`, `pnpm --filter blasphem run pack:check`, `pnpm --filter blasphem run test`.
- Produces for `apps/web`: `import type { BlasphemDetector, BlasphemResult } from "blasphem"` and `typeof import("blasphem")` through `index.d.ts`.

- [ ] **Step 1: Write the manifest**

Create `packages/blasphem/package.json`:

```json
{
  "name": "blasphem",
  "version": "0.1.0",
  "private": true,
  "description": "Experimental multilingual pre-send toxicity nudge for browsers",
  "license": "Apache-2.0",
  "type": "module",
  "sideEffects": false,
  "engines": {
    "node": "24.18.0",
    "pnpm": "11.13.0"
  },
  "exports": {
    ".": {
      "types": "./index.d.ts",
      "browser": "./dist/blasphem.js",
      "default": "./dist/blasphem.js"
    },
    "./blasphem_bg.wasm": "./dist/blasphem_bg.wasm"
  },
  "types": "./index.d.ts",
  "files": [
    "dist/blasphem.js",
    "dist/blasphem_bg.wasm",
    "index.d.ts",
    "README.md",
    "NOTICE",
    "LICENSE"
  ],
  "scripts": {
    "build": "node scripts/build.mjs",
    "pack:check": "node scripts/pack-check.mjs",
    "test": "node scripts/pack-check.mjs"
  }
}
```

- [ ] **Step 2: Write the declarations**

Create `packages/blasphem/index.d.ts`:

```ts
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
}

/** Loads and instantiates the WebAssembly module. Call once before creating a detector. */
export default function init(
  input?: InitInput | Promise<InitInput> | { module_or_path: InitInput | Promise<InitInput> },
): Promise<InitOutput>;

/** Instantiates synchronously from bytes or a compiled module. */
export function initSync(input: BufferSource | WebAssembly.Module | { module: BufferSource | WebAssembly.Module }): InitOutput;

/** The result of one check. Read the fields, then call free(). */
export class BlasphemResult {
  private constructor();
  /** True when no nudge is due. Also true for an unknown automatic route. */
  readonly ok: boolean;
  /** Ordinal integer from 0 through 100. Not a probability. */
  readonly score: number;
  /** The nudge boundary the score is compared against. */
  readonly threshold: number;
  /** True when the pre-send nudge should show. */
  readonly shouldNudge: boolean;
  /** False when automatic routing found no reliable supported language. */
  readonly evaluated: boolean;
  /** One of the 15 language codes, or "unknown". */
  readonly resolvedLanguage: string;
  readonly languageReliable: boolean;
  /** Present only on automatic routes. */
  readonly languageScore: number | undefined;
  free(): void;
}

/** One detector for an explicit language code or "AUTO". */
export class BlasphemDetector {
  /**
   * @param language "EN", "ZH", "ES", "AR", "MS", "PT", "FR", "HI", "RU", "JA", "DE", "TR", "VI", "KO", "IT", or "AUTO".
   * "ID" is accepted as an alias for "MS". Any other value throws.
   */
  constructor(language: string);
  /** The selection code this detector was built with. "ID" reports as "MS". */
  readonly language: string;
  check(text: string): BlasphemResult;
  free(): void;
}
```

- [ ] **Step 3: Write the crate manifest reader**

Create `packages/blasphem/scripts/crate.mjs`:

```js
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const projectRoot = resolve(packageRoot, "../..");
export const crateManifest = resolve(projectRoot, "crates/blasphem-wasm/Cargo.toml");

function manifestValue(manifest, key) {
  const prefix = `${key} = `;
  const line = manifest.split("\n").find((candidate) => candidate.startsWith(prefix));
  if (!line) throw new Error(`crates/blasphem-wasm/Cargo.toml has no "${key}" entry`);
  return JSON.parse(line.slice(prefix.length).trim());
}

export function readCrate() {
  const manifest = readFileSync(crateManifest, "utf8");
  const name = manifestValue(manifest, "name");
  const requirement = manifestValue(manifest, "wasm-bindgen");
  if (!requirement.startsWith("=")) {
    throw new Error(`wasm-bindgen must pin an exact version, found "${requirement}"`);
  }
  return { name, libName: name.replaceAll("-", "_"), wasmBindgenVersion: requirement.slice(1) };
}
```

- [ ] **Step 4: Write the build script**

Create `packages/blasphem/scripts/build.mjs`:

```js
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { packageRoot, projectRoot, readCrate } from "./crate.mjs";

const distribution = resolve(packageRoot, "dist");
const targetDir = resolve(projectRoot, "target/npm-wasm");
const REQUIRED_CLASSES = ["class BlasphemDetector", "class BlasphemResult"];

function capture(command, args) {
  return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

function stream(command, args, env = process.env) {
  execFileSync(command, args, { stdio: "inherit", env });
}

function assertWasmBindgen(expected) {
  const found = capture("wasm-bindgen", ["--version"]).trim();
  if (found === `wasm-bindgen ${expected}`) return;
  throw new Error(
    `wasm-bindgen-cli must be ${expected}, found "${found}". Run: cargo install wasm-bindgen-cli --version ${expected} --locked`,
  );
}

function buildCrate(crate) {
  stream(
    "cargo",
    ["build", "--release", "--locked", "--target", "wasm32-unknown-unknown", "-p", crate.name, "--manifest-path", resolve(projectRoot, "Cargo.toml")],
    { ...process.env, CARGO_TARGET_DIR: targetDir },
  );
  return resolve(targetDir, "wasm32-unknown-unknown/release", `${crate.libName}.wasm`);
}

function generateGlue(wasmPath) {
  rmSync(distribution, { recursive: true, force: true });
  mkdirSync(distribution, { recursive: true });
  stream("wasm-bindgen", [wasmPath, "--target", "web", "--out-dir", distribution, "--out-name", "blasphem"]);
}

function assertClasses() {
  const glue = readFileSync(resolve(distribution, "blasphem.js"), "utf8");
  const missing = REQUIRED_CLASSES.filter((marker) => !glue.includes(marker));
  if (missing.length === 0) return;
  throw new Error(`dist/blasphem.js lacks ${missing.join(", ")}. The crate needs js_name and js_class attributes.`);
}

const crate = readCrate();
assertWasmBindgen(crate.wasmBindgenVersion);
generateGlue(buildCrate(crate));
assertClasses();
const wasmBytes = statSync(resolve(distribution, "blasphem_bg.wasm")).size;
const glueBytes = statSync(resolve(distribution, "blasphem.js")).size;
console.log(`status=built wasm_bytes=${wasmBytes} glue_bytes=${glueBytes}`);
```

- [ ] **Step 5: Write the pack check**

Create `packages/blasphem/scripts/pack-check.mjs`:

```js
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { packageRoot } from "./crate.mjs";

const REQUIRED_FILES = [
  "LICENSE",
  "NOTICE",
  "README.md",
  "dist/blasphem.js",
  "dist/blasphem_bg.wasm",
  "index.d.ts",
  "package.json",
];
const FORBIDDEN_PREFIXES = ["crates/", "data/", "reports/", "resources/", "src/", "target/"];

function readManifest() {
  return JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));
}

function assertManifest(manifest) {
  if (manifest.name !== "blasphem") throw new Error(`the package name must be "blasphem", found "${manifest.name}"`);
  if (manifest.private !== true) throw new Error("the package must stay private");
  const entry = manifest.exports?.["."];
  if (entry?.types !== "./index.d.ts") throw new Error('exports["."].types must be ./index.d.ts');
  if (entry?.default !== "./dist/blasphem.js") throw new Error('exports["."].default must be ./dist/blasphem.js');
}

function assertDistribution() {
  const missing = ["dist/blasphem.js", "dist/blasphem_bg.wasm"].filter((file) => !existsSync(resolve(packageRoot, file)));
  if (missing.length === 0) return;
  throw new Error(`missing ${missing.join(", ")}. Run: pnpm --filter blasphem run build`);
}

function packedPaths() {
  const output = execFileSync("pnpm", ["pack", "--dry-run", "--json"], { cwd: packageRoot, encoding: "utf8" });
  return JSON.parse(output).files.map((file) => file.path).toSorted();
}

function assertPaths(paths) {
  const missing = REQUIRED_FILES.filter((file) => !paths.includes(file));
  if (missing.length > 0) throw new Error(`the archive is missing ${missing.join(", ")}`);
  const forbidden = paths.filter((path) => FORBIDDEN_PREFIXES.some((prefix) => path.startsWith(prefix)));
  if (forbidden.length > 0) throw new Error(`the archive must not carry ${forbidden.join(", ")}`);
  const unexpected = paths.filter((path) => !REQUIRED_FILES.includes(path));
  if (unexpected.length > 0) throw new Error(`the archive carries unexpected files: ${unexpected.join(", ")}`);
}

assertManifest(readManifest());
assertDistribution();
const paths = packedPaths();
assertPaths(paths);
console.log(`status=packed files=${paths.length}`);
```

- [ ] **Step 6: Write the package documents**

Create `packages/blasphem/README.md`:

```markdown
# blasphem

Experimental multilingual pre-send toxicity nudge for browsers. Deterministic rules, HurtLex lexica, and one sparse integer table per language, compiled to WebAssembly. No AI runtime, no network request after the module loads.

This package is private and unpublished. Build it from the repository:

```bash
pnpm install --frozen-lockfile
pnpm --filter blasphem run build
```

The build writes `dist/blasphem.js` and `dist/blasphem_bg.wasm`.

## Use

```js
import init, { BlasphemDetector } from "blasphem";

await init();
const detector = new BlasphemDetector("AUTO");
const result = detector.check("message text");

console.log(result.ok, result.score, result.shouldNudge, result.resolvedLanguage);

result.free();
detector.free();
```

The constructor accepts `EN`, `ZH`, `ES`, `AR`, `MS`, `PT`, `FR`, `HI`, `RU`, `JA`, `DE`, `TR`, `VI`, `KO`, `IT`, or `AUTO`. `ID` is an alias for `MS`.

`score` is an ordinal integer from 0 through 100. It is not a probability.

`AUTO` returns `resolvedLanguage = "unknown"`, `evaluated = false`, and `ok = true` for text it cannot route reliably. The nudge fails open.

Call `free()` on every result and detector.

## Serving the module

When `blasphem_bg.wasm` is not next to `blasphem.js`, pass its URL:

```js
await init({ module_or_path: new URL("/assets/blasphem_bg.wasm", location.origin) });
```

Serve the file as `application/wasm`.

## Pinned tools

See `TOOLCHAIN.md`. The build stops when `wasm-bindgen --version` differs from the crate pin.
```

Create `packages/blasphem/NOTICE`:

```text
blasphem browser package
Copyright 2026 Rubén Sospedra

Licensed under the Apache License, Version 2.0. See LICENSE.

This package embeds third-party data.

HurtLex 1.2 lexica for AR, DE, EN, ES, FR, HI, ID, IT, JA, KO, PT, RU, TR, VI, ZH
  Source: https://github.com/valeriobasile/hurtlex
  Revision: d4d5cf1199c09868486f978fcea58af0e8936a1e
  License: CC BY-SA 4.0, https://creativecommons.org/licenses/by-sa/4.0/
  Citation: Bassignana et al., 2018. HurtLex: A Multilingual Lexicon of Words to Hurt.
  Share-alike: the embedded lexica are redistributed under CC BY-SA 4.0. Any
  redistribution of this data, modified or not, must carry the same license
  and this attribution.

Language detection tables ported from ELDC
  Source: https://github.com/nitotm/eldc
  Commit: a0301db809ff2e48a418018aa5359fb0c4354eb8
  Author: Nito
  License: Apache License 2.0

The sparse toxicity tables were trained on these corpora. The tables contain
hashed integer weights and no corpus text.

  TextDetox multilingual toxicity dataset (AR, DE, EN, FR, HI, IT, JA, RU, ZH)
    https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset
    CC BY 4.0
  Ibrohim and Budi, 2019 (ID)
    https://github.com/okkyibrohim/id-multi-label-hate-speech-and-abusive-language-detection
    CC BY 4.0
  ToLD-Br, Leite et al., 2020 (PT)
    https://github.com/joaoaleite/ToLD-Br
    CC BY 4.0
  OffensEval-TR, Çöltekin, 2020 (TR)
    https://coltekin.github.io/offensive-turkish/
    CC BY 4.0
  ViHOS (VI)
    https://github.com/phusroyal/ViHOS
    CC BY 4.0
  K-MHaS, Lee et al., 2022 (KO)
    https://github.com/adlnlp/K-MHaS
    CC BY-NC 4.0
  GermEval 2018, Wiegand, Siegel, and Ruppenhofer, 2018 (DE)
    https://github.com/uds-lsv/GermEval-2018-Data
    License not asserted by the source
```

Download the license text:

```bash
curl -sSL https://www.apache.org/licenses/LICENSE-2.0.txt -o packages/blasphem/LICENSE
head -3 packages/blasphem/LICENSE
wc -l packages/blasphem/LICENSE
```

Expected: the first non-empty line is `Apache License` and the file has 202 lines.

Create `packages/blasphem/TOOLCHAIN.md`:

```markdown
# Pinned tools

The browser package builds only with these versions.

| Tool | Version | Source |
| --- | --- | --- |
| Rust | 1.97.0 | `rust-toolchain.toml` at the repository root |
| Node | 24.18.0 | root `package.json` `engines` |
| pnpm | 11.13.0 | root `package.json` `packageManager` |
| `wasm-bindgen-cli` | 0.2.127 | `crates/blasphem-wasm/Cargo.toml`; install with `cargo install wasm-bindgen-cli --version 0.2.127 --locked` |
| `wasm32-unknown-unknown` target | matches Rust | `rustup target add wasm32-unknown-unknown` |

`scripts/build.mjs` reads the crate name and the `wasm-bindgen` pin from the crate manifest and stops on a CLI mismatch.
```

- [ ] **Step 7: Register the package task in Turbo**

In `turbo.json`, add two entries inside `"tasks"` after `"build"`:

```json
    "blasphem#build": {
      "inputs": [
        "$TURBO_DEFAULT$",
        "$TURBO_ROOT$/Cargo.toml",
        "$TURBO_ROOT$/Cargo.lock",
        "$TURBO_ROOT$/rust-toolchain.toml",
        "$TURBO_ROOT$/.cargo/**",
        "$TURBO_ROOT$/src/**",
        "$TURBO_ROOT$/crates/**",
        "$TURBO_ROOT$/data/raw-v1/hurtlex/**",
        "$TURBO_ROOT$/resources/models/**"
      ],
      "outputs": ["dist/**"]
    },
    "blasphem#test": {
      "dependsOn": ["blasphem#build"],
      "cache": false
    },
```

- [ ] **Step 8: Build, pack-check, and verify**

Run: `pnpm install`
Expected: `Done in`.

Run: `node -e "import('./packages/blasphem/scripts/crate.mjs').then((m) => console.log(JSON.stringify(m.readCrate())))"`
Expected: `{"name":"blasphem-wasm","libName":"blasphem_wasm","wasmBindgenVersion":"0.2.127"}`

Run: `pnpm --filter blasphem run pack:check; echo "exit=$?"`
Expected before the build: `missing dist/blasphem.js, dist/blasphem_bg.wasm. Run: pnpm --filter blasphem run build` and `exit=1`.

Run: `pnpm --filter blasphem run build 2>&1 | tail -5`
Expected: cargo finishes the `wasm32-unknown-unknown` release build, then `status=built wasm_bytes=<about 24000000> glue_bytes=<about 12000>`.

Run: `grep -c "class BlasphemDetector" packages/blasphem/dist/blasphem.js; grep -c "class BlasphemResult" packages/blasphem/dist/blasphem.js`
Expected: `1` and `1`.

Run: `pnpm --filter blasphem run pack:check`
Expected: `status=packed files=7`

Run: `pnpm turbo run build --filter=blasphem --dry-run | grep -E "blasphem#build|Inputs Files"`
Expected: `blasphem#build` listed with an `Inputs Files` count above 200 (the Rust and model inputs are hashed).

Run: `pnpm turbo run build --filter=blasphem && pnpm turbo run build --filter=blasphem | grep -c "cache hit"`
Expected: the second run prints `1` (the package build is cached and replayed).

Run: `grep -rnE "toxcheck|eldc|ELDC" packages/blasphem --include='*.mjs' --include='*.json' --include='*.md' --include='*.ts' | grep -v "^packages/blasphem/NOTICE" ; echo "retired-scan exit=$?"`
Expected: no matches and `retired-scan exit=1`.

- [ ] **Step 9: Commit**

```bash
git add packages/blasphem turbo.json pnpm-lock.yaml
git commit -m "Add the private blasphem browser package"
```

---

### Task 4: Astro site scaffold and the WASM asset integration

**Files:**
- Create: `apps/web/package.json`
- Create: `apps/web/astro.config.ts`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/vercel.json`
- Create: `apps/web/src/env.d.ts`
- Create: `apps/web/integrations/blasphem-assets.ts`
- Create: `apps/web/src/pages/index.astro` (temporary shell, replaced in Task 6)
- Create: `apps/web/src/pages/robots.txt.ts`
- Create: `apps/web/public/favicon.svg`
- Modify: `turbo.json`

**Interfaces:**
- Consumes: `packages/blasphem/dist/blasphem.js` and `dist/blasphem_bg.wasm` when present.
- Produces: build-time constants `__BLASPHEM_BASE__: string` (`"/blasphem/<16 hex>"` or `""`) and `__BLASPHEM_WASM_BYTES__: number`; output files `<outDir>/blasphem/<hash>/blasphem.js` and `blasphem_bg.wasm`; dev middleware for the same URLs; `pnpm --filter web build|dev|check`.

- [ ] **Step 1: Write the site manifest**

Create `apps/web/package.json`:

```json
{
  "name": "web",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "astro dev",
    "build": "astro build",
    "preview": "astro preview --host 127.0.0.1 --port 4321",
    "check": "astro check"
  },
  "dependencies": {
    "blasphem": "workspace:*"
  },
  "devDependencies": {
    "@astrojs/check": "0.9.10",
    "@astrojs/sitemap": "3.7.4",
    "@types/node": "24.13.3",
    "astro": "7.2.10",
    "typescript": "5.9.3"
  }
}
```

- [ ] **Step 2: Write the TypeScript configuration and constants**

Create `apps/web/tsconfig.json`:

```json
{
  "extends": "astro/tsconfigs/strict",
  "include": [".astro/types.d.ts", "**/*"],
  "exclude": ["dist"],
  "compilerOptions": {
    "types": ["node"],
    "verbatimModuleSyntax": true
  }
}
```

Create `apps/web/src/env.d.ts`:

```ts
declare const __BLASPHEM_BASE__: string;
declare const __BLASPHEM_WASM_BYTES__: number;
```

- [ ] **Step 3: Write the integration**

Create `apps/web/integrations/blasphem-assets.ts`:

```ts
import type { AstroIntegration } from "astro";
import { createHash } from "node:crypto";
import { copyFileSync, createReadStream, existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const GLUE = "blasphem.js";
const WASM = "blasphem_bg.wasm";
const CONTENT_TYPES: Record<string, string> = {
  [GLUE]: "text/javascript; charset=utf-8",
  [WASM]: "application/wasm",
};

export interface BlasphemAssetsOptions {
  distDir: string;
}

interface Located {
  base: string;
  wasmBytes: number;
}

function locate(distDir: string): Located | null {
  const glue = resolve(distDir, GLUE);
  const wasm = resolve(distDir, WASM);
  if (!existsSync(glue) || !existsSync(wasm)) return null;
  const digest = createHash("sha256").update(readFileSync(glue)).update(readFileSync(wasm)).digest("hex");
  return { base: `/blasphem/${digest.slice(0, 16)}`, wasmBytes: statSync(wasm).size };
}

function assetName(url: string | undefined, base: string): string | null {
  if (!url?.startsWith(`${base}/`)) return null;
  const name = url.slice(base.length + 1).split("?")[0];
  return name in CONTENT_TYPES ? name : null;
}

export default function blasphemAssets(options: BlasphemAssetsOptions): AstroIntegration {
  const located = locate(options.distDir);
  return {
    name: "blasphem-assets",
    hooks: {
      "astro:config:setup": ({ updateConfig, logger }) => {
        if (!located) logger.warn("packages/blasphem/dist is missing; the playground will report that the package is not built");
        updateConfig({
          vite: {
            define: {
              __BLASPHEM_BASE__: JSON.stringify(located?.base ?? ""),
              __BLASPHEM_WASM_BYTES__: JSON.stringify(located?.wasmBytes ?? 0),
            },
          },
        });
      },
      "astro:server:setup": ({ server }) => {
        if (!located) return;
        server.middlewares.use((request, response, next) => {
          const name = assetName(request.url, located.base);
          if (!name) return next();
          response.setHeader("Content-Type", CONTENT_TYPES[name]);
          response.setHeader("Cache-Control", "no-store");
          createReadStream(resolve(options.distDir, name)).pipe(response);
        });
      },
      "astro:build:done": ({ dir, logger }) => {
        if (!located) return;
        const target = resolve(fileURLToPath(dir), located.base.slice(1));
        mkdirSync(target, { recursive: true });
        copyFileSync(resolve(options.distDir, GLUE), resolve(target, GLUE));
        copyFileSync(resolve(options.distDir, WASM), resolve(target, WASM));
        logger.info(`copied ${GLUE} and ${WASM} to ${located.base}/`);
      },
    },
  };
}
```

- [ ] **Step 4: Write the Astro configuration**

Create `apps/web/astro.config.ts`:

```ts
import sitemap from "@astrojs/sitemap";
import { defineConfig } from "astro/config";
import { fileURLToPath } from "node:url";
import blasphemAssets from "./integrations/blasphem-assets";

const site = process.env.SITE_URL ?? "https://blasphem.sospedra.me";
const packageDist = fileURLToPath(new URL("../../packages/blasphem/dist/", import.meta.url));

export default defineConfig({
  site,
  output: "static",
  compressHTML: true,
  integrations: [sitemap(), blasphemAssets({ distDir: packageDist })],
});
```

- [ ] **Step 5: Write the temporary page, robots, favicon, Vercel headers**

Create `apps/web/src/pages/index.astro`:

```astro
---
const title = "blasphem";
---
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
  </head>
  <body>
    <main id="main">
      <h1>{title}</h1>
      <p>Scaffold. Replaced in Task 6.</p>
    </main>
  </body>
</html>
```

Create `apps/web/src/pages/robots.txt.ts`:

```ts
import type { APIRoute } from "astro";

export const GET: APIRoute = ({ site }) => {
  const sitemap = new URL("sitemap-index.xml", site);
  const body = `User-agent: *\nAllow: /\n\nSitemap: ${sitemap.href}\n`;
  return new Response(body, { headers: { "Content-Type": "text/plain; charset=utf-8" } });
};
```

Create `apps/web/public/favicon.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <rect width="32" height="32" fill="#0b0a0a"/>
  <path d="M14.5 3h3v8h6v3h-6v15h-3V14h-6v-3h6z" fill="#e23127"/>
  <path d="M25 20c.4 2.6 1 3.2 3.6 3.6C26 24 25.4 24.6 25 27.2c-.4-2.6-1-3.2-3.6-3.6 2.6-.4 3.2-1 3.6-3.6z" fill="#ece7de"/>
</svg>
```

Create `apps/web/vercel.json`:

```json
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "headers": [
    {
      "source": "/blasphem/(.*)",
      "headers": [{ "key": "Cache-Control", "value": "public, max-age=31536000, immutable" }]
    }
  ]
}
```

- [ ] **Step 6: Register the site tasks in Turbo**

In `turbo.json`, add inside `"tasks"` after `"blasphem#test"`:

```json
    "web#build": {
      "dependsOn": ["blasphem#build"],
      "inputs": ["$TURBO_DEFAULT$", "$TURBO_ROOT$/reports/**"],
      "outputs": ["dist/**"],
      "env": ["SITE_URL"]
    },
```

- [ ] **Step 7: Install and build without the package dist**

Run: `pnpm install`
Expected: `Done in`, `apps/web/node_modules/blasphem` is a symlink to `../../../packages/blasphem`.

Run: `pnpm --filter web build 2>&1 | tail -15`
Expected: a line containing `[blasphem-assets] packages/blasphem/dist is missing`, then `1 page(s) built`, then `Complete!`.

Run: `ls apps/web/dist`
Expected: `favicon.svg  index.html  robots.txt  sitemap-0.xml  sitemap-index.xml`

Run: `cat apps/web/dist/robots.txt`
Expected: ends with `Sitemap: https://blasphem.sospedra.me/sitemap-index.xml`.

Run: `SITE_URL=https://example.test pnpm --filter web build >/dev/null && grep -o "https://example.test[^<]*" apps/web/dist/sitemap-0.xml`
Expected: `https://example.test/`

Run: `pnpm --filter web check`
Expected: `0 errors`.

Run: `pnpm turbo run build --dry-run | grep -E "^(blasphem#build|web#build)|Dependencies" `
Expected: `blasphem#build`, then `web#build` whose `Dependencies` line lists `blasphem#build`.

- [ ] **Step 8: Commit**

```bash
git add apps/web turbo.json pnpm-lock.yaml
git commit -m "Scaffold the Astro site and the WASM asset integration"
```

---

### Task 5: Report loaders, languages, formatting, samples

**Files:**
- Create: `apps/web/src/lib/reports.ts`
- Create: `apps/web/src/lib/metrics.ts`
- Create: `apps/web/src/lib/languages.ts`
- Create: `apps/web/src/lib/format.ts`
- Create: `apps/web/src/lib/samples.ts`

**Interfaces:**
- Consumes: every `reports/*.json` file, selected by content, never by file name.
- Produces (server only, `reports.ts`, `metrics.ts`, `samples.ts`): `validation`, `performance`, `sizes`, `browser`, `routing`, `behavior`, `smoke`; `medianP95Ms(fixtures, suffix)`, `worstP95Ms(fixtures, suffix)`, `caseTotals(report)`, `routingTotals(routing)`, `SAMPLES`.
- Produces (server and client, `languages.ts`, `format.ts`): `LANGUAGES`, `LanguageCode`, `Selection`, `normalizeSelection(raw)`, `storageCode(code)`, `formatInt`, `formatPercent`, `formatBytes`, `formatMegabytes`, `formatMs`.

- [ ] **Step 1: Write the report loader**

Create `apps/web/src/lib/reports.ts`:

```ts
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

export type Ratio = { numerator: number; denominator: number; value: number };
export type Matrix = { false_negative: number; false_positive: number; true_negative: number; true_positive: number };

export type ValidationLanguage = {
  language: string;
  split: string;
  gates: { false_warning_passed: boolean; has_true_positive: boolean; precision_passed: boolean };
  matrix: Matrix;
  metrics: {
    f1: number;
    false_warning_rate: number;
    precision: number;
    recall: number;
    specificity: number;
    projected_precision_1_percent: number;
    projected_precision_5_percent: number;
  };
};
export type ValidationReport = {
  evidence_status: string;
  split: string;
  languages: Record<string, ValidationLanguage>;
  pooled_matrix: Matrix;
};

export type PerformanceFixture = {
  input_bytes: number;
  samples: number;
  p50_nanoseconds: number;
  p95_nanoseconds: number;
  p99_nanoseconds: number;
  maximum_nanoseconds: number;
  checks_per_second: number;
  bytes_per_second: number;
  peak_rss_bytes: number;
  latency_gate_passed: boolean;
};
export type PerformanceReport = {
  evidence_status: string;
  computer: string;
  target_triple: string;
  rust_version: string;
  all_latency_gates_passed: boolean;
  peak_rss_bytes: number;
  fixtures: Record<string, PerformanceFixture>;
};

export type SizedFile = { bytes: number; relative_path: string; sha256: string };
export type SizeReport = {
  evidence_status: string;
  all_gates_passed: boolean;
  target_triple: string;
  artifacts: Record<string, SizedFile>;
  binary: SizedFile;
  hurtlex: Record<string, SizedFile>;
};

export type CompressedFile = { raw_bytes: number; gzip_bytes: number; brotli_bytes: number; relative_path: string; sha256: string };
export type BrowserBuild = {
  wasm: CompressedFile;
  javascript_glue: CompressedFile;
  raw_total_bytes: number;
  gzip_total_bytes: number;
  brotli_total_bytes: number;
};
export type BrowserReport = {
  evidence_status: string;
  status: string;
  browser_engine: string;
  browser_version: string;
  wasm_bindgen_version: string;
  supplied_case_count: number;
  passed_case_count: number;
  auto_case_count: number;
  passed_auto_case_count: number;
  unknown_case_count: number;
  passed_unknown_case_count: number;
  runtime_network_requests: string[];
  browser_builds: { full: BrowserBuild; explicit_only: BrowserBuild };
};

export type RouteCounts = {
  rows: number;
  correct: number;
  unknown: number;
  misrouted: number;
  known_route_precision: Ratio;
  route_accuracy: Ratio;
  unknown_rate: Ratio;
  misroute_rate: Ratio;
};
export type TimingGroup = {
  samples: number;
  p50_nanoseconds: number;
  p95_nanoseconds: number;
  p99_nanoseconds: number;
  maximum_nanoseconds: number;
  checks_per_second: number;
};
export type RoutingReport = {
  evidence_status: string;
  computer: string;
  target_triple: string;
  cold_initialization_nanoseconds: number;
  corpus: { rows: number; supported_rows: number; unsupported_rows: number };
  supported: RouteCounts;
  unsupported: { rows: number; falsely_routed: number; rejected_as_unknown: number; unsupported_rejection_rate: Ratio };
  languages: Record<string, RouteCounts>;
  timing: { groups: Record<"unicode_scalars_280" | "utf8_bytes_4096", TimingGroup> };
  limitations: string[];
};

export type ContractCase = { case_id: string; text: string; expected_nudge: boolean; passed: boolean };
export type ContractReport = {
  evidence_status: string;
  languages: Record<string, { language: string; passed: boolean; cases: ContractCase[] }>;
};

export type SmokeCase = ContractCase & {
  language: string;
  suite: string;
  ok: boolean;
  score: number;
  should_nudge: boolean;
  threshold: number;
};
export type SmokeReport = {
  evidence_status: string;
  languages: Record<string, { language: string; passed: boolean; cases: SmokeCase[] }>;
};

type Loose = Record<string, unknown>;

const reportsDir = fileURLToPath(new URL("../../../../reports/", import.meta.url));

const reports: Loose[] = readdirSync(reportsDir)
  .filter((name) => name.endsWith(".json"))
  .map((name) => JSON.parse(readFileSync(join(reportsDir, name), "utf8")) as Loose);

function pick<T>(label: string, matches: (report: Loose) => boolean): T {
  const hits = reports.filter(matches);
  if (hits.length !== 1) throw new Error(`expected one ${label} report under reports/, found ${hits.length}`);
  return hits[0] as T;
}

export const validation = pick<ValidationReport>("validation", (report) => report.evidence_status === "calibration_evidence");
export const performance = pick<PerformanceReport>("performance", (report) => "all_latency_gates_passed" in report);
export const sizes = pick<SizeReport>("size", (report) => "artifacts" in report && "all_gates_passed" in report);
export const browser = pick<BrowserReport>("browser", (report) => report.execution_environment === "actual_browser");
export const routing = pick<RoutingReport>("routing", (report) => "c_parity" in report);
export const behavior = pick<ContractReport>("behavior", (report) => report.evidence_status === "behavior_contract_evidence");
export const smoke = pick<SmokeReport>("smoke", (report) => report.evidence_status === "native_cli_smoke_evidence");
```

- [ ] **Step 2: Write the pure aggregates**

Create `apps/web/src/lib/metrics.ts`:

```ts
import type { ContractReport, PerformanceFixture, RoutingReport } from "./reports";

const NANOSECONDS_PER_MILLISECOND = 1_000_000;

function median(values: readonly number[]): number {
  const sorted = values.toSorted((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[middle];
  return (sorted[middle - 1] + sorted[middle]) / 2;
}

function p95Values(fixtures: Record<string, PerformanceFixture>, suffix: string): number[] {
  return Object.entries(fixtures)
    .filter(([name]) => name.endsWith(suffix))
    .map(([, fixture]) => fixture.p95_nanoseconds);
}

export function medianP95Ms(fixtures: Record<string, PerformanceFixture>, suffix: string): number {
  return median(p95Values(fixtures, suffix)) / NANOSECONDS_PER_MILLISECOND;
}

export function worstP95Ms(fixtures: Record<string, PerformanceFixture>, suffix: string): number {
  return Math.max(...p95Values(fixtures, suffix)) / NANOSECONDS_PER_MILLISECOND;
}

export function fixtureCount(fixtures: Record<string, PerformanceFixture>, suffix: string): number {
  return p95Values(fixtures, suffix).length;
}

export function caseTotals(report: ContractReport): { total: number; passed: number } {
  const cases = Object.values(report.languages).flatMap((language) => language.cases);
  return { total: cases.length, passed: cases.filter((entry) => entry.passed).length };
}

export function routingTotals(report: RoutingReport): { knownPrecision: number; unknownRate: number; misrouteRate: number; rows: number } {
  return {
    knownPrecision: report.supported.known_route_precision.value,
    unknownRate: report.supported.unknown_rate.value,
    misrouteRate: report.supported.misroute_rate.value,
    rows: report.supported.rows,
  };
}

export function nanosecondsToMs(nanoseconds: number): number {
  return nanoseconds / NANOSECONDS_PER_MILLISECOND;
}
```

- [ ] **Step 3: Write the language table**

Create `apps/web/src/lib/languages.ts`:

```ts
export type LanguageCode = "EN" | "ZH" | "ES" | "AR" | "MS" | "PT" | "FR" | "HI" | "RU" | "JA" | "DE" | "TR" | "VI" | "KO" | "IT";
export type Selection = LanguageCode | "AUTO";

export type Language = {
  code: LanguageCode;
  name: string;
  tag: string;
  direction: "ltr" | "rtl";
};

export const LANGUAGES: readonly Language[] = [
  { code: "EN", name: "English", tag: "en", direction: "ltr" },
  { code: "ZH", name: "Chinese", tag: "zh", direction: "ltr" },
  { code: "ES", name: "Spanish", tag: "es", direction: "ltr" },
  { code: "AR", name: "Arabic", tag: "ar", direction: "rtl" },
  { code: "MS", name: "Malay", tag: "ms", direction: "ltr" },
  { code: "PT", name: "Portuguese", tag: "pt", direction: "ltr" },
  { code: "FR", name: "French", tag: "fr", direction: "ltr" },
  { code: "HI", name: "Hindi", tag: "hi", direction: "ltr" },
  { code: "RU", name: "Russian", tag: "ru", direction: "ltr" },
  { code: "JA", name: "Japanese", tag: "ja", direction: "ltr" },
  { code: "DE", name: "German", tag: "de", direction: "ltr" },
  { code: "TR", name: "Turkish", tag: "tr", direction: "ltr" },
  { code: "VI", name: "Vietnamese", tag: "vi", direction: "ltr" },
  { code: "KO", name: "Korean", tag: "ko", direction: "ltr" },
  { code: "IT", name: "Italian", tag: "it", direction: "ltr" },
];

const ALIASES: Record<string, LanguageCode> = { ID: "MS" };
const CODES = new Set<string>(LANGUAGES.map((language) => language.code));

export function normalizeSelection(raw: string): Selection | null {
  const upper = raw.trim().toUpperCase();
  if (upper === "AUTO") return "AUTO";
  const resolved = ALIASES[upper] ?? upper;
  return CODES.has(resolved) ? (resolved as LanguageCode) : null;
}

export function storageCode(code: LanguageCode): string {
  return code === "MS" ? "ID" : code;
}

export function languageByCode(code: LanguageCode): Language {
  const found = LANGUAGES.find((language) => language.code === code);
  if (!found) throw new Error(`unknown language ${code}`);
  return found;
}
```

- [ ] **Step 4: Write the formatters**

Create `apps/web/src/lib/format.ts`:

```ts
const integer = new Intl.NumberFormat("en-US", { maximumFractionDigits: 0 });

export function formatInt(value: number): string {
  return integer.format(value);
}

export function formatPercent(ratio: number, digits = 1): string {
  return `${(ratio * 100).toFixed(digits)}%`;
}

export function formatBytes(bytes: number): string {
  return `${formatInt(bytes)} B`;
}

export function formatMegabytes(bytes: number, digits = 2): string {
  return `${(bytes / 1_000_000).toFixed(digits)} MB`;
}

export function formatKibibytes(bytes: number): string {
  return `${Math.round(bytes / 1024)} KiB`;
}

export function formatMs(milliseconds: number, digits = 2): string {
  return `${milliseconds.toFixed(digits)} ms`;
}
```

- [ ] **Step 5: Write the sample list**

Create `apps/web/src/lib/samples.ts`:

```ts
import { LANGUAGES, type LanguageCode } from "./languages";
import { smoke } from "./reports";

export type Sample = {
  code: LanguageCode;
  tag: string;
  direction: "ltr" | "rtl";
  name: string;
  kind: "toxic" | "clean";
  text: string;
};

export const SAMPLES: readonly Sample[] = LANGUAGES.flatMap((language) =>
  smoke.languages[language.code].cases
    .filter((entry) => entry.suite === "supplied")
    .map((entry) => ({
      code: language.code,
      tag: language.tag,
      direction: language.direction,
      name: language.name,
      kind: entry.expected_nudge ? "toxic" : "clean",
      text: entry.text,
    })),
);
```

- [ ] **Step 6: Verify the types and the loader**

Run: `pnpm --filter web check`
Expected: `0 errors`.

Run:

```bash
cat > /tmp/blasphem-reports-probe.mjs <<'MJS'
import { readdirSync, readFileSync } from "node:fs";
const names = readdirSync("reports").filter((n) => n.endsWith(".json"));
const docs = names.map((n) => JSON.parse(readFileSync(`reports/${n}`, "utf8")));
const count = (test) => docs.filter(test).length;
console.log(
  count((r) => r.evidence_status === "calibration_evidence"),
  count((r) => "all_latency_gates_passed" in r),
  count((r) => "artifacts" in r && "all_gates_passed" in r),
  count((r) => r.execution_environment === "actual_browser"),
  count((r) => "c_parity" in r),
  count((r) => r.evidence_status === "behavior_contract_evidence"),
  count((r) => r.evidence_status === "native_cli_smoke_evidence"),
);
MJS
node /tmp/blasphem-reports-probe.mjs
```

Expected: `1 1 1 1 1 1 1` (each signature selects exactly one report).

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/lib
git commit -m "Load benchmark reports and language data for the site"
```

---

### Task 6: The codex shell: layout, tokens, fonts, head, frame, rails, Open Graph image

Invoke `frontend-design:frontend-design` and `emil-design-eng` before writing CSS in this task. Keep the reference palette and type voices; refine spacing, contrast, and states.

**Files:**
- Create: `apps/web/src/styles/tokens.css`
- Create: `apps/web/src/styles/base.css`
- Create: `apps/web/src/styles/codex.css`
- Create: `apps/web/src/components/Head.astro`
- Create: `apps/web/src/components/Glyphs.astro`
- Create: `apps/web/src/components/Frame.astro`
- Create: `apps/web/src/components/RailRight.astro`
- Create: `apps/web/src/components/Spread.astro`
- Create: `apps/web/src/components/Numeral.astro`
- Create: `apps/web/src/layouts/Codex.astro`
- Create: `apps/web/src/pages/og.png.ts`
- Create: `apps/web/src/assets/fonts/PirataOne-Regular.ttf`, `apps/web/src/assets/fonts/OFL.txt`
- Modify: `apps/web/src/pages/index.astro`
- Modify: `apps/web/package.json` (dependencies)

**Interfaces:**
- Produces: `<Codex title description>` layout with a `<slot />` inside `<main id="main" class="codex">`; `<Spread id label class>` section wrapper with a `<slot />`; `<Numeral value class>`; global classes `.page.l`, `.page.r`, `.rubric`, `.rubric.mute`, `.title`, `.copy`, `.acts`, `.act`, `.act.ghost`, `.ledger`; SVG symbols `#star`, `#cross`, `#archClip`, `#sealPath`, `#archFrame`, `#thornRing`.

- [ ] **Step 1: Add dependencies**

Add to `apps/web/package.json` `devDependencies` (keep the existing entries, sorted):

```json
    "@fontsource/archivo": "5.3.0",
    "@fontsource/cinzel": "5.3.0",
    "@fontsource/eb-garamond": "5.3.0",
    "@fontsource/ibm-plex-mono": "5.3.0",
    "@fontsource/pirata-one": "5.3.0",
    "@resvg/resvg-js": "2.6.2",
    "sharp": "0.35.4",
```

Run: `pnpm install`
Expected: `Done in`.

- [ ] **Step 2: Fetch the OFL font for the Open Graph render**

```bash
mkdir -p apps/web/src/assets/fonts
curl -sSL -o apps/web/src/assets/fonts/PirataOne-Regular.ttf https://github.com/google/fonts/raw/main/ofl/pirataone/PirataOne-Regular.ttf
curl -sSL -o apps/web/src/assets/fonts/OFL.txt https://github.com/google/fonts/raw/main/ofl/pirataone/OFL.txt
wc -c apps/web/src/assets/fonts/PirataOne-Regular.ttf
head -1 apps/web/src/assets/fonts/OFL.txt
```

Expected: `56316` bytes; the OFL first line names the copyright holder.

- [ ] **Step 3: Write the tokens**

Create `apps/web/src/styles/tokens.css`:

```css
:root {
  --pitch: #0b0a0a;
  --coal: #111010;
  --burgundy: #4a1418;
  --blood: #e23127;
  --blood-ink: #ff5a50;
  --bone: #ece7de;
  --parchment: #cfc7bb;
  --taupe: #8a7f70;
  --taupe-ink: #b3a695;
  --ochre: #a8862e;
  --hair: rgba(236, 231, 222, 0.14);
  --blood-hair: rgba(226, 49, 39, 0.42);
  --rail: 56px;
  --inset: 72px;
  --font-goth: "Pirata One", "Cinzel", Georgia, serif;
  --font-rubric: "Cinzel", Georgia, serif;
  --font-body: "EB Garamond", Georgia, serif;
  --font-mono: "IBM Plex Mono", ui-monospace, SFMono-Regular, monospace;
  --font-poster: "Archivo", system-ui, sans-serif;
  --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  --dur-fast: 180ms;
  --dur-slow: 420ms;
}

@media (max-width: 1023px) {
  :root {
    --inset: 22px;
  }
}
```

`--blood-ink` and `--taupe-ink` exist for text below 18px. `--blood` on `--pitch` measures about 4.1:1, under the 4.5:1 floor for small text; `--blood-ink` clears it.

- [ ] **Step 4: Write the base document styles**

Create `apps/web/src/styles/base.css`:

```css
*,
*::before,
*::after {
  box-sizing: border-box;
}

html {
  background: var(--pitch);
  overflow-x: hidden;
  scroll-behavior: smooth;
}

body {
  margin: 0;
  overflow-x: hidden;
  color: var(--bone);
  font-family: var(--font-body);
  font-size: 17px;
  line-height: 1.65;
  background-color: var(--pitch);
  background-image:
    radial-gradient(70% 46% at 74% 8%, rgba(74, 20, 24, 0.55), rgba(74, 20, 24, 0) 70%),
    radial-gradient(60% 40% at 12% 40%, rgba(255, 240, 220, 0.035), rgba(255, 240, 220, 0) 70%),
    radial-gradient(70% 44% at 50% 100%, rgba(226, 49, 39, 0.14), rgba(226, 49, 39, 0) 70%);
}

body::before,
body::after {
  content: "";
  position: fixed;
  inset: 0;
  pointer-events: none;
}

body::before {
  z-index: 96;
  opacity: 0.22;
  mix-blend-mode: overlay;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.75' numOctaves='5'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
}

body::after {
  z-index: 95;
  opacity: 0.5;
  mix-blend-mode: multiply;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='m'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.012' numOctaves='3'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23m)'/%3E%3C/svg%3E");
}

::selection {
  background: var(--blood);
  color: var(--pitch);
}

a {
  color: inherit;
}

button,
select,
textarea {
  font: inherit;
  color: inherit;
}

:focus-visible {
  outline: 2px solid var(--blood-ink);
  outline-offset: 3px;
}

.skip {
  position: absolute;
  left: 16px;
  top: -100px;
  z-index: 100;
  padding: 10px 14px;
  background: var(--bone);
  color: var(--pitch);
  font-family: var(--font-mono);
  font-size: 13px;
  text-decoration: none;
}

.skip:focus-visible {
  top: 16px;
}

@media (prefers-reduced-motion: reduce) {
  html {
    scroll-behavior: auto;
  }

  *,
  *::before,
  *::after {
    animation-duration: 0.001ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.001ms !important;
  }
}
```

- [ ] **Step 5: Write the shared codex layout classes**

Create `apps/web/src/styles/codex.css`:

```css
.codex {
  position: relative;
  z-index: 1;
  width: 100%;
  overflow: hidden;
}

.gutter,
.rule-l,
.rule-r {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--hair);
  z-index: 0;
}

.gutter {
  left: 50%;
}

.rule-l {
  left: var(--rail);
}

.rule-r {
  right: var(--rail);
}

.spread {
  position: relative;
  display: grid;
  grid-template-columns: var(--rail) 1fr 1fr var(--rail);
  border-bottom: 1px solid var(--hair);
}

.spread > .rail {
  grid-column: 1;
  position: relative;
  z-index: 2;
}

.spread > .rail span {
  position: absolute;
  top: 96px;
  left: 50%;
  transform: translateX(-50%) rotate(180deg);
  writing-mode: vertical-rl;
  font-family: var(--font-goth);
  font-size: 13px;
  letter-spacing: 0.22em;
  color: var(--blood-ink);
  white-space: nowrap;
}

.page {
  min-width: 0;
  position: relative;
  z-index: 1;
  padding: 96px var(--inset) 120px;
}

.page.l {
  grid-column: 2;
}

.page.r {
  grid-column: 3;
}

.page.r.mirror {
  text-align: right;
}

.page.r.mirror .copy {
  margin-left: auto;
}

.node {
  position: absolute;
  left: 50%;
  bottom: -3.5px;
  width: 6px;
  height: 6px;
  border: 1px solid var(--blood);
  background: var(--pitch);
  transform: translateX(-50%);
  z-index: 3;
}

.rubric {
  font-family: var(--font-rubric);
  font-size: 12px;
  letter-spacing: 0.22em;
  text-transform: uppercase;
  color: var(--blood-ink);
  margin: 0 0 10px;
}

.rubric.mute {
  color: var(--taupe-ink);
}

.title {
  font-family: var(--font-goth);
  font-weight: 400;
  font-size: 56px;
  line-height: 0.95;
  margin: 0 0 22px;
  color: var(--bone);
}

.copy {
  max-width: 44ch;
  color: var(--parchment);
  margin: 0 0 14px;
}

.copy b {
  color: var(--bone);
  font-weight: 500;
}

.copy code,
.inline-code {
  font-family: var(--font-mono);
  font-size: 0.82em;
  color: var(--bone);
  background: rgba(236, 231, 222, 0.06);
  padding: 1px 5px;
  border: 1px solid var(--hair);
}

.numeral {
  display: inline-block;
  font-family: var(--font-goth);
  font-weight: 400;
  font-size: 320px;
  line-height: 0.8;
  color: var(--blood);
  margin: -24px 0 18px -112px;
  position: relative;
  z-index: 0;
  user-select: none;
}

.numeral .spark {
  position: absolute;
  width: 16px;
  height: 16px;
  left: 0.13em;
  top: 0.36em;
}

.numeral .spark svg {
  display: block;
  width: 100%;
  height: 100%;
  fill: var(--bone);
}

.numeral.two {
  margin: -24px -112px 18px 0;
}

.numeral.four {
  margin-left: -140px;
}

.acts {
  display: flex;
  flex-wrap: wrap;
  gap: 9px;
  margin-top: 22px;
}

.act {
  font-family: var(--font-mono);
  font-size: 13px;
  padding: 13px 18px;
  min-height: 44px;
  cursor: pointer;
  background: var(--blood);
  border: 1px solid var(--blood);
  color: var(--pitch);
  font-weight: 500;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
  transition: background var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
}

.act:hover {
  background: var(--blood-ink);
  border-color: var(--blood-ink);
}

.act.ghost {
  background: transparent;
  color: var(--bone);
  border-color: var(--hair);
}

.act.ghost:hover {
  border-color: var(--blood-ink);
  color: var(--blood-ink);
}

.ledger {
  width: 100%;
  border-collapse: collapse;
  font-size: 14px;
  border-top: 1px solid var(--blood-hair);
  border-bottom: 1px solid var(--blood-hair);
}

.ledger caption {
  text-align: left;
  font-family: var(--font-rubric);
  font-size: 12px;
  letter-spacing: 0.2em;
  text-transform: uppercase;
  color: var(--taupe-ink);
  padding: 0 0 10px;
}

.ledger th,
.ledger td {
  padding: 14px 8px;
  text-align: right;
  border-bottom: 1px solid var(--hair);
  font-variant-numeric: tabular-nums;
}

.ledger th:first-child,
.ledger td:first-child {
  text-align: left;
}

.ledger thead th {
  font-family: var(--font-goth);
  font-weight: 400;
  font-size: 17px;
  color: var(--taupe-ink);
}

.ledger thead th.mine {
  color: var(--blood-ink);
}

.ledger td:first-child {
  font-family: var(--font-rubric);
  font-size: 11px;
  letter-spacing: 0.18em;
  text-transform: uppercase;
  color: var(--taupe-ink);
}

.ledger .us {
  font-family: var(--font-mono);
  color: var(--blood-ink);
}

.ledger .them {
  font-family: var(--font-mono);
  color: var(--taupe-ink);
}

.evidence {
  display: inline-block;
  margin-top: 8px;
  font-family: var(--font-mono);
  font-size: 11px;
  letter-spacing: 0.04em;
  color: var(--taupe-ink);
  border: 1px solid var(--hair);
  padding: 3px 8px;
}

@media (max-width: 1023px) {
  .gutter,
  .rule-l,
  .rule-r,
  .spread > .rail,
  .node {
    display: none;
  }

  .spread {
    grid-template-columns: 1fr;
  }

  .page.l,
  .page.r {
    grid-column: 1;
  }

  .page {
    padding: 64px var(--inset) 72px;
  }

  .page.r.mirror {
    text-align: left;
  }

  .page.r.mirror .copy {
    margin-left: 0;
  }

  .title {
    font-size: 40px;
  }

  .numeral,
  .numeral.two {
    font-size: 200px;
    margin: -16px 0 10px -8px;
  }

  .numeral.four {
    margin-left: -8px;
  }

  .ledger th,
  .ledger td {
    padding: 12px 6px;
  }
}
```

- [ ] **Step 6: Write the head, glyphs, frame, rail, spread, numeral components**

Create `apps/web/src/components/Head.astro`:

```astro
---
interface Props {
  title: string;
  description: string;
}
const { title, description } = Astro.props;
const canonical = new URL(Astro.url.pathname, Astro.site);
const image = new URL("og.png", Astro.site);
---
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{title}</title>
<meta name="description" content={description} />
<link rel="canonical" href={canonical.href} />
<link rel="icon" href="/favicon.svg" type="image/svg+xml" />
<link rel="sitemap" href="/sitemap-index.xml" />
<meta name="theme-color" content="#0b0a0a" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="blasphem" />
<meta property="og:title" content={title} />
<meta property="og:description" content={description} />
<meta property="og:url" content={canonical.href} />
<meta property="og:image" content={image.href} />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta property="og:image:alt" content="blasphem wordmark in red blackletter on black" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content={title} />
<meta name="twitter:description" content={description} />
<meta name="twitter:image" content={image.href} />
```

Create `apps/web/src/components/Glyphs.astro`:

```astro
<svg width="0" height="0" style="position:absolute" aria-hidden="true">
  <defs>
    <path id="star" d="M12 0 C13.2 8.4, 15.6 10.8, 24 12 C15.6 13.2, 13.2 15.6, 12 24 C10.8 15.6, 8.4 13.2, 0 12 C8.4 10.8, 10.8 8.4, 12 0 Z"></path>
    <path id="cross" d="M11 0 H13 V7 H20 V9 H13 V26 H11 V9 H4 V7 H11 Z"></path>
    <clipPath id="archClip" clipPathUnits="objectBoundingBox"><path d="M0 1 L0 .34 Q0 0 .5 0 Q1 0 1 .34 L1 1 Z"></path></clipPath>
    <path id="sealPath" d="M95,95 m-70,0 a70,70 0 1,1 140,0 a70,70 0 1,1 -140,0"></path>
    <symbol id="archFrame" viewBox="0 0 160 220" preserveAspectRatio="none">
      <path d="M2 218 L2 92 Q2 22 80 2 Q158 22 158 92 L158 218 Z" fill="#0b0a0a" stroke="#e23127" stroke-width="1" vector-effect="non-scaling-stroke"></path>
      <path d="M12 218 L12 94 Q12 32 80 14 Q148 32 148 94 L148 218" fill="none" stroke="rgba(226,49,39,.35)" stroke-width="1" vector-effect="non-scaling-stroke"></path>
    </symbol>
    <symbol id="thornRing" viewBox="0 0 200 200">
      <circle cx="100" cy="100" r="70" stroke-width="1.5"></circle>
      <g stroke-width="1.5">
        <path d="M100 30 L96 14 M100 30 L106 16"></path><path d="M135 39 L138 22 M135 39 L145 28"></path><path d="M161 61 L172 50 M161 61 L176 62"></path><path d="M170 100 L186 96 M170 100 L184 106"></path>
        <path d="M161 139 L176 138 M161 139 L172 150"></path><path d="M135 161 L145 172 M135 161 L138 178"></path><path d="M100 170 L106 184 M100 170 L96 186"></path><path d="M65 161 L62 178 M65 161 L55 172"></path>
        <path d="M39 139 L28 150 M39 139 L24 138"></path><path d="M30 100 L14 104 M30 100 L16 94"></path><path d="M39 61 L24 62 M39 61 L28 50"></path><path d="M65 39 L55 28 M65 39 L62 22"></path>
      </g>
    </symbol>
  </defs>
</svg>
```

Create `apps/web/src/components/Frame.astro`:

```astro
<div class="frame" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
<div class="scratch" aria-hidden="true"></div>
<div class="vignette" aria-hidden="true"></div>

<style>
  .frame {
    position: fixed;
    inset: 9px;
    border: 1px solid rgba(236, 231, 222, 0.1);
    z-index: 92;
    pointer-events: none;
  }
  .frame i {
    position: absolute;
    width: 13px;
    height: 13px;
    border: 1px solid var(--blood);
  }
  .frame i:nth-child(1) { top: -1px; left: -1px; border-right: 0; border-bottom: 0; }
  .frame i:nth-child(2) { top: -1px; right: -1px; border-left: 0; border-bottom: 0; }
  .frame i:nth-child(3) { bottom: -1px; left: -1px; border-right: 0; border-top: 0; }
  .frame i:nth-child(4) { bottom: -1px; right: -1px; border-left: 0; border-top: 0; }
  .scratch {
    position: fixed;
    inset: 0;
    z-index: 94;
    pointer-events: none;
    background-image:
      linear-gradient(112deg, transparent 49.92%, rgba(236, 231, 222, 0.04) 50%, transparent 50.08%),
      linear-gradient(64deg, transparent 49.94%, rgba(236, 231, 222, 0.03) 50%, transparent 50.06%),
      linear-gradient(98deg, transparent 49.95%, rgba(236, 231, 222, 0.025) 50%, transparent 50.05%);
    background-size: 180% 100%, 140% 100%, 220% 100%;
    background-position: 20% 0, 70% 0, 45% 0;
  }
  .vignette {
    position: fixed;
    inset: 0;
    z-index: 93;
    pointer-events: none;
    background: radial-gradient(110% 80% at 50% 40%, rgba(0, 0, 0, 0) 40%, rgba(0, 0, 0, 0.86) 100%);
  }
</style>
```

Create `apps/web/src/components/RailRight.astro`:

```astro
---
const words = ["profanity", "targeted abuse", "identity attack", "threat language", "harm wish", "self-harm command", "negation", "quotation", "counterspeech"];
const run = Array.from({ length: 4 }, () => words).flat();
---
<aside class="rail-r" aria-hidden="true">
  <div class="track">
    <span>{run.map((word) => <Fragment>{word}<i>&#10022;</i></Fragment>)}</span>
    <span>{run.map((word) => <Fragment>{word}<i>&#10022;</i></Fragment>)}</span>
  </div>
</aside>

<style>
  .rail-r {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: var(--rail);
    overflow: hidden;
    z-index: 4;
    background: var(--pitch);
  }
  .track {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    animation: climb 90s linear infinite;
  }
  span {
    display: block;
    writing-mode: vertical-rl;
    font-family: var(--font-goth);
    font-size: 20px;
    line-height: var(--rail);
    color: var(--blood);
    white-space: nowrap;
    opacity: 0.85;
  }
  i {
    font-style: normal;
    color: var(--bone);
    opacity: 0.45;
    padding: 14px 0;
  }
  @keyframes climb {
    from { transform: translateY(0); }
    to { transform: translateY(-50%); }
  }
  @media (max-width: 1023px) {
    .rail-r { display: none; }
  }
</style>
```

Create `apps/web/src/components/Spread.astro`:

```astro
---
interface Props {
  id: string;
  label?: string;
  class?: string;
}
const { id, label, class: className } = Astro.props;
---
<section id={id} class:list={["spread", className]} aria-labelledby={`${id}-title`}>
  <div class="rail">{label && <span>{label}</span>}</div>
  <slot />
  <i class="node" aria-hidden="true"></i>
</section>
```

Create `apps/web/src/components/Numeral.astro`:

```astro
---
interface Props {
  value: string;
  class?: string;
}
const { value, class: className } = Astro.props;
---
<div class:list={["numeral", className]} aria-hidden="true">{value}<i class="spark"><svg viewBox="0 0 24 24"><use href="#star"></use></svg></i></div>
```

- [ ] **Step 7: Write the layout**

Create `apps/web/src/layouts/Codex.astro`:

```astro
---
import "@fontsource/pirata-one/400.css";
import "@fontsource/cinzel/400.css";
import "@fontsource/cinzel/700.css";
import "@fontsource/eb-garamond/400.css";
import "@fontsource/eb-garamond/400-italic.css";
import "@fontsource/eb-garamond/500.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "@fontsource/archivo/800.css";
import "../styles/tokens.css";
import "../styles/base.css";
import "../styles/codex.css";
import Frame from "../components/Frame.astro";
import Glyphs from "../components/Glyphs.astro";
import Head from "../components/Head.astro";
import RailRight from "../components/RailRight.astro";

interface Props {
  title: string;
  description: string;
}
const { title, description } = Astro.props;
---
<!doctype html>
<html lang="en">
  <head>
    <Head title={title} description={description} />
  </head>
  <body>
    <a class="skip" href="#main">Skip to content</a>
    <Glyphs />
    <Frame />
    <main id="main" class="codex">
      <div class="gutter" aria-hidden="true"></div>
      <div class="rule-l" aria-hidden="true"></div>
      <div class="rule-r" aria-hidden="true"></div>
      <RailRight />
      <slot />
    </main>
  </body>
</html>
```

- [ ] **Step 8: Write the Open Graph endpoint**

Create `apps/web/src/pages/og.png.ts`:

```ts
import { Resvg } from "@resvg/resvg-js";
import type { APIRoute } from "astro";
import { fileURLToPath } from "node:url";

const fontFile = fileURLToPath(new URL("../assets/fonts/PirataOne-Regular.ttf", import.meta.url));

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#0b0a0a"/>
  <rect x="24" y="24" width="1152" height="582" fill="none" stroke="#ece7de" stroke-opacity="0.16"/>
  <path d="M24 24h14v1H25v13h-1z M1176 24v14h-1V25h-13v-1z M24 606v-14h1v13h13v1z M1176 606h-14v-1h13v-13h1z" fill="#e23127"/>
  <text x="600" y="345" text-anchor="middle" font-family="Pirata One" font-size="280" fill="#e23127">blasphem</text>
  <text x="600" y="455" text-anchor="middle" font-family="Pirata One" font-size="38" fill="#ece7de">hostile messages, judged in the browser before they send</text>
  <text x="600" y="560" text-anchor="middle" font-family="Pirata One" font-size="24" fill="#8a7f70">fifteen languages · deterministic · no request after load</text>
</svg>`;

export const GET: APIRoute = () => {
  const renderer = new Resvg(svg, {
    fitTo: { mode: "width", value: 1200 },
    font: { loadSystemFonts: false, fontFiles: [fontFile], defaultFontFamily: "Pirata One" },
  });
  const png = renderer.render().asPng();
  return new Response(png, { headers: { "Content-Type": "image/png" } });
};
```

- [ ] **Step 9: Point the page at the layout**

Replace `apps/web/src/pages/index.astro`:

```astro
---
import Codex from "../layouts/Codex.astro";
import Spread from "../components/Spread.astro";

const title = "blasphem · a pre-send nudge for hostile messages";
const description = "Deterministic multilingual toxicity detection compiled to WebAssembly. Fifteen languages, ordinal scores, no request after the module loads.";
---
<Codex title={title} description={description}>
  <Spread id="front" label="">
    <div class="page l">
      <h1 id="front-title" class="title">blasphem</h1>
      <p class="copy">Shell. Chapters arrive in Tasks 7 to 11.</p>
    </div>
  </Spread>
</Codex>
```

- [ ] **Step 10: Verify**

Run: `pnpm --filter web build 2>&1 | tail -6`
Expected: `2 page(s) built` or `1 page(s) built` plus the `og.png` endpoint line, then `Complete!`.

Run: `file apps/web/dist/og.png`
Expected: `PNG image data, 1200 x 630`.

Run: `grep -c 'property="og:image"' apps/web/dist/index.html; grep -c 'rel="canonical" href="https://blasphem.sospedra.me/"' apps/web/dist/index.html; grep -c "fonts.googleapis" apps/web/dist/index.html`
Expected: `1`, `1`, `0`.

Run: `ls apps/web/dist/_astro/*.woff2 | wc -l`
Expected: a count of at least `9` (one file per imported weight and subset).

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 11: Commit**

```bash
git add apps/web pnpm-lock.yaml
git commit -m "Add the codex shell, fonts, metadata, and Open Graph image"
```

---

### Task 7: The frontispiece

**Files:**
- Create: `apps/web/src/components/Frontispiece.astro`
- Create: `apps/web/src/components/Seal.astro`
- Create: `apps/web/src/components/Advisory.astro`
- Create: `apps/web/src/components/Badges.astro`
- Create: `apps/web/src/assets/bust.jpg`
- Modify: `apps/web/src/pages/index.astro`

**Interfaces:**
- Consumes: `browser`, `performance`, `routing` from `reports.ts`; `worstP95Ms`, `routingTotals` from `metrics.ts`; `LANGUAGES`; formatters; `packages/blasphem/package.json` `version`.
- Produces: `<Frontispiece />` rendering `<section id="front">` with `<h1 id="front-title">`; in-page nav links to `#detector`, `#rite`, `#vows`, `#reckoning`, `#colophon`.

- [ ] **Step 1: Fetch the CC0 plate image**

```bash
curl -sSL -o apps/web/src/assets/bust.jpg "https://commons.wikimedia.org/wiki/Special:FilePath/Marble_Portrait_Bust_of_a_Woman_with_a_Scroll_MET_DP345059.jpg?width=1200"
wc -c apps/web/src/assets/bust.jpg
file apps/web/src/assets/bust.jpg
```

Expected: `248857` bytes, `JPEG image data`. The Met releases this object under CC0; the colophon credits it.

- [ ] **Step 2: Write the seal, advisory, and badges**

Create `apps/web/src/components/Seal.astro`:

```astro
<svg class="seal" viewBox="0 0 190 190" aria-hidden="true">
  <circle cx="95" cy="95" r="88" fill="none" stroke="#ece7de" stroke-opacity=".7"></circle>
  <circle cx="95" cy="95" r="54" fill="none" stroke="#ece7de" stroke-opacity=".7"></circle>
  <text><textPath href="#sealPath">in principio erat verbum &#10022; </textPath></text>
  <g transform="translate(83,83)" fill="#a8862e"><use href="#star"></use></g>
</svg>

<style>
  .seal {
    position: absolute;
    left: calc(50% + var(--inset));
    top: calc(72px + 50vh);
    transform: translate(-50%, -50%);
    width: 190px;
    height: 190px;
    z-index: 5;
  }
  .seal text {
    font-family: var(--font-goth);
    font-size: 13.5px;
    letter-spacing: 0.16em;
    fill: var(--bone);
  }
  @media (max-width: 1023px) {
    .seal {
      position: relative;
      left: auto;
      top: auto;
      transform: none;
      width: 120px;
      height: 120px;
      margin: -60px 0 0 calc(100% - var(--inset) - 76% - 60px);
    }
  }
</style>
```

Create `apps/web/src/components/Advisory.astro`:

```astro
<div class="advisory" aria-hidden="true">
  <div class="t">Content<br />Advisory</div>
  <div class="b">judged locally</div>
</div>

<style>
  .advisory {
    position: absolute;
    left: 50%;
    bottom: 0;
    transform: translate(-50%, 50%);
    z-index: 6;
    width: 230px;
    border: 2px solid var(--bone);
    background: var(--pitch);
  }
  .t {
    padding: 9px 10px 7px;
    text-align: center;
    font-family: var(--font-poster);
    font-weight: 800;
    font-size: 18px;
    line-height: 0.95;
    text-transform: uppercase;
    color: var(--bone);
  }
  .b {
    background: var(--bone);
    color: var(--pitch);
    text-align: center;
    font-family: var(--font-poster);
    font-weight: 800;
    font-size: 11px;
    letter-spacing: 0.1em;
    padding: 4px 6px;
    text-transform: uppercase;
  }
  @media (max-width: 1023px) {
    .advisory {
      position: relative;
      left: auto;
      bottom: auto;
      transform: none;
      margin: 40px 0 -40px var(--inset);
      width: 200px;
    }
  }
</style>
```

Create `apps/web/src/components/Badges.astro`:

```astro
---
import { formatInt, formatMegabytes, formatMs, formatPercent } from "../lib/format";
import { LANGUAGES } from "../lib/languages";
import { routingTotals, worstP95Ms } from "../lib/metrics";
import { browser, performance, routing } from "../lib/reports";

const totals = routingTotals(routing);
const badges = [
  {
    value: String(LANGUAGES.length),
    label: "languages",
    shape: "diamond",
    title: LANGUAGES.map((language) => language.code).join(" "),
  },
  {
    value: formatMegabytes(browser.browser_builds.full.brotli_total_bytes, 1),
    label: "brotli transfer",
    shape: "rect",
    title: `${browser.evidence_status}: default build, WASM plus glue, Chromium ${browser.browser_version}`,
  },
  {
    value: formatMs(worstP95Ms(performance.fixtures, "-280"), 2),
    label: "worst p95 · 280 chars",
    shape: "ellipse",
    title: `${performance.evidence_status}: native release build on ${performance.computer}`,
  },
  {
    value: formatPercent(totals.knownPrecision, 1),
    label: "known-route precision",
    shape: "slashed",
    title: `${routing.evidence_status}: ${formatInt(totals.rows)} supported Tatoeba rows`,
  },
] as const;
---
<ul class="badges">
  {badges.map((badge) => (
    <li class="badge" title={badge.title}>
      <svg viewBox="0 0 120 64" preserveAspectRatio="none" aria-hidden="true">
        {badge.shape === "diamond" && <path d="M60 3 L117 32 L60 61 L3 32 Z"></path>}
        {badge.shape === "rect" && <rect x="3" y="9" width="114" height="46"></rect>}
        {badge.shape === "ellipse" && <ellipse cx="60" cy="32" rx="57" ry="27"></ellipse>}
        {badge.shape === "slashed" && <ellipse cx="60" cy="32" rx="38" ry="28"></ellipse>}
        {badge.shape === "slashed" && <path d="M36 54 L84 10"></path>}
      </svg>
      <span>{badge.value}<em>{badge.label}</em></span>
    </li>
  ))}
</ul>

<style>
  .badges {
    grid-column: 3;
    grid-row: 3;
    align-self: end;
    margin: 0 var(--inset) 88px;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
    position: relative;
    z-index: 2;
  }
  .badge {
    position: relative;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .badge svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    fill: var(--pitch);
    stroke: var(--blood);
    vector-effect: non-scaling-stroke;
  }
  .badge svg * {
    vector-effect: non-scaling-stroke;
  }
  .badge span {
    position: relative;
    font-family: var(--font-goth);
    font-size: 16px;
    color: var(--blood-ink);
    line-height: 1;
    text-align: center;
  }
  .badge em {
    display: block;
    font-style: normal;
    font-family: var(--font-rubric);
    font-size: 10px;
    letter-spacing: 0.2em;
    color: var(--taupe-ink);
    text-transform: uppercase;
    margin-top: 4px;
  }
  @media (max-width: 1023px) {
    .badges {
      grid-column: 1;
      grid-row: auto;
      order: 6;
      grid-template-columns: 1fr 1fr;
      margin: 40px var(--inset) 0;
    }
  }
</style>
```

- [ ] **Step 3: Write the frontispiece**

Create `apps/web/src/components/Frontispiece.astro`:

```astro
---
import { Image } from "astro:assets";
import bust from "../assets/bust.jpg";
import manifest from "../../../../packages/blasphem/package.json";
import Advisory from "./Advisory.astro";
import Badges from "./Badges.astro";
import Seal from "./Seal.astro";

const links = [
  { href: "#detector", label: "detector" },
  { href: "#rite", label: "rite" },
  { href: "#vows", label: "vows" },
  { href: "#reckoning", label: "reckoning" },
  { href: "#colophon", label: "colophon" },
  { href: "https://github.com/sospedra/blasphem", label: "github" },
];
const words = ["profanity", "targeted abuse", "identity attack", "threat language", "harm wish", "self-harm command", "counterspeech"];
---
<section id="front" class="spread front" aria-labelledby="front-title">
  <div class="rail" aria-hidden="true"></div>
  <header class="stamp">
    <svg width="12" height="16" viewBox="0 0 24 26" aria-hidden="true"><use href="#cross"></use></svg>
    <span>blasphem</span>
    <nav aria-label="Chapters">
      {links.map((link) => <a href={link.href}>{link.label}</a>)}
    </nav>
  </header>
  <div class="meta">
    <span>v{manifest.version}</span>
    <span>{manifest.license}</span>
    <span>experimental</span>
  </div>

  <div class="arch-plate">
    <Image src={bust} alt="" widths={[420, 640, 900]} sizes="(max-width: 1023px) 76vw, 31vh" class="photo" loading="eager" />
    <div class="wash"></div>
    <svg class="edge" preserveAspectRatio="none" viewBox="0 0 160 220" aria-hidden="true">
      <path d="M1 219 L1 75 Q1 1 80 1 Q159 1 159 75 L159 219" fill="none" stroke="#e23127" stroke-width="1" vector-effect="non-scaling-stroke"></path>
      <path d="M11 219 L11 78 Q11 13 80 13 Q149 13 149 78 L149 219" fill="none" stroke="rgba(226,49,39,.35)" stroke-width="1" vector-effect="non-scaling-stroke"></path>
    </svg>
  </div>

  <h1 id="front-title" class="wordmark">blas<span class="brk"></span>phem<i class="sp1"><svg viewBox="0 0 24 24"><use href="#star"></use></svg></i><i class="sp2"><svg viewBox="0 0 24 24"><use href="#star"></use></svg></i><i class="sp3"><svg viewBox="0 0 24 24"><use href="#star"></use></svg></i></h1>

  <Seal />

  <div class="lead">
    <p class="strap">hostile messages, judged before they send</p>
    <p class="copy">Deterministic rules, HurtLex lexica, and one sparse integer table per language, compiled to <b>WebAssembly</b>. No AI runtime. No request after the module loads. The verdict lands in the browser before the submit fires.</p>
    <div class="acts">
      <a class="act" href="#detector">Try the detector</a>
      <a class="act ghost" href="#rite">Read the rite</a>
    </div>
  </div>

  <Badges />
  <Advisory />
  <i class="node" aria-hidden="true"></i>
</section>

<div class="band" aria-hidden="true">
  <div class="run">
    <span>{words.map((word) => <Fragment>{word}<i>&#10022;</i></Fragment>)}</span>
    <span>{words.map((word) => <Fragment>{word}<i>&#10022;</i></Fragment>)}</span>
  </div>
</div>

<style>
  .front {
    min-height: 100vh;
    grid-template-rows: 72px auto 1fr;
  }
  .front > .rail {
    grid-row: 1 / 4;
  }
  .stamp {
    grid-column: 2;
    grid-row: 1;
    align-self: center;
    padding-left: var(--inset);
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--font-goth);
    font-size: 13px;
    letter-spacing: 0.06em;
    color: var(--blood-ink);
    position: relative;
    z-index: 2;
  }
  .stamp svg {
    fill: var(--blood);
  }
  .stamp nav {
    display: flex;
    gap: 14px;
    margin-left: 18px;
    font-family: var(--font-rubric);
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  .stamp nav a {
    color: var(--taupe-ink);
    text-decoration: none;
    padding: 6px 0;
    border-bottom: 1px solid transparent;
    transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
  }
  .stamp nav a:hover {
    color: var(--blood-ink);
    border-color: var(--blood-ink);
  }
  .meta {
    grid-column: 3;
    grid-row: 1;
    align-self: center;
    padding-right: var(--inset);
    display: flex;
    justify-content: flex-end;
    gap: 18px;
    font-family: var(--font-rubric);
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
    color: var(--taupe-ink);
    position: relative;
    z-index: 2;
  }
  .wordmark {
    grid-column: 2 / 5;
    grid-row: 2;
    position: relative;
    z-index: 3;
    margin: calc(32vh - 72px) 0 0 var(--inset);
    font-family: var(--font-goth);
    font-weight: 400;
    font-size: 28vw;
    line-height: 0.8;
    color: var(--blood);
    letter-spacing: -0.005em;
    white-space: nowrap;
    filter: drop-shadow(0 0 44px rgba(226, 49, 39, 0.22));
  }
  .wordmark .brk {
    display: none;
  }
  .wordmark i {
    position: absolute;
    width: 18px;
    height: 18px;
    z-index: 4;
  }
  .wordmark i svg {
    fill: var(--bone);
    display: block;
    width: 18px;
    height: 18px;
  }
  .wordmark .sp1 { left: 0.24em; top: 0.45em; }
  .wordmark .sp2 { left: 0.82em; top: 0.58em; }
  .wordmark .sp3 { left: 2.48em; top: 0.3em; }
  .lead {
    grid-column: 2;
    grid-row: 3;
    align-self: start;
    padding: 24px var(--inset) 0;
    position: relative;
    z-index: 2;
  }
  .strap {
    font-style: italic;
    font-size: 26px;
    line-height: 1.25;
    color: var(--bone);
    margin: 0 0 18px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--hair);
    display: inline-block;
  }
  .arch-plate {
    position: absolute;
    top: 72px;
    left: calc(50% + var(--inset));
    height: 50vh;
    width: calc(50vh * 0.62);
    z-index: 1;
  }
  .arch-plate .edge {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    z-index: 3;
  }
  .arch-plate :global(.photo) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: 50% 18%;
    clip-path: url(#archClip);
    filter: sepia(1) saturate(0.6) contrast(0.72) brightness(0.55) blur(1.4px);
    opacity: 0.85;
    z-index: 1;
  }
  .arch-plate .wash {
    position: absolute;
    inset: 0;
    z-index: 2;
    clip-path: url(#archClip);
    background: linear-gradient(180deg, rgba(74, 20, 24, 0.18) 0%, rgba(11, 10, 10, 0) 30%, rgba(11, 10, 10, 0.35) 62%, rgba(11, 10, 10, 0.96) 100%);
  }
  .band {
    display: none;
  }
  @media (max-width: 1023px) {
    .front {
      min-height: 0;
      grid-template-rows: none;
      padding-bottom: 0;
    }
    .stamp {
      grid-column: 1;
      grid-row: auto;
      order: 1;
      padding: 22px var(--inset) 0;
      flex-wrap: wrap;
    }
    .stamp nav {
      margin-left: 0;
      flex-basis: 100%;
      flex-wrap: wrap;
      gap: 10px 14px;
    }
    .meta {
      display: none;
    }
    .wordmark {
      grid-column: 1;
      grid-row: auto;
      order: 2;
      margin: 38px 0 0 var(--inset);
      font-size: 42vw;
      filter: none;
    }
    .wordmark .brk {
      display: block;
    }
    .wordmark .sp3 { left: 0.85em; top: 1.1em; }
    .arch-plate {
      position: relative;
      order: 3;
      top: auto;
      left: auto;
      height: auto;
      width: 76%;
      aspect-ratio: 3 / 4.3;
      margin: 34px var(--inset) 0 auto;
    }
    .lead {
      grid-column: 1;
      grid-row: auto;
      order: 5;
      padding: 30px var(--inset) 0;
    }
    .strap {
      font-size: 20px;
    }
    .band {
      display: block;
      width: 100vw;
      overflow: hidden;
      border-top: 1px solid var(--blood-hair);
      border-bottom: 1px solid var(--blood-hair);
      background: rgba(226, 49, 39, 0.05);
      margin-top: 60px;
    }
    .band .run {
      display: flex;
      width: max-content;
      animation: slide 30s linear infinite;
    }
    .band span {
      font-family: var(--font-goth);
      font-size: 26px;
      color: var(--blood);
      white-space: nowrap;
      padding: 5px 0;
    }
    .band i {
      font-style: normal;
      padding: 0 14px;
      color: var(--bone);
      opacity: 0.45;
    }
  }
  @keyframes slide {
    from { transform: translateX(0); }
    to { transform: translateX(-50%); }
  }
</style>
```

The `Seal` and `Advisory` components position themselves against `.front` (`position: absolute` inside the `position: relative` spread). On small screens they flow with `order` set on the spread children: add to `Seal.astro` `order: 4` and to `Advisory.astro` `order: 7` inside their `@media (max-width: 1023px)` blocks.

- [ ] **Step 4: Mount it on the page**

Replace the `<Spread id="front">` block in `apps/web/src/pages/index.astro` with `<Frontispiece />` and import it:

```astro
---
import Codex from "../layouts/Codex.astro";
import Frontispiece from "../components/Frontispiece.astro";

const title = "blasphem · a pre-send nudge for hostile messages";
const description = "Deterministic multilingual toxicity detection compiled to WebAssembly. Fifteen languages, ordinal scores, no request after the module loads.";
---
<Codex title={title} description={description}>
  <Frontispiece />
</Codex>
```

- [ ] **Step 5: Verify**

Run: `pnpm --filter web build 2>&1 | tail -4`
Expected: `Complete!` and no `[WARN]` about the image.

Run:

```bash
python3 - <<'PY'
import json, re
b = json.load(open("reports/multilingual-wasm.json"))
mb = f"{b['browser_builds']['full']['brotli_total_bytes']/1e6:.1f} MB"
html = open("apps/web/dist/index.html").read()
print("transfer badge:", mb in html)
print("h1:", 'id="front-title"' in html)
print("nav links:", all(f'href="#{a}"' in html for a in ["detector", "rite", "vows", "reckoning", "colophon"]))
print("image srcset:", "srcset=" in html)
PY
```

Expected: four `True` lines.

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 6: Commit**

```bash
git add apps/web
git commit -m "Add the frontispiece with report-driven badges"
```

---

### Task 8: Chapter I, the detector playground

**Files:**
- Create: `apps/web/src/scripts/playground-state.ts`
- Create: `apps/web/src/scripts/playground.ts`
- Create: `apps/web/src/components/Detector.astro`
- Modify: `apps/web/src/pages/index.astro`

**Interfaces:**
- Consumes: `__BLASPHEM_BASE__`, `__BLASPHEM_WASM_BYTES__` (Task 4); `normalizeSelection`, `LANGUAGES` (Task 5); `SAMPLES` (Task 5); `typeof import("blasphem")` (Task 3).
- Produces: `mountPlayground(root: HTMLElement): void`; pure `transition(phase, event)`, `verdictFor(snapshot)`, `statusCopy(phase, megabytes)`; markup ids `playground`, `message`, `language`, `status`, `verdict`, `ruling`, `note`, `clock`, `meter`, `bar`, `failure`, `failure-message`, `retry`, `f-ok`, `f-score`, `f-nudge`, `f-lang`, `f-evaluated`, `f-langscore`; sample buttons `[data-sample][data-code][data-text]`.

- [ ] **Step 1: Write the pure state module**

Create `apps/web/src/scripts/playground-state.ts`:

```ts
export type Phase =
  | { status: "idle" }
  | { status: "unavailable" }
  | { status: "loading" }
  | { status: "ready" }
  | { status: "error"; message: string };

export type PhaseEvent =
  | { type: "LOAD" }
  | { type: "LOADED" }
  | { type: "FAILED"; message: string }
  | { type: "RETRY" }
  | { type: "UNAVAILABLE" };

export function transition(phase: Phase, event: PhaseEvent): Phase {
  switch (event.type) {
    case "LOAD":
      return phase.status === "idle" ? { status: "loading" } : phase;
    case "LOADED":
      return phase.status === "loading" ? { status: "ready" } : phase;
    case "FAILED":
      return phase.status === "loading" ? { status: "error", message: event.message } : phase;
    case "RETRY":
      return phase.status === "error" ? { status: "idle" } : phase;
    case "UNAVAILABLE":
      return { status: "unavailable" };
  }
}

export type Snapshot = {
  ok: boolean;
  score: number;
  threshold: number;
  shouldNudge: boolean;
  evaluated: boolean;
  resolvedLanguage: string;
  languageReliable: boolean;
  languageScore: number | undefined;
};

export type Tone = "clean" | "hit" | "unknown";
export type Verdict = { word: string; tone: Tone; note: string };

const UNHEARD: Verdict = { word: "Unheard", tone: "unknown", note: "no reliable language route · the nudge fails open" };
const CONDEMNED: Verdict = { word: "Condemned", tone: "hit", note: "shouldNudge is true · the pre-send nudge fires" };
const ABSOLVED: Verdict = { word: "Absolved", tone: "clean", note: "ok is true · no nudge" };

export function verdictFor(snapshot: Pick<Snapshot, "evaluated" | "shouldNudge">): Verdict {
  if (!snapshot.evaluated) return UNHEARD;
  return snapshot.shouldNudge ? CONDEMNED : ABSOLVED;
}

export function statusCopy(phase: Phase, megabytes: string): string {
  const copy: Record<Phase["status"], string> = {
    idle: `Type to wake the detector. The first keystroke fetches the ${megabytes} module. Nothing you write leaves this page.`,
    loading: `Fetching the ${megabytes} module and initializing. Keep typing; the check runs when it lands.`,
    ready: "Ready. Every check runs inside the page and times itself.",
    error: "The detector failed to start.",
    unavailable: "The browser package is not built. Run pnpm --filter blasphem run build, then rebuild the site.",
  };
  return copy[phase.status];
}
```

- [ ] **Step 2: Write the DOM shell**

Create `apps/web/src/scripts/playground.ts`:

```ts
import type { BlasphemDetector, BlasphemResult } from "blasphem";
import { normalizeSelection, type Selection } from "../lib/languages";
import { statusCopy, transition, verdictFor, type Phase, type PhaseEvent, type Snapshot } from "./playground-state";

type Module = typeof import("blasphem");

const BASE = __BLASPHEM_BASE__;
const MEGABYTES = `${(__BLASPHEM_WASM_BYTES__ / 1_000_000).toFixed(1)} MB`;
const FIELD_IDS = ["f-ok", "f-score", "f-nudge", "f-lang", "f-evaluated", "f-langscore"] as const;

type FieldId = (typeof FIELD_IDS)[number];

type Elements = {
  root: HTMLElement;
  message: HTMLTextAreaElement;
  language: HTMLSelectElement;
  status: HTMLElement;
  verdict: HTMLElement;
  ruling: HTMLElement;
  note: HTMLElement;
  clock: HTMLElement;
  meter: HTMLElement;
  bar: HTMLElement;
  failure: HTMLElement;
  failureMessage: HTMLElement;
  retry: HTMLButtonElement;
  fields: Record<FieldId, HTMLElement>;
};

type Session = {
  phase: Phase;
  module: Module | null;
  detectors: Map<Selection, BlasphemDetector>;
};

function required<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelector<T>(selector);
  if (!found) throw new Error(`playground markup lacks ${selector}`);
  return found;
}

function collect(root: HTMLElement): Elements {
  const fields = Object.fromEntries(FIELD_IDS.map((id) => [id, required<HTMLElement>(root, `#${id}`)])) as Record<FieldId, HTMLElement>;
  return {
    root,
    message: required(root, "#message"),
    language: required(root, "#language"),
    status: required(root, "#status"),
    verdict: required(root, "#verdict"),
    ruling: required(root, "#ruling"),
    note: required(root, "#note"),
    clock: required(root, "#clock"),
    meter: required(root, "#meter"),
    bar: required(root, "#bar"),
    failure: required(root, "#failure"),
    failureMessage: required(root, "#failure-message"),
    retry: required(root, "#retry"),
    fields,
  };
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function loadModule(): Promise<Module> {
  const glue = (await import(/* @vite-ignore */ `${BASE}/blasphem.js`)) as Module;
  await glue.default({ module_or_path: new URL(`${BASE}/blasphem_bg.wasm`, location.href) });
  return glue;
}

function snapshot(result: BlasphemResult): Snapshot {
  return {
    ok: result.ok,
    score: result.score,
    threshold: result.threshold,
    shouldNudge: result.shouldNudge,
    evaluated: result.evaluated,
    resolvedLanguage: result.resolvedLanguage,
    languageReliable: result.languageReliable,
    languageScore: result.languageScore,
  };
}

function renderPhase(elements: Elements, phase: Phase): void {
  elements.root.dataset.state = phase.status;
  elements.status.textContent = statusCopy(phase, MEGABYTES);
  elements.failure.hidden = phase.status !== "error";
  elements.failureMessage.textContent = phase.status === "error" ? phase.message : "";
  elements.message.disabled = phase.status === "unavailable";
}

function renderResult(elements: Elements, taken: Snapshot, elapsedMs: number): void {
  const verdict = verdictFor(taken);
  elements.verdict.hidden = false;
  elements.verdict.dataset.tone = verdict.tone;
  elements.ruling.textContent = verdict.word;
  elements.note.textContent = verdict.note;
  elements.clock.textContent = `${elapsedMs.toFixed(3)} ms`;
  elements.meter.style.setProperty("--threshold", `${taken.threshold}%`);
  elements.bar.style.width = `${taken.score}%`;
  elements.fields["f-ok"].textContent = String(taken.ok);
  elements.fields["f-score"].textContent = `${taken.score} / ${taken.threshold}`;
  elements.fields["f-nudge"].textContent = String(taken.shouldNudge);
  elements.fields["f-lang"].textContent = taken.resolvedLanguage;
  elements.fields["f-evaluated"].textContent = String(taken.evaluated);
  elements.fields["f-langscore"].textContent = taken.languageScore === undefined ? "none" : taken.languageScore.toFixed(3);
}

function applyQuerySelection(select: HTMLSelectElement): void {
  const requested = new URLSearchParams(location.search).get("lang");
  if (!requested) return;
  const selection = normalizeSelection(requested);
  if (selection) select.value = selection;
}

export function mountPlayground(root: HTMLElement): void {
  const elements = collect(root);
  const session: Session = { phase: { status: "idle" }, module: null, detectors: new Map() };

  const dispatch = (event: PhaseEvent): void => {
    session.phase = transition(session.phase, event);
    renderPhase(elements, session.phase);
  };

  const disposeDetectors = (): void => {
    for (const detector of session.detectors.values()) detector.free();
    session.detectors.clear();
  };

  const currentSelection = (): Selection => normalizeSelection(elements.language.value) ?? "AUTO";

  const detectorFor = (module: Module, selection: Selection): BlasphemDetector => {
    const cached = session.detectors.get(selection);
    if (cached) return cached;
    disposeDetectors();
    const created = new module.BlasphemDetector(selection);
    session.detectors.set(selection, created);
    return created;
  };

  const evaluate = (module: Module): void => {
    const text = elements.message.value;
    if (text.trim() === "") {
      elements.verdict.hidden = true;
      return;
    }
    const detector = detectorFor(module, currentSelection());
    const started = performance.now();
    const result = detector.check(text);
    const elapsed = performance.now() - started;
    const taken = snapshot(result);
    result.free();
    renderResult(elements, taken, elapsed);
  };

  const ensureModule = async (): Promise<Module | null> => {
    if (session.module) return session.module;
    if (session.phase.status !== "idle") return null;
    dispatch({ type: "LOAD" });
    try {
      session.module = await loadModule();
    } catch (error) {
      dispatch({ type: "FAILED", message: describe(error) });
      return null;
    }
    dispatch({ type: "LOADED" });
    return session.module;
  };

  const check = async (): Promise<void> => {
    if (BASE === "") {
      dispatch({ type: "UNAVAILABLE" });
      return;
    }
    const module = await ensureModule();
    if (module) evaluate(module);
  };

  const useSample = (button: HTMLButtonElement): void => {
    const sampleCode = normalizeSelection(button.dataset.code ?? "") ?? "AUTO";
    elements.language.value = currentSelection() === "AUTO" ? "AUTO" : sampleCode;
    elements.message.value = button.dataset.text ?? "";
    elements.message.focus();
    disposeDetectors();
    void check();
  };

  elements.message.addEventListener("input", () => void check());
  elements.language.addEventListener("change", () => {
    disposeDetectors();
    void check();
  });
  elements.retry.addEventListener("click", () => {
    dispatch({ type: "RETRY" });
    void check();
  });
  for (const button of document.querySelectorAll<HTMLButtonElement>("[data-sample]")) {
    button.addEventListener("click", () => useSample(button));
  }
  window.addEventListener("pagehide", disposeDetectors);

  applyQuerySelection(elements.language);
  renderPhase(elements, session.phase);
}
```

Rules this module honors: the module downloads only inside `loadModule`, which only `ensureModule` calls after the first user event. `init` receives the `.wasm` URL explicitly, so no `<link rel="modulepreload">` or asset hint exists in the HTML. Every `BlasphemResult` is freed right after `snapshot`. At most one `BlasphemDetector` lives at a time; language changes and `pagehide` free it.

- [ ] **Step 3: Write the chapter markup and styles**

Create `apps/web/src/components/Detector.astro`:

```astro
---
import { LANGUAGES } from "../lib/languages";
import { SAMPLES } from "../lib/samples";
import Numeral from "./Numeral.astro";
import Spread from "./Spread.astro";
---
<Spread id="detector" label="the detector" class="ch">
  <div class="page l">
    <Numeral value="I" />
    <p class="rubric">chapter one</p>
    <h2 id="detector-title" class="title">Speak and be judged</h2>
    <p class="copy">Type in the panel. The module wakes on your first keystroke. After that every check runs inside the page and times itself. Leave the tongue on <b>AUTO</b> and the router picks one of fifteen, or choose it yourself. Nothing is sent anywhere.</p>
    <p class="rubric mute">samples from the native smoke report</p>
    <ul class="samples">
      {SAMPLES.map((sample) => (
        <li>
          <button type="button" data-sample data-code={sample.code} data-text={sample.text} title={sample.text}>
            <span class:list={["kind", sample.kind]}>{sample.kind}</span>
            <span class="name">{sample.name}</span>
            <span class="text" lang={sample.tag} dir={sample.direction}>{sample.text}</span>
          </button>
        </li>
      ))}
    </ul>
  </div>
  <div class="page r">
    <div class="detector" id="playground" data-state="idle">
      <div class="head">
        <label class="tongue">
          <span class="rubric mute">tongue</span>
          <select id="language">
            <option value="AUTO" selected>AUTO · detect the language</option>
            {LANGUAGES.map((language) => (
              <option value={language.code}>{language.code} · {language.name}{language.code === "MS" ? " (alias ID)" : ""}</option>
            ))}
          </select>
        </label>
      </div>
      <div class="pane">
        <textarea id="message" rows="4" spellcheck="false" aria-label="Message to check" placeholder="Write the message you are about to send"></textarea>
      </div>
      <p class="status" id="status" role="status" aria-live="polite"></p>
      <div class="verdict" id="verdict" hidden>
        <div class="ruling">
          <strong class="word">
            <svg class="ring thorns" viewBox="0 0 200 200" aria-hidden="true"><use href="#thornRing"></use></svg>
            <svg class="ring halo" viewBox="0 0 200 200" aria-hidden="true"><circle cx="100" cy="100" r="66" stroke-width="1"></circle></svg>
            <span id="ruling"></span>
          </strong>
          <span class="clock" id="clock"></span>
        </div>
        <p class="note" id="note"></p>
        <div class="meter" id="meter" aria-hidden="true"><i id="bar"></i></div>
        <dl class="fields">
          <div><dt>ok</dt><dd id="f-ok"></dd></div>
          <div><dt>score / threshold</dt><dd id="f-score"></dd></div>
          <div><dt>shouldNudge</dt><dd id="f-nudge"></dd></div>
          <div><dt>resolvedLanguage</dt><dd id="f-lang"></dd></div>
          <div><dt>evaluated</dt><dd id="f-evaluated"></dd></div>
          <div><dt>languageScore</dt><dd id="f-langscore"></dd></div>
        </dl>
      </div>
      <div class="failure" id="failure" hidden>
        <p id="failure-message"></p>
        <button type="button" class="act ghost" id="retry">Try again</button>
      </div>
    </div>
  </div>
</Spread>

<script>
  import { mountPlayground } from "../scripts/playground";

  const root = document.getElementById("playground");
  if (root) mountPlayground(root);
</script>

<style>
  .samples {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    max-height: 420px;
    overflow-y: auto;
    border-top: 1px solid var(--hair);
    scrollbar-width: thin;
    scrollbar-color: var(--blood-hair) transparent;
  }
  .samples li {
    border-bottom: 1px solid var(--hair);
  }
  .samples button {
    display: grid;
    grid-template-columns: 62px 96px 1fr;
    gap: 12px;
    align-items: baseline;
    width: 100%;
    min-height: 44px;
    padding: 9px 4px;
    background: none;
    border: 0;
    text-align: left;
    cursor: pointer;
    color: var(--taupe-ink);
    transition: color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
  }
  .samples button:hover {
    color: var(--bone);
    background: rgba(236, 231, 222, 0.03);
  }
  .kind {
    font-family: var(--font-rubric);
    font-size: 10px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  .kind.toxic {
    color: var(--blood-ink);
  }
  .kind.clean {
    color: var(--ochre);
  }
  .name {
    font-family: var(--font-goth);
    font-size: 16px;
  }
  .text {
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ch .page.r {
    padding-top: 136px;
  }
  .detector {
    border: 1px solid var(--hair);
    background: linear-gradient(180deg, var(--coal), #080707);
    transition: border-color var(--dur-slow) var(--ease-out);
  }
  .detector[data-state="loading"] {
    border-color: var(--ochre);
  }
  .detector[data-state="error"],
  .detector[data-state="unavailable"] {
    border-color: var(--blood-hair);
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 18px;
    border-bottom: 1px solid var(--hair);
  }
  .tongue {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .tongue .rubric {
    margin: 0;
  }
  select {
    background: var(--pitch);
    border: 1px solid var(--hair);
    color: var(--bone);
    font-family: var(--font-mono);
    font-size: 13px;
    padding: 8px 10px;
    min-height: 40px;
  }
  .pane {
    padding: 18px;
  }
  textarea {
    width: 100%;
    min-height: 112px;
    resize: vertical;
    background: transparent;
    border: none;
    outline: none;
    color: var(--bone);
    font-family: var(--font-body);
    font-size: 19px;
    line-height: 1.5;
  }
  textarea::placeholder {
    color: var(--taupe);
  }
  textarea:disabled {
    color: var(--taupe);
  }
  .status {
    margin: 0;
    padding: 0 18px 14px;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
    color: var(--taupe-ink);
  }
  .detector[data-state="loading"] .status {
    color: var(--ochre);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .verdict {
    position: relative;
    isolation: isolate;
    border-top: 1px solid var(--hair);
    padding: 18px 18px 20px;
    transition: background var(--dur-slow) var(--ease-out), border-color var(--dur-slow) var(--ease-out);
  }
  .verdict[data-tone="hit"] {
    background: linear-gradient(180deg, rgba(74, 20, 24, 0.55), rgba(74, 20, 24, 0));
    border-top-color: var(--blood-hair);
  }
  .ruling {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .word {
    position: relative;
    display: inline-block;
    font-family: var(--font-goth);
    font-weight: 400;
    font-size: clamp(44px, 7vw, 96px);
    line-height: 0.9;
    transition: color var(--dur-fast) var(--ease-out), text-shadow var(--dur-fast) var(--ease-out);
  }
  .verdict[data-tone="hit"] .word {
    color: var(--blood);
    text-shadow: 0 0 30px rgba(226, 49, 39, 0.45);
  }
  .verdict[data-tone="clean"] .word {
    color: var(--bone);
  }
  .verdict[data-tone="unknown"] .word {
    color: var(--taupe-ink);
  }
  .ring {
    position: absolute;
    left: -0.32em;
    top: 50%;
    transform: translateY(-50%);
    width: 1.7em;
    height: 1.7em;
    z-index: -1;
    fill: none;
    opacity: 0;
    transition: opacity var(--dur-slow) var(--ease-out);
  }
  .ring.thorns {
    stroke: var(--blood);
  }
  .ring.halo {
    stroke: var(--ochre);
  }
  .verdict[data-tone="hit"] .ring.thorns,
  .verdict[data-tone="clean"] .ring.halo {
    opacity: 0.9;
  }
  .clock {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--taupe-ink);
    white-space: nowrap;
  }
  .note {
    margin: 8px 0 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--taupe-ink);
  }
  .meter {
    position: relative;
    height: 2px;
    background: rgba(236, 231, 222, 0.12);
    margin: 16px 0 14px;
  }
  .meter i {
    display: block;
    height: 100%;
    width: 0;
    background: var(--blood);
    box-shadow: 0 0 10px rgba(226, 49, 39, 0.8);
    transition: width var(--dur-slow) var(--ease-out);
  }
  .meter::after {
    content: "";
    position: absolute;
    top: -5px;
    left: var(--threshold, 50%);
    width: 1px;
    height: 12px;
    background: var(--bone);
    opacity: 0.6;
  }
  .fields {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px 14px;
    margin: 0;
  }
  .fields div {
    border: 1px solid var(--hair);
    padding: 8px 10px;
  }
  .fields dt {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--taupe-ink);
  }
  .fields dd {
    margin: 2px 0 0;
    font-family: var(--font-mono);
    font-size: 14px;
    color: var(--bone);
  }
  .failure {
    border-top: 1px solid var(--blood-hair);
    padding: 16px 18px 18px;
  }
  .failure p {
    margin: 0 0 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--blood-ink);
    word-break: break-word;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.55; }
  }
  @media (max-width: 1023px) {
    .ch .page.r {
      padding-top: 0;
    }
    .samples button {
      grid-template-columns: 56px 1fr;
    }
    .text {
      grid-column: 1 / -1;
    }
    .fields {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
```

- [ ] **Step 4: Mount the chapter**

In `apps/web/src/pages/index.astro`, import `Detector` and place `<Detector />` after `<Frontispiece />`:

```astro
---
import Codex from "../layouts/Codex.astro";
import Detector from "../components/Detector.astro";
import Frontispiece from "../components/Frontispiece.astro";

const title = "blasphem · a pre-send nudge for hostile messages";
const description = "Deterministic multilingual toxicity detection compiled to WebAssembly. Fifteen languages, ordinal scores, no request after the module loads.";
---
<Codex title={title} description={description}>
  <Frontispiece />
  <Detector />
</Codex>
```

- [ ] **Step 5: Verify the pure module and the built page**

Run: `node -e "import('./apps/web/src/scripts/playground-state.ts').then((m) => { const loading = m.transition({ status: 'idle' }, { type: 'LOAD' }); console.log(loading.status, m.transition(loading, { type: 'LOADED' }).status, m.transition(loading, { type: 'RETRY' }).status, m.verdictFor({ evaluated: false, shouldNudge: false }).word, m.verdictFor({ evaluated: true, shouldNudge: true }).word); })"`
Expected: `loading ready loading Unheard Condemned` (Node 24 strips the types natively).

Run: `pnpm --filter web build 2>&1 | tail -3`
Expected: `Complete!`

Run: `grep -c "blasphem_bg.wasm" apps/web/dist/index.html; grep -c "modulepreload" apps/web/dist/index.html; grep -o 'data-sample' apps/web/dist/index.html | wc -l`
Expected: `0`, `0`, `30` (two supplied samples for each of 15 languages).

Run: `grep -l "blasphem_bg.wasm" apps/web/dist/_astro/*.js | wc -l`
Expected: `1` (only the playground chunk knows the file name, inside a string).

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 6: Commit**

```bash
git add apps/web
git commit -m "Add the detector playground with lazy WASM loading"
```

---

### Task 9: Chapter II, the rite (installation and browser API)

**Files:**
- Create: `apps/web/src/components/Rite.astro`
- Modify: `apps/web/src/pages/index.astro`

**Interfaces:**
- Consumes: `smoke` (Task 5) for the worked example values; `LANGUAGES`.
- Produces: `<Rite />` rendering `<section id="rite">` with `<h2 id="rite-title">`.

The worked example uses the Spanish supplied toxic case from the native smoke report, so its `ok`, `score`, `threshold`, and `shouldNudge` values are report data, not typed numbers. The explicit `"ES"` selection makes `resolvedLanguage` deterministic.

- [ ] **Step 1: Write the chapter**

Create `apps/web/src/components/Rite.astro`:

```astro
---
import { LANGUAGES } from "../lib/languages";
import { smoke } from "../lib/reports";
import Numeral from "./Numeral.astro";
import Spread from "./Spread.astro";

const example = smoke.languages.ES.cases.find((entry) => entry.case_id === "supplied-es-toxic");
if (!example) throw new Error("the smoke report lacks supplied-es-toxic");
const codes = LANGUAGES.map((language) => language.code).join(", ");

const fields = [
  ["ok", "boolean", "True when no nudge is due. Also true for an unknown automatic route."],
  ["score", "integer 0 to 100", "Ordinal. Higher means more evidence. Not a probability, not calibrated across languages."],
  ["threshold", "integer", "The boundary the score is compared against."],
  ["shouldNudge", "boolean", "True when the pre-send nudge should show. Equals score at or above threshold."],
  ["evaluated", "boolean", "False when AUTO found no reliable supported language. No toxicity check ran."],
  ["resolvedLanguage", "string", "One of the 15 codes, or \"unknown\"."],
  ["languageReliable", "boolean", "True on explicit routes and on confident automatic routes."],
  ["languageScore", "number or undefined", "Present only on automatic routes."],
] as const;
---
<Spread id="rite" label="the rite" class="ch rite">
  <div class="page l">
    <pre><code><span class="c">// ESM glue plus one .wasm file. No worker, no fetch after init.</span>
<span class="k">import</span> init, &#123; BlasphemDetector &#125; <span class="k">from</span> <span class="s">"blasphem"</span>;

<span class="k">await</span> init();
<span class="k">const</span> detector = <span class="k">new</span> BlasphemDetector(<span class="s">"ES"</span>);
<span class="k">const</span> result = detector.check(<span class="s">{JSON.stringify(example.text)}</span>);

result.ok               <span class="c">// {String(example.ok)}</span>
result.score            <span class="c">// {example.score} of {example.threshold}, ordinal</span>
result.shouldNudge      <span class="c">// {String(example.should_nudge)}</span>
result.resolvedLanguage <span class="c">// "ES"</span>

result.free();
detector.free();</code></pre>
    <p class="evidence">values from {smoke.evidence_status.replaceAll("_", " ")}</p>

    <h3 class="sub">Install</h3>
    <p class="copy">The package is private and unpublished. Build it from a clone of <a href="https://github.com/sospedra/blasphem">sospedra/blasphem</a>:</p>
    <pre><code>pnpm install --frozen-lockfile
pnpm --filter blasphem run build
pnpm --filter blasphem pack</code></pre>
    <p class="copy">Then <code>pnpm add ./blasphem-0.1.0.tgz</code> in your app, or reference the package as <code>workspace:*</code> inside this repository. Serve <code>blasphem_bg.wasm</code> as <code>application/wasm</code>. When it does not sit next to the glue, pass its URL: <code>init(&#123; module_or_path: url &#125;)</code>.</p>

    <h3 class="sub">Tongues</h3>
    <p class="copy"><code>{codes}</code>, or <code>AUTO</code>. <code>ID</code> is accepted as an alias for <code>MS</code>. Any other value throws in the constructor.</p>
  </div>
  <div class="page r mirror">
    <Numeral value="II" class="two" />
    <p class="rubric">chapter two</p>
    <h2 id="rite-title" class="title">One module, two calls</h2>
    <p class="copy"><code>init()</code> once. Then <code>check()</code> as often as the keystrokes come. The same bytes judge the same text the same way in every browser.</p>
    <p class="copy">Under <b>AUTO</b> the router identifies the language before the toxicity check. Unreliable input returns an unknown route: <code>ok</code> stays true, <code>evaluated</code> is false, nothing is judged. The nudge fails open by design.</p>
    <p class="copy">Free what you create. Results and detectors hold WebAssembly memory until <code>free()</code>.</p>
    <table class="ledger fields">
      <caption>BlasphemResult</caption>
      <thead><tr><th>field</th><th>type</th><th>meaning</th></tr></thead>
      <tbody>
        {fields.map(([name, type, meaning]) => (
          <tr><td>{name}</td><td class="us">{type}</td><td class="meaning">{meaning}</td></tr>
        ))}
      </tbody>
    </table>
  </div>
  <div class="spine" aria-hidden="true">one module &middot; deterministic &middot; offline after init</div>
</Spread>

<style>
  pre {
    margin: 0 0 18px;
    padding: 18px;
    overflow-x: auto;
    border: 1px solid var(--hair);
    background: #080707;
    font-family: var(--font-mono);
    font-size: 12.5px;
    line-height: 1.85;
    color: #c3bcb4;
    tab-size: 2;
  }
  pre .k {
    color: var(--blood-ink);
  }
  pre .s {
    color: #e0a49e;
  }
  pre .c {
    color: #7d746d;
    font-style: italic;
  }
  .sub {
    font-family: var(--font-goth);
    font-weight: 400;
    font-size: 26px;
    margin: 34px 0 8px;
    color: var(--bone);
  }
  .rite {
    min-height: 640px;
  }
  .ch .page.r {
    padding-top: 136px;
  }
  .fields th,
  .fields td {
    text-align: left;
    vertical-align: top;
  }
  .fields .meaning {
    font-size: 14px;
    color: var(--parchment);
  }
  .fields caption {
    text-align: right;
  }
  .page.r.mirror .fields {
    text-align: left;
  }
  .spine {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%) rotate(180deg);
    writing-mode: vertical-rl;
    font-family: var(--font-rubric);
    font-size: 15px;
    letter-spacing: 0.3em;
    text-transform: uppercase;
    color: var(--bone);
    background: var(--pitch);
    padding: 14px 0;
    z-index: 3;
    white-space: nowrap;
  }
  @media (max-width: 1023px) {
    .rite {
      min-height: 0;
    }
    .ch .page.r {
      padding-top: 0;
    }
    .spine {
      display: none;
    }
    pre {
      font-size: 11.5px;
    }
  }
</style>
```

- [ ] **Step 2: Mount the chapter**

In `apps/web/src/pages/index.astro`, import `Rite` and place `<Rite />` after `<Detector />`.

- [ ] **Step 3: Verify**

Run: `pnpm --filter web build 2>&1 | tail -2`
Expected: `Complete!`

Run:

```bash
python3 - <<'PY'
import json, html
s = json.load(open("reports/multilingual-cli-smoke.json"))
case = next(c for c in s["languages"]["ES"]["cases"] if c["case_id"] == "supplied-es-toxic")
page = open("apps/web/dist/index.html").read()
print("example text:", html.escape(json.dumps(case["text"], ensure_ascii=False), quote=False) in page or json.dumps(case["text"], ensure_ascii=False) in page)
print("example score:", f"// {case['score']} of {case['threshold']}, ordinal" in page)
print("class count:", page.count("BlasphemDetector") >= 2)
print("alias:", "alias for" in page)
PY
```

Expected: four `True` lines.

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 4: Commit**

```bash
git add apps/web
git commit -m "Add the rite chapter with report-backed API usage"
```

---

### Task 10: Chapter III, the vows, and Chapter IV, the reckoning

**Files:**
- Create: `apps/web/src/components/Ledger.astro`
- Create: `apps/web/src/components/Vows.astro`
- Create: `apps/web/src/components/Reckoning.astro`
- Modify: `apps/web/src/pages/index.astro`

**Interfaces:**
- Consumes: every export of `reports.ts` and `metrics.ts`; `LANGUAGES`, `storageCode`; formatters.
- Produces: `<Ledger caption columns rows evidence />`; `<Vows />` (`<section id="vows">`, `<h2 id="vows-title">`); `<Reckoning />` (`<section id="reckoning">`, `<h2 id="reckoning-title">`).

- [ ] **Step 1: Write the generic ledger**

Create `apps/web/src/components/Ledger.astro`:

```astro
---
interface Props {
  caption: string;
  columns: readonly string[];
  rows: readonly (readonly (string | number)[])[];
  evidence?: string;
}
const { caption, columns, rows, evidence } = Astro.props;
---
<div class="ledger-wrap">
  <table class="ledger">
    <caption>{caption}</caption>
    <thead>
      <tr>{columns.map((column, index) => <th class:list={{ mine: index > 0 }} scope="col">{column}</th>)}</tr>
    </thead>
    <tbody>
      {rows.map((row) => (
        <tr>{row.map((cell, index) => (index === 0 ? <th scope="row">{cell}</th> : <td class="us">{cell}</td>))}</tr>
      ))}
    </tbody>
  </table>
  {evidence && <p class="evidence">{evidence}</p>}
</div>

<style>
  .ledger-wrap {
    margin: 0 0 34px;
    overflow-x: auto;
  }
  tbody th {
    font-family: var(--font-rubric);
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--taupe-ink);
    font-weight: 400;
    text-align: left;
    padding: 14px 8px;
    border-bottom: 1px solid var(--hair);
  }
</style>
```

- [ ] **Step 2: Write the vows**

Create `apps/web/src/components/Vows.astro`:

```astro
---
import { formatKibibytes } from "../lib/format";
import { browser, sizes, smoke } from "../lib/reports";
import Numeral from "./Numeral.astro";
import Spread from "./Spread.astro";

const thresholds = [...new Set(Object.values(smoke.languages).flatMap((language) => language.cases.map((entry) => entry.threshold)))];
const tableBytes = Math.max(...Object.values(sizes.artifacts).map((artifact) => artifact.bytes));

const vows = [
  {
    numeral: "I",
    title: "Nothing leaves the page",
    body: `After init() the module makes no request. The Chromium run recorded ${browser.runtime_network_requests.length} runtime network requests across ${browser.supplied_case_count + browser.auto_case_count + browser.unknown_case_count} checks.`,
    glyph: "star",
  },
  {
    numeral: "II",
    title: "Ordinal, not probability",
    body: `The score is an integer from 0 through 100 compared against one boundary, ${thresholds.join(" or ")}. It ranks evidence. It does not estimate a chance.`,
    glyph: "scale",
  },
  {
    numeral: "III",
    title: "Unknown fails open",
    body: `When AUTO cannot route a message, nothing is judged: ok stays true and evaluated is false. ${browser.passed_unknown_case_count} of ${browser.unknown_case_count} unknown-route contracts passed in the browser.`,
    glyph: "eye",
  },
  {
    numeral: "IV",
    title: "Fifteen tongues, one table each",
    body: `Every language carries its own sparse integer table of ${formatKibibytes(tableBytes)}, trained offline. No table reads another.`,
    glyph: "key",
  },
] as const;
---
<Spread id="vows" label="the vows" class="vows">
  <Numeral value="III" class="three" />
  <h2 id="vows-title" class="visually-hidden">The vows</h2>
  <div class="arches">
    {vows.map((vow) => (
      <figure class="arch">
        <div class="plate">
          <svg preserveAspectRatio="none" aria-hidden="true"><use href="#archFrame"></use></svg>
          <span class="num">{vow.numeral}</span>
          <svg class="glyph" viewBox="0 0 160 220" aria-hidden="true">
            {vow.glyph === "star" && <g transform="translate(62,88) scale(1.5)" fill="#e23127"><use href="#star"></use></g>}
            {vow.glyph === "scale" && <g fill="none" stroke="#e23127" stroke-width="2"><path d="M80 70 V150 M50 150 H110 M44 96 H116 M44 96 L34 124 H54 Z M116 96 L106 124 H126 Z"></path></g>}
            {vow.glyph === "eye" && <g stroke="#e23127" fill="none" stroke-width="2"><path d="M46 110 Q80 84 114 110 Q80 136 46 110 Z"></path><circle cx="80" cy="110" r="12" fill="#e23127" stroke="none"></circle></g>}
            {vow.glyph === "key" && <g stroke="#e23127" fill="none" stroke-width="2"><path d="M62 132 L98 96"></path><circle cx="58" cy="136" r="9"></circle><path d="M92 90 L104 102 M84 98 L94 108"></path></g>}
          </svg>
        </div>
        <figcaption>
          <h3>{vow.title}</h3>
          <p>{vow.body}</p>
        </figcaption>
      </figure>
    ))}
  </div>
</Spread>

<style>
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .vows :global(.numeral.three) {
    position: absolute;
    left: 50%;
    top: 20px;
    margin: 0;
    transform: translateX(-50%);
    z-index: 0;
  }
  .arches {
    grid-column: 2 / 4;
    position: relative;
    z-index: 2;
    padding: 96px var(--inset) 120px;
    display: grid;
    grid-template-columns: 1fr 1fr 72px 1fr 1fr;
    gap: 14px;
  }
  .arch {
    margin: 0;
  }
  .arch:nth-child(3) {
    grid-column: 4;
  }
  .arch:nth-child(4) {
    grid-column: 5;
  }
  .plate {
    position: relative;
    aspect-ratio: 3 / 4;
  }
  .plate svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .num {
    position: absolute;
    top: 14%;
    left: 0;
    right: 0;
    text-align: center;
    font-family: var(--font-rubric);
    font-size: 13px;
    letter-spacing: 0.2em;
    color: var(--blood-ink);
  }
  h3 {
    margin: 12px 0 3px;
    font-family: var(--font-goth);
    font-weight: 400;
    font-size: 20px;
    line-height: 1.05;
    color: var(--bone);
  }
  p {
    margin: 0;
    color: var(--taupe-ink);
    font-size: 14px;
    line-height: 1.5;
  }
  @media (max-width: 1023px) {
    .vows :global(.numeral.three) {
      top: -8px;
      font-size: 220px;
    }
    .arches {
      grid-column: 1;
      grid-template-columns: 1fr 1fr;
      padding: 150px var(--inset) 72px;
      gap: 20px 14px;
    }
    .arch:nth-child(3) {
      grid-column: 1;
    }
    .arch:nth-child(4) {
      grid-column: 2;
    }
    .num {
      font-size: 11px;
    }
  }
</style>
```

- [ ] **Step 3: Write the reckoning**

Create `apps/web/src/components/Reckoning.astro`:

```astro
---
import { formatBytes, formatInt, formatMegabytes, formatMs, formatPercent } from "../lib/format";
import { LANGUAGES, storageCode } from "../lib/languages";
import { caseTotals, fixtureCount, medianP95Ms, nanosecondsToMs, routingTotals, worstP95Ms } from "../lib/metrics";
import { behavior, browser, performance, routing, sizes, smoke, validation } from "../lib/reports";
import Ledger from "./Ledger.astro";
import Numeral from "./Numeral.astro";
import Spread from "./Spread.astro";

const label = (status: string) => status.replaceAll("_", " ");

const validationRows = LANGUAGES.map((language) => {
  const entry = validation.languages[language.code];
  if (!entry) return [language.code, "not in this report", "", "", ""];
  const rows = Object.values(entry.matrix).reduce((sum, count) => sum + count, 0);
  return [
    language.code,
    formatInt(rows),
    formatPercent(entry.metrics.precision),
    formatPercent(entry.metrics.recall),
    formatPercent(entry.metrics.false_warning_rate, 2),
  ];
});

const latencyRows = [
  ["280 scalars, median p95", formatMs(medianP95Ms(performance.fixtures, "-280")), formatInt(fixtureCount(performance.fixtures, "-280"))],
  ["280 scalars, worst p95", formatMs(worstP95Ms(performance.fixtures, "-280")), formatInt(fixtureCount(performance.fixtures, "-280"))],
  ["4096 bytes, median p95", formatMs(medianP95Ms(performance.fixtures, "-4096")), formatInt(fixtureCount(performance.fixtures, "-4096"))],
  ["4096 bytes, worst p95", formatMs(worstP95Ms(performance.fixtures, "-4096")), formatInt(fixtureCount(performance.fixtures, "-4096"))],
  ["AUTO, 280 scalars, p95", formatMs(nanosecondsToMs(routing.timing.groups.unicode_scalars_280.p95_nanoseconds)), formatInt(routing.timing.groups.unicode_scalars_280.samples)],
  ["AUTO, 4096 bytes, p95", formatMs(nanosecondsToMs(routing.timing.groups.utf8_bytes_4096.p95_nanoseconds)), formatInt(routing.timing.groups.utf8_bytes_4096.samples)],
  ["router cold start", formatMs(nanosecondsToMs(routing.cold_initialization_nanoseconds)), "1"],
];

const builds = browser.browser_builds;
const transferRows = [
  ["default, raw", formatBytes(builds.full.raw_total_bytes), formatMegabytes(builds.full.raw_total_bytes)],
  ["default, gzip", formatBytes(builds.full.gzip_total_bytes), formatMegabytes(builds.full.gzip_total_bytes)],
  ["default, brotli", formatBytes(builds.full.brotli_total_bytes), formatMegabytes(builds.full.brotli_total_bytes)],
  ["explicit-only, raw", formatBytes(builds.explicit_only.raw_total_bytes), formatMegabytes(builds.explicit_only.raw_total_bytes)],
  ["explicit-only, brotli", formatBytes(builds.explicit_only.brotli_total_bytes), formatMegabytes(builds.explicit_only.brotli_total_bytes)],
  ["one sparse table", formatBytes(sizes.artifacts[storageCode("EN")].bytes), formatMegabytes(sizes.artifacts[storageCode("EN")].bytes)],
];

const totals = routingTotals(routing);
const routingRows = [
  ...LANGUAGES.map((language) => {
    const entry = routing.languages[language.code];
    return [language.code, formatInt(entry.rows), formatPercent(entry.route_accuracy.value, 2), formatPercent(entry.unknown_rate.value, 2), formatPercent(entry.misroute_rate.value, 3)];
  }),
  ["all supported", formatInt(totals.rows), formatPercent(routing.supported.route_accuracy.value, 2), formatPercent(totals.unknownRate, 2), formatPercent(totals.misrouteRate, 3)],
];

const behaviorTotals = caseTotals(behavior);
const smokeTotals = caseTotals(smoke);
const contractRows = [
  ["behavior panels", `${formatInt(behaviorTotals.passed)} / ${formatInt(behaviorTotals.total)}`, label(behavior.evidence_status)],
  ["native smoke", `${formatInt(smokeTotals.passed)} / ${formatInt(smokeTotals.total)}`, label(smoke.evidence_status)],
  ["browser explicit", `${formatInt(browser.passed_case_count)} / ${formatInt(browser.supplied_case_count)}`, `${browser.browser_engine} ${browser.browser_version}`],
  ["browser AUTO", `${formatInt(browser.passed_auto_case_count)} / ${formatInt(browser.auto_case_count)}`, `${browser.browser_engine} ${browser.browser_version}`],
  ["browser unknown", `${formatInt(browser.passed_unknown_case_count)} / ${formatInt(browser.unknown_case_count)}`, `${browser.browser_engine} ${browser.browser_version}`],
  ["routing parity", `${formatInt(routing.supported.correct)} / ${formatInt(routing.supported.known_route_precision.denominator)}`, label(routing.evidence_status)],
];
---
<Spread id="reckoning" label="the reckoning" class="ch">
  <div class="page l">
    <Numeral value="IV" class="four" />
    <p class="rubric">chapter four</p>
    <h2 id="reckoning-title" class="title">Weighed on committed evidence</h2>
    <p class="copy">Every figure on this page is read from the JSON reports in the repository at build time. Nothing here is typed by hand. Each ledger names the evidence status of its source.</p>
    <p class="copy"><b>The score is ordinal.</b> It is not a probability, and it is not calibrated across languages. The validation table is calibration evidence: those rows selected the boundaries, so they overstate live precision when toxic messages are rare. The clean-warned column is the clearer nuisance measure.</p>
    <p class="copy">Limits stated by the reports: {routing.limitations.join(" ")} The detector targets native scripts and standard Latin spellings. It has no dedicated Pinyin, Arabizi, or Hinglish model. Source corpora use different toxicity definitions and do not represent live traffic. Chinese and Turkish recall is low by the precision-first policy.</p>
    <p class="copy">Machine: {performance.computer}, {performance.target_triple}, {performance.rust_version.split(" ").slice(0, 2).join(" ")}. Peak resident memory {formatMegabytes(performance.peak_rss_bytes, 1)}.</p>
  </div>
  <div class="page r">
    <Ledger caption="Validation split" columns={["tongue", "rows", "precision", "recall", "clean warned"]} rows={validationRows} evidence={`${label(validation.evidence_status)} · split ${validation.split}`} />
    <Ledger caption="Latency, release build" columns={["input", "p95", "fixtures"]} rows={latencyRows} evidence={`${label(performance.evidence_status)} · all latency gates ${performance.all_latency_gates_passed ? "passed" : "failed"}`} />
    <Ledger caption="Transfer, WASM plus glue" columns={["build", "bytes", "size"]} rows={transferRows} evidence={`${label(browser.evidence_status)} · wasm-bindgen ${browser.wasm_bindgen_version} · ${browser.browser_engine} ${browser.browser_version}`} />
    <Ledger caption="Language routing" columns={["tongue", "rows", "correct", "unknown", "misrouted"]} rows={routingRows} evidence={`${label(routing.evidence_status)} · ${formatInt(routing.corpus.rows)} Tatoeba sentences · unsupported rejected ${formatPercent(routing.unsupported.unsupported_rejection_rate.value, 1)}`} />
    <Ledger caption="Contract evidence" columns={["suite", "passed", "source"]} rows={contractRows} />
  </div>
</Spread>

<style>
  .ch .page.r {
    padding-top: 136px;
  }
  @media (max-width: 1023px) {
    .ch .page.r {
      padding-top: 0;
    }
  }
</style>
```

- [ ] **Step 4: Mount both chapters**

In `apps/web/src/pages/index.astro`, import `Vows` and `Reckoning` and place `<Vows />` then `<Reckoning />` after `<Rite />`.

- [ ] **Step 5: Verify**

Run: `pnpm --filter web build 2>&1 | tail -2`
Expected: `Complete!`

Run:

```bash
python3 - <<'PY'
import json, statistics
page = open("apps/web/dist/index.html").read()
v = json.load(open("reports/multilingual-validation.json"))["languages"]["EN"]["metrics"]
p = json.load(open("reports/multilingual-performance.json"))["fixtures"]
w = json.load(open("reports/multilingual-wasm.json"))["browser_builds"]["full"]
r = [json.load(open(f"reports/{n}")) for n in __import__("os").listdir("reports") if n.endswith(".json")]
routing = next(x for x in r if "c_parity" in x)
p95 = statistics.median(f["p95_nanoseconds"] for k, f in p.items() if k.endswith("-280")) / 1e6
print("EN precision:", f"{v['precision']*100:.1f}%" in page)
print("median p95:", f"{p95:.2f} ms" in page)
print("brotli bytes:", f"{w['brotli_total_bytes']:,} B" in page)
print("route precision:", f"{routing['supported']['route_accuracy']['value']*100:.2f}%" in page)
print("ordinal statement:", "The score is ordinal." in page)
print("evidence labels:", page.count("calibration evidence") >= 1 and page.count("behavior contract evidence") >= 1)
PY
```

Expected: six `True` lines.

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 6: Commit**

```bash
git add apps/web
git commit -m "Add the vows and the reckoning from committed reports"
```

---

### Task 11: The colophon (contribute, links, credits)

**Files:**
- Create: `apps/web/src/components/Colophon.astro`
- Modify: `apps/web/src/pages/index.astro`

**Interfaces:**
- Produces: `<Colophon />` rendering `<footer id="colophon">` with `<h2 id="colophon-title">`; links to `https://github.com/sospedra/blasphem`, its `CONTRIBUTING.md`, `LICENSE`, and the design spec.

- [ ] **Step 1: Write the colophon**

Create `apps/web/src/components/Colophon.astro`:

```astro
---
const repository = "https://github.com/sospedra/blasphem";
const links = [
  { href: repository, label: "github" },
  { href: `${repository}/blob/main/CONTRIBUTING.md`, label: "contributing" },
  { href: `${repository}/blob/main/LICENSE`, label: "apache-2.0" },
  { href: `${repository}/blob/main/docs/superpowers/specs/2026-09-02-blasphem-public-package-and-corpus-design.md`, label: "design spec" },
];
const bars = [0, 4, 8, 14, 18, 23, 30, 34, 39, 43, 50, 54, 59, 66, 70, 75, 79, 85, 90, 94, 101, 105, 110, 116, 120, 125, 129, 136, 140, 145];
const widths = [2, 1, 3, 1, 2, 4, 1, 2, 1, 3, 1, 2, 4, 1, 2, 1, 3, 2, 1, 4, 1, 2, 3, 1, 2, 1, 4, 1, 2, 3];
---
<footer id="colophon" class="spread colophon" aria-labelledby="colophon-title">
  <div class="rail" aria-hidden="true"><span>the colophon</span></div>
  <div class="page l">
    <p class="rubric">contribute</p>
    <h2 id="colophon-title" class="title">Add a tongue, a row, a rule</h2>
    <h3 class="sub">Corpus rows</h3>
    <p class="copy">A contribution is a TSV with three columns: <code>native_id</code>, <code>label</code>, <code>text</code>. The label is <code>toxic</code> or <code>clean</code>. Each source declares one role: <code>baseline</code>, <code>training_only</code>, or <code>sealed_evaluation</code>. New community corpora default to <code>training_only</code> and enter only the development partition. Sealed validation and test rows never move; the preparation command rejects a change to any sealed hash.</p>
    <p class="copy">Record the language, the source identity, the license, and a citation. Pull request checks read only committed raw inputs and pinned dependencies. They fetch no contributor URL.</p>
    <h3 class="sub">Code</h3>
    <p class="copy">Rust lives in the Cargo workspace. JavaScript lives in the pnpm workspace under <code>apps/web</code> and <code>packages/blasphem</code>. The data-offline reproduction command rebuilds every artifact from the committed inputs and stops on any mismatch: <code>cargo run --release --locked -p blasphem-train -- reproduce</code>.</p>
    <p class="copy">Any row that shapes a rule becomes audit-only and leaves the quality evidence. The <a href={`${repository}/blob/main/CONTRIBUTING.md`}>contribution guide</a> describes both paths.</p>
  </div>
  <div class="page r">
    <nav class="links" aria-label="Repository links">
      {links.map((link) => <a href={link.href}>{link.label}</a>)}
    </nav>
    <svg class="barcode" width="140" height="22" viewBox="0 0 150 26" aria-hidden="true">
      <g fill="#ece7de">{bars.map((x, index) => <rect x={x} width={widths[index]} height="26"></rect>)}</g>
    </svg>
    <p class="credits">set in pirata one, cinzel, eb garamond, ibm plex mono, archivo. self-hosted.</p>
    <p class="credits">plate: marble portrait bust of a woman with a scroll, the metropolitan museum of art, cc0.</p>
    <p class="credits">hurtlex lexica cc by-sa 4.0. language tables apache-2.0. notices ship in the package.</p>
    <p class="credits">apache licensed. experimental. scores are ordinal.</p>
  </div>
  <div class="skyline" aria-hidden="true">
    <svg viewBox="0 0 900 90">
      <path d="M0 82 L130 82 L150 46 L160 22 L170 46 L190 82 L250 82 L300 50 L316 18 L316 2 L316 8 L310 8 L322 8 L316 8 L316 18 L332 50 L352 82 L410 82 L410 56 L450 30 L450 44 A14 14 0 1 0 450 72 A14 14 0 1 0 450 44 L450 30 L490 56 L490 82 L560 82 L580 40 L590 14 L600 40 L620 82 L900 82"></path>
    </svg>
  </div>
  <div class="motto-wrap" aria-hidden="true"><p class="motto">nihil profanum</p></div>
</footer>

<style>
  .colophon {
    border-bottom: 0;
  }
  .colophon .page {
    padding-top: 96px;
    padding-bottom: 40px;
  }
  .sub {
    font-family: var(--font-goth);
    font-weight: 400;
    font-size: 24px;
    margin: 26px 0 8px;
    color: var(--bone);
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 22px;
    font-family: var(--font-rubric);
    font-size: 11px;
    letter-spacing: 0.22em;
    text-transform: uppercase;
  }
  .links a {
    color: var(--taupe-ink);
    text-decoration: none;
    border-bottom: 1px solid var(--hair);
    padding: 6px 0;
    transition: color var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out);
  }
  .links a:hover {
    color: var(--blood-ink);
    border-color: var(--blood-ink);
  }
  .barcode {
    display: block;
    margin-top: 22px;
    opacity: 0.45;
  }
  .credits {
    margin: 14px 0 0;
    font-family: var(--font-rubric);
    font-size: 11px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--taupe-ink);
    line-height: 2;
  }
  .skyline {
    grid-column: 2 / 4;
    padding: 0 var(--inset);
    position: relative;
    z-index: 2;
  }
  .skyline svg {
    display: block;
    width: 100%;
    height: auto;
    stroke: var(--bone);
    stroke-opacity: 0.75;
    fill: none;
    stroke-width: 1;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .motto-wrap {
    grid-column: 1 / 5;
    font-size: 14vw;
    height: 0.865em;
    overflow: hidden;
    text-align: center;
    margin-top: 30px;
  }
  .motto {
    font-family: var(--font-goth);
    font-weight: 400;
    font-size: 1em;
    line-height: 1;
    margin: 0;
    color: var(--blood);
    white-space: nowrap;
  }
  @media (max-width: 1023px) {
    .colophon .page {
      padding: 56px var(--inset) 24px;
    }
    .skyline,
    .motto-wrap {
      grid-column: 1;
    }
    .motto-wrap {
      font-size: 17vw;
    }
  }
</style>
```

- [ ] **Step 2: Mount it**

In `apps/web/src/pages/index.astro`, import `Colophon` and place `<Colophon />` last inside `<Codex>`.

- [ ] **Step 3: Verify**

Run: `pnpm --filter web build 2>&1 | tail -2 && grep -c 'href="https://github.com/sospedra/blasphem"' apps/web/dist/index.html && grep -c "CONTRIBUTING.md" apps/web/dist/index.html && grep -c 'id="colophon"' apps/web/dist/index.html`
Expected: `Complete!`, then `1` or more, `2` or more, `1`.

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 4: Commit**

```bash
git add apps/web
git commit -m "Add the colophon with contribution paths and credits"
```

---

### Task 12: Motion and polish pass

Invoke `find-animation-opportunities` first (read-only proposals), then `emil-design-eng` for the review. Implement only motion that carries meaning; reject decoration.

**Files:**
- Modify: `apps/web/src/components/Seal.astro`, `Detector.astro`, `Frontispiece.astro`, `Vows.astro`, `apps/web/src/styles/base.css`

**Interfaces:** none new.

- [ ] **Step 1: Implement the accepted motion**

Candidates the skill is expected to surface, with the intended values:

1. Seal slow rotation: `.seal text { animation: turn 120s linear infinite; transform-origin: 95px 95px; }` with `@keyframes turn { to { transform: rotate(360deg); } }`. Continuous, ambient, stopped under reduced motion by the global rule in `base.css`.
2. Verdict entrance: `.verdict:not([hidden]) { animation: settle var(--dur-slow) var(--ease-out); }` with `@keyframes settle { from { opacity: 0; transform: translateY(6px); } }`. Communicates a new judgment.
3. Score bar: already transitions `width` over `--dur-slow`; keep.
4. Loading pulse on the status line: already present; keep.
5. Arch hover: `.arch:hover .plate svg:first-child { filter: drop-shadow(0 0 12px rgba(226,49,39,.35)); }` with a `--dur-fast` transition. Reject if the skill judges it decorative.
6. Sample button tap feedback: `.samples button:active { background: rgba(226,49,39,.08); }`.

Reject: scroll-triggered reveals, parallax on the plate, marquee speed changes.

- [ ] **Step 2: Polish checklist**

- Every interactive element has a `:focus-visible` ring from `base.css`; confirm the `select`, `textarea`, sample buttons, nav links, and `.act` links show it with keyboard Tab.
- Tap targets: sample buttons and `.act` are at least 44px tall; the select is at least 40px.
- Contrast: small text uses `--blood-ink` or `--taupe-ink`, never `--blood` or `--taupe`. Run `grep -n "color: var(--blood);" apps/web/src -r` and confirm every hit is display-size text (numerals, wordmark, verdict word, badges over 16px).
- `lang` and `dir` are set on every sample text span.
- The playground works with the keyboard alone: Tab to the select, Tab to the textarea, type.

- [ ] **Step 3: Verify**

Run: `pnpm --filter web build >/dev/null && grep -o "prefers-reduced-motion" apps/web/dist/_astro/*.css | wc -l`
Expected: `1` or more.

Run: `pnpm --filter web check`
Expected: `0 errors`.

- [ ] **Step 4: Commit**

```bash
git add apps/web
git commit -m "Tune motion, focus states, and contrast on the landing page"
```

---

### Task 13: Full verification and report

**Files:** none new. This task runs the ten verification steps from the brief, records exact output, and sends `DONE blasphem-49`.

- [ ] **Step 1: Clean install and full build**

Run from the repository root:

```bash
rm -rf node_modules apps/web/node_modules packages/blasphem/node_modules
pnpm install --frozen-lockfile
pnpm turbo run build --force 2>&1 | tail -20
```

Expected: install succeeds without lockfile changes (`git status --porcelain pnpm-lock.yaml` prints nothing); Turbo runs `blasphem#build` then `web#build`; the integration logs `copied blasphem.js and blasphem_bg.wasm to /blasphem/<hash>/`; `2 successful, 2 total`.

- [ ] **Step 2: Website and package checks**

```bash
pnpm turbo run test 2>&1 | tail -8
pnpm --filter web check
```

Expected: `blasphem#test` prints `status=packed files=7`; `astro check` prints `0 errors`.

- [ ] **Step 3: Rust workspace tests**

```bash
cargo test --locked 2>&1 | grep -E "^test result|FAILED|error" | head -40
```

Expected: every `test result:` line ends in `0 failed`. `tests/rename_contract.rs` scans `apps/` and `packages/` on disk; a failure there names a file with a retired name, which must be fixed before DONE. If a failure is in a suite this branch did not touch, record it verbatim as an inherited failure from `development`.

- [ ] **Step 4: Turbo graph**

```bash
pnpm turbo run build --dry-run | grep -E "^(blasphem#build|web#build)|Dependencies|Cached"
```

Expected: `web#build` lists `Dependencies = blasphem#build`. Record the lines.

- [ ] **Step 5: Browser checks with chrome-devtools-axi**

Invoke the `chrome-devtools-axi` skill. Serve the built site with `pnpm --filter web preview` (binds `127.0.0.1:4321`). Then, in the browser:

1. Open `http://127.0.0.1:4321/`. List network requests. Expected: no URL ends with `.wasm` and none contains `/blasphem/`.
2. Confirm the page shows `id="playground"` with `data-state="idle"`.
3. Type into `#message`. Expected: `data-state` passes through `loading` to `ready`; exactly one request for `/blasphem/<hash>/blasphem.js` and one for `/blasphem/<hash>/blasphem_bg.wasm`.
4. Click the Spanish toxic sample. Expected: `#ruling` reads `Condemned`, `#f-nudge` reads `true`, `#f-lang` reads `ES` (or the AUTO route, `ES`), `#f-score` shows an integer of the form `N / 50`.
5. Click an English clean sample. Expected: `#ruling` reads `Absolved`, `#f-ok` reads `true`.
6. Select `MS` in `#language`, type `Dia memberitahu saya yang dia benar-benar letih.`. Expected: `#f-lang` reads `MS`.
7. Open `http://127.0.0.1:4321/?lang=ID`. Expected: the select shows `MS`.
8. Run an accessibility snapshot of the page. Expected: one `h1`, `main`, `nav`, `footer` landmarks, every form control labelled.

Record each observation with the request list as evidence.

- [ ] **Step 6: Retired-name scan and final state**

```bash
grep -rnE "toxcheck|toxtrain|toxbench|eldc|ELDC" apps packages --include='*.ts' --include='*.mjs' --include='*.js' --include='*.json' --include='*.md' --include='*.html' --include='*.yml' -l | grep -v "packages/blasphem/NOTICE"; echo "scan exit=$?"
git status --porcelain | wc -l
git log --oneline development..development
```

Expected: `scan exit=1` (no files), `0` uncommitted changes, and the list of this branch's commits.

- [ ] **Step 7: Report and hand off**

Report to the user: the exact commands and outputs from Steps 1 to 6, the remaining gaps (Vercel build image needs Rust + `wasm-bindgen-cli` 0.2.127 + the `wasm32-unknown-unknown` target, or a prebuilt `packages/blasphem/dist`; the NOTICE license rows for K-MHaS and GermEval await the user's ruling; the browser smoke lands from the other agent's Task 15), and the branch tip.

Send to `blasphem-98` and `blasphem-36`: `DONE blasphem-49` plus the branch name `development`, the tip commit, and the note that the package exposes `init`, `BlasphemDetector`, `BlasphemResult`, and `index.d.ts` types, and does not expose a `judge()` function.

---

## Self-review notes

- Spec coverage: pnpm workspace and Turbo (Task 1, 3, 4); package exports, build, pack check, notices, toolchain (Task 3); Astro static site, single page, anchors (Tasks 4, 6 to 11); lazy WASM delivery and no preload (Tasks 4, 8); playground fields, `AUTO`, `ID` alias, states, `free()`, text stays local (Task 8); report-driven content, ordinal statement, limitations, evidence labels (Tasks 7, 9, 10); contribution paths and links (Task 11); metadata, Open Graph, sitemap, robots, favicon (Tasks 4, 6); accessibility, responsive, reduced motion (Tasks 6, 8, 12); Vercel headers (Task 4); acceptance criteria (Task 13).
- Type consistency: `Selection`, `LanguageCode`, `normalizeSelection`, `storageCode` come from `languages.ts`; `Snapshot`, `Phase`, `PhaseEvent`, `transition`, `verdictFor`, `statusCopy` from `playground-state.ts`; `__BLASPHEM_BASE__` and `__BLASPHEM_WASM_BYTES__` are defined by the integration and declared in `env.d.ts`; report field names match the JSON dumps taken on 2026-09-03.
- Deliberate omissions: no website test files (user decision); no browser smoke in the package (user decision, other agent's Task 15); no Cargo or Rust edits (other agent owns them).
