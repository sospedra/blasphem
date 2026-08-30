# blasphem

Experimental multilingual pre-send toxicity nudge. Deterministic rules, one lexicon and one sparse integer table per language, compiled to WebAssembly and to a native Node binary. No AI runtime. No network request after the judge is built.

The package carries code only, 1.13 MB of wasm. Language data lives in `@blasphem/packs`, one `.pack` and one `.detect` file per language, and a judge loads only the locales you ask for.

This package is private and unpublished. Build it from the repository:

```bash
pnpm install --frozen-lockfile
pnpm --filter @blasphem/packs run build
pnpm --filter blasphem run build
```

## Use

```ts
import { init, judge } from "blasphem";

await init({ locales: ["en", "es"], grawlix: true });

const v = judge("you are a stupid loser");
v.safe;    // false
v.score;   // 0.64
v.locale;  // "en"
v.grawlix; // "you are a @#$%&! loser"
```

`init` loads the locales once and installs one judge for the module. `judge()` is synchronous, so it runs on every keystroke. Before `init` resolves, and after `close()`, `judge()` returns the fail-open verdict `{ safe: true, score: 0, locale: null, grawlix: null }` and never throws. `ready()` says which. Calling `init` again with the same options is free; with other options it builds a new judge and retires the old one only when the new one is ready.

`locales` is required; `init({})` rejects with `BLASPHEM_LOCALES_EMPTY`.

Several judges at once, for example one per language on a moderation page, come from `createJudge(options)`, which returns a `Judge` with the same `judge()` and `close()`.

## Where the bytes come from

| Runtime | Code | Data |
| --- | --- | --- |
| browser, default | jsDelivr: `cdn.jsdelivr.net/npm/blasphem@<version>/dist/blasphem_bg.wasm` | jsDelivr: `cdn.jsdelivr.net/npm/@blasphem/packs@<version>/dist/` |
| browser, `assets: "/blasphem"` | `/blasphem/blasphem_bg.wasm` on your origin | `/blasphem/manifest.json`, then `<code>.pack` and `<code>.detect` per locale |
| browser, `assets: { wasm, packs }` | the `wasm` base | the `packs` base |
| Node | `@blasphem/node-<os>-<cpu>` when installed, else the wasm in this package | the installed `@blasphem/packs`, or `assets` as a directory |

The default needs nothing copied and nothing configured. Both versions are pinned to the ones this build was made with (`VERSIONS`), jsDelivr answers with `Access-Control-Allow-Origin: *`, serves `.wasm` as `application/wasm`, and caches exact versions for a year. Every file is verified against `manifest.json` before it parses. The preset serves bytes once both packages are published.

Self-hosting: `pnpm add @blasphem/packs`, then `blasphem-assets public/blasphem` copies the wasm and the packs into one directory (32 files, 10.34 MB), and `assets: "/blasphem"` points at it. Serve `.wasm` as `application/wasm`. The browser entry never resolves a path from `import.meta.url`.

Node: `pnpm add blasphem @blasphem/packs`. The loader reads the packs through `@blasphem/packs/files`, a module of literal `new URL` entries, and requires the native binary by a literal name per platform, so deployment tracers such as `@vercel/nft` include every file with no configuration. `judge.transport` reports `"native"` or `"wasm"`; `BLASPHEM_FORCE_WASM=1` skips the binary.

## Next.js

Two files.

`next.config.ts`, so the server loads the real modules instead of bundling them:

```ts
import type { NextConfig } from "next";

const config: NextConfig = {
  serverExternalPackages: ["blasphem", "@blasphem/packs"],
};

export default config;
```

A client component:

```tsx
"use client";

import { init, judge, type Judgement } from "blasphem";
import { useEffect, useState } from "react";

export function Composer() {
  const [verdict, setVerdict] = useState<Judgement | null>(null);

  useEffect(() => {
    void init({ locales: ["en", "es"], grawlix: true });
  }, []);

  return (
    <>
      <textarea onChange={(event) => setVerdict(judge(event.target.value))} />
      {verdict && !verdict.safe && <p role="status">Take another look: {verdict.grawlix}</p>}
    </>
  );
}
```

Until the packs arrive, `judge()` fails open and nothing fires. That is the nudge's promise.

A route handler:

```ts
import { init, judge } from "blasphem";

export const runtime = "nodejs";

const loaded = init({ locales: ["en", "es"] });

export async function POST(request: Request) {
  await loaded;
  const { text } = (await request.json()) as { text: string };
  return Response.json(judge(text));
}
```

Self-hosting instead of jsDelivr adds `"prebuild": "blasphem-assets public/blasphem"` to `package.json` and `assets: "/blasphem"` to `init`. A Content Security Policy, if you have one, is the next section.

## Content Security Policy

The browser loader does three things a CSP can block: it compiles WebAssembly, it fetches the wasm, and it fetches the packs. It evaluates no strings, spawns no workers, and creates no `blob:` URLs, so nothing else opens up.

| directive | value | why |
| --- | --- | --- |
| `script-src` | `'wasm-unsafe-eval'` | `WebAssembly.instantiate` is refused without it. Chrome 97, Firefox 102, Safari 16 and later. Older engines need `'unsafe-eval'`. |
| `connect-src` | the origins in `assets` | `fetch()` of `blasphem_bg.wasm`, `manifest.json`, `.pack`, and `.detect` |

Self-hosted, everything on your origin:

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'
```

The jsDelivr preset:

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' https://cdn.jsdelivr.net
```

Split bases: add each origin in `{ wasm, packs }` to `connect-src`. Bundle `blasphem` with your application; then `script-src` needs no CDN entry. If you load `browser.js` itself from a CDN, add that origin to `script-src` too.

Not needed: `worker-src`, `child-src`, `'unsafe-inline'`, `Cross-Origin-Opener-Policy`, or `Cross-Origin-Embedder-Policy`. The wasm runs on the main thread without `SharedArrayBuffer`.

The browser smoke serves its page under `default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; connect-src 'self' https://cdn.jsdelivr.net` (`'unsafe-inline'` covers the test page's own inline module, not the library) and passes in Chromium and WebKit. `reports/browser-smoke.json` records the policy under `content_security_policy`.

Serve `blasphem_bg.wasm` as `application/wasm`. With another type the glue logs a warning and falls back from `instantiateStreaming` to `WebAssembly.instantiate`, which is slower but works.

jsDelivr answers `Cross-Origin-Resource-Policy: cross-origin` (HEAD request, 2026-09-04), so the preset also works on a page that sets `Cross-Origin-Embedder-Policy: require-corp`.

## Options

The same object goes to `init` and to `createJudge`.

| option | type | default | meaning |
| --- | --- | --- | --- |
| `locales` | `string[]` | required | Lowercase locale codes to load. |
| `assets` | `string \| { wasm, packs }` | jsDelivr in browsers, installed packs on Node | A path on your origin, `"jsdelivr"`, or split bases. On Node, a packs directory or `{ packs }`. |
| `detectLanguage` | `boolean` | `true` | Route by detected language. Loads one `.detect` per locale. |
| `grawlix` | `boolean` | `false` | Return the masked text. |

Locales: `en`, `zh`, `es`, `ar`, `ms`, `pt`, `fr`, `hi`, `ru`, `ja`, `de`, `tr`, `vi`, `ko`, `it`. `id` is an alias for `ms`. Any other value throws.

With `detectLanguage: false` the judge scores every loaded locale and reports the highest.

## Result

| field | type | meaning |
| --- | --- | --- |
| `safe` | `boolean` | True when no nudge is due. |
| `score` | `number` | Ordinal, 0 through 1. Not a probability. |
| `locale` | `string \| null` | The locale that produced the score. |
| `grawlix` | `string \| null` | The masked text, when requested. |

Text that no loaded locale routes returns `locale: null`, `score: 0`, and `safe: true`. The nudge fails open. That includes text detected as a language you did not load.

## Errors

`init` and `createJudge` reject with a plain `Error` whose `code` is one of:

| code | when |
| --- | --- |
| `BLASPHEM_LOCALES_EMPTY` | `locales` missing or empty |
| `BLASPHEM_LOCALE_UNSUPPORTED` | an unknown code |
| `BLASPHEM_LOCALE_MISSING` | a known code the installed packs do not include |
| `BLASPHEM_ASSETS_REQUIRED` | `assets` of a shape the runtime cannot use, or Node without `@blasphem/packs` and without a directory |
| `BLASPHEM_FETCH_FAILED` | a file did not load; the message names it |
| `BLASPHEM_DIGEST_MISMATCH` | bytes disagree with `manifest.json` |
| `BLASPHEM_FORMAT_VERSION` | a pack or manifest version this build does not read |
| `BLASPHEM_PACK_INVALID` | the engine rejected the bytes |

The module-level `judge()` never throws; it fails open before `init` and after `close()`. A `Judge` from `createJudge` throws `BLASPHEM_CLOSED` after its own `close()`.

## Test

```bash
pnpm --filter blasphem test
pnpm --filter blasphem run test:browser
```

`test` runs the pack check and the Node smoke against `dist/` twice, once on the native binary and once with `BLASPHEM_FORCE_WASM=1`, and requires identical verdicts. `test:browser` serves `dist/` and the packs over HTTP, runs the same cases in Playwright Chromium and WebKit, asserts that an EN-only judge downloads exactly `manifest.json`, `en.pack`, and `en.detect`, and writes `reports/browser-smoke.json`. Install the pinned browsers once:

```bash
pnpm --filter blasphem exec playwright install chromium webkit
```

## Pinned tools

See `TOOLCHAIN.md`. The build stops when `wasm-bindgen --version` differs from the crate pin.
