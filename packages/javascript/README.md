# blasphem

Local toxicity checks for JavaScript and TypeScript.
Browsers run WebAssembly.
Node uses a native addon when available, with WebAssembly as the fallback.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

The public npm release is pending.
Use [the source build](#build-from-source) for the current checkout.

The release installation command is:

```sh
npm install blasphem @blasphem/packs
```

The package includes TypeScript declarations and ESM exports.
Its manifest specifies Node 24.18.0 and pnpm 11.13.0.

## Quick start

This example uses Node's installed language packs.
For a browser, configure [browser assets](#browser-assets) first.

```ts
import { init, judge } from "blasphem";

await init({ locales: ["en", "es"], grawlix: true });

const verdict = judge("you are a stupid loser");
console.log(verdict);
```

Initialize once and reuse the judge.
The call to `judge` is synchronous.

## API

| Export | Purpose |
| --- | --- |
| `init(options)` | Load data and initialize the module judge |
| `judge(text)` | Return a `Judgement` synchronously |
| `ready()` | Report whether a module judge is ready |
| `close()` | Release the module judge |
| `createJudge(options)` | Create an independent judge |

Before initialization and after closure, the module returns a safe verdict.
Repeated initialization with unchanged options reuses the judge.
Different options replace it after the new judge becomes ready.
A failed initialization preserves the previous judge.

For independent instances:

```ts
import { createJudge } from "blasphem";

const detector = await createJudge({ locales: ["en"] });
try {
  console.log(detector.judge("you are a stupid loser"));
  console.log(detector.transport);
} finally {
  detector.close();
}
```

An independent judge throws `BLASPHEM_CLOSED` after closure.
Its `transport` is `"native"` or `"wasm"`.

## Options

| Option | Default | Meaning |
| --- | --- | --- |
| `locales` | Required | Nonempty array of supported locale codes |
| `assets` | Runtime-specific | Pack directory or browser asset bases |
| `detectLanguage` | `true` | Route to the detected language |
| `grawlix` | `false` | Return masked text |

With detection disabled, the judge returns the highest score across loaded locales.
Use `id` for Indonesian and `ms` for Malay.
See [all 16 supported languages](../javascript-packs/README.md#locales).

## Result

| Field | Type | Meaning |
| --- | --- | --- |
| `safe` | `boolean` | No warning is due |
| `score` | `number` | Ordinal value from 0 to 1 |
| `locale` | `string \| null` | Selected model profile |
| `grawlix` | `string \| null` | Masked text when requested |

The score is not a probability.
Unrouted text returns `{ safe: true, score: 0, locale: null, grawlix: null }`.
See [the API contract](../javascript-common/src/contract.ts).

## Browser assets

Copy the engine and packs from your application's installed packages:

```sh
pnpm exec blasphem-assets public/blasphem
```

Serve that directory at `/blasphem`, then initialize the browser entry:

```ts
import { init, judge } from "blasphem";

await init({
  locales: ["en", "es"],
  assets: "/blasphem",
  grawlix: true,
});

console.log(judge("you are a stupid loser"));
```

The loader fetches the manifest and files for the requested profiles.
Serve `.wasm` files as `application/wasm`.
Message checks need no network connection after initialization.

| `assets` value | Browser behavior |
| --- | --- |
| `"/blasphem"` | Load code and data from one base |
| `{ wasm: "/engine", packs: "/packs" }` | Use separate directory bases |
| Omitted or `"jsdelivr"` | Use versioned npm CDN URLs |

The CDN mode requires a published version.
Use local assets for unreleased builds.

### Content Security Policy

A policy for bundled scripts and assets on your origin is:

```http
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'
```

Allow each external asset origin in `connect-src`.
CDN mode needs `https://cdn.jsdelivr.net`.
The [`wasm-unsafe-eval` directive](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src#unsafe_webassembly_execution) permits WebAssembly compilation.

## Node assets

Node reads the installed `@blasphem/packs` by default.
Set `assets` to a filesystem directory to use custom files.
It does not use the browser CDN preset.

`BLASPHEM_FORCE_WASM=1` disables native addon selection.

For Next.js server usage, externalize both packages in `next.config.ts`:

```ts
import type { NextConfig } from "next";

const config: NextConfig = {
  serverExternalPackages: ["blasphem", "@blasphem/packs"],
};

export default config;
```

Initialize the browser entry from client-side application code.

## Errors

`init` and `createJudge` reject with an `Error` carrying a `code`:

| Code | Cause |
| --- | --- |
| `BLASPHEM_LOCALES_EMPTY` | No locales |
| `BLASPHEM_LOCALE_UNSUPPORTED` | Unknown locale |
| `BLASPHEM_LOCALE_MISSING` | Missing manifest entry |
| `BLASPHEM_ASSETS_REQUIRED` | Missing or invalid asset configuration |
| `BLASPHEM_FETCH_FAILED` | A file could not be read |
| `BLASPHEM_DIGEST_MISMATCH` | A file differs from its recorded digest |
| `BLASPHEM_FORMAT_VERSION` | Unsupported manifest or pack format |
| `BLASPHEM_PACK_INVALID` | Invalid data |

The loader checks file digests before engine construction.
See [the loader](../javascript-common/src/loader.ts) for initialization behavior.

## Build from source

Install the [development tools](../../CONTRIBUTING.md#set-up).
Run from the repository root:

```sh
pnpm install --frozen-lockfile
pnpm --filter @blasphem/packs run build
pnpm --filter blasphem run build
```

Link the built packages from your application's directory:

```sh
pnpm add link:/path/to/blasphem/packages/javascript link:/path/to/blasphem/packages/javascript-packs
```

Run the Node checks from the repository root:

```sh
pnpm --filter blasphem run test:node
```

For browser checks:

```sh
pnpm --filter blasphem exec playwright install chromium webkit
pnpm --filter blasphem run test:browser
```

These checks exercise generated distributions.
Rebuild after source changes.

[Contribute](../../CONTRIBUTING.md) · [CLI guide](../cli/README.md) · [WASM bindings](../../crates/blasphem-wasm/README.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
