# blasphem

Local toxicity checks for JavaScript and TypeScript.
Browsers run WebAssembly.
Node uses a native addon when available, with WebAssembly as the fallback.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

These registry commands require the published `1.0.0` release.
Use [the source build](#build-from-source) for the current checkout.

The release installation command is:

```sh
npm install blasphem
```

The package includes TypeScript declarations and ESM exports.
Its manifest specifies Node 24.18.0 and pnpm 11.13.0.

## Quick start

Declare the selection once in the application's `package.json`:

```json
{
  "blasphem": {
    "locales": ["en", "es"],
    "assets": "bundled",
    "detectLanguage": true
  }
}
```

Node reads this configuration from the application directory.
Browsers require the [build integration or asset helper](#browser-assets).

```ts
import { init, judge } from "blasphem";

await init();

const verdict = judge("you are a stupid loser");
console.log(verdict);
```

Initialize once and reuse the judge.
The call to `judge` is synchronous.
Runtime behavior remains configurable with `await init({ grawlix: true })`.

## API

| Export | Purpose |
| --- | --- |
| `init(options?)` | Read application configuration and initialize the module judge |
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

## Configuration

| Option | Default | Meaning |
| --- | --- | --- |
| `locales` | Required | Nonempty locale array or `"all"` |
| `assets` | `"bundled"` | Local data, or `"remote"` in browsers |
| `detectLanguage` | `true` | Route to the detected language |

`grawlix` is an initialization option, not a packaging choice. Its default is `false`.
The library installs its exact internal data dependency automatically.
`"all"` expands against the installed engine release, never the CDN catalog.
Unknown codes and empty arrays fail. Aliases normalize and deduplicate in registry order.
Explicit `"all"` fails when a reduced export lacks release data.

With detection disabled, the judge returns the highest score across loaded locales.
Use `id` for Indonesian and `ms` for Malay.
See [all 16 supported languages](../javascript-packs/README.md#locales).

## Result

| Field | Type | Meaning |
| --- | --- | --- |
| `safe` | `boolean` | No warning is due |
| `score` | `number` | Ordinal value from 0 to 1 |
| `locale` | `string \| null` | Selected model profile |
| `grawlix` | `string \| null` | Masked text for unsafe verdicts when requested, otherwise `null` |

The score is not a probability.
TypeScript narrows `grawlix` to `null` when `safe` is `true`.
Unrouted text returns `{ safe: true, score: 0, locale: null, grawlix: null }`.
See [the API contract](../javascript-common/src/contract.ts).

## Browser assets

For Vite, register the build integration:

```ts
// vite.config.ts
import { defineConfig } from "vite";
import blasphem from "blasphem/vite";

export default defineConfig({ plugins: [blasphem()] });
```

The integration emits configuration and only selected local assets.
Generated URLs include Vite's public `base` path.
No additional initialization options are required.

Other builds can publish assets with the helper:

```sh
pnpm exec blasphem-assets public/blasphem --base /blasphem
```

Serve that directory at `/blasphem`. Load its configuration before the application module:

```html
<script src="/blasphem/config.js"></script>
<script type="module" src="/src/main.ts"></script>
```

Use the application's public prefix in both `--base` and the script URL.
The helper reads the same `package.json` selection as Vite.
It removes only its previously generated files after selection changes.
Bundled builds include selected packs, optional detection slices, WASM, and notices.
Remote builds emit configuration and notices, without language data or WASM.
Builds do not request CDN files.
Serve `.wasm` files as `application/wasm`.
Message checks need no network connection after initialization.

Set `"assets": "remote"` in application configuration for exact-version jsDelivr delivery.
`"jsdelivr"` remains a compatibility alias.
The browser stores the verified manifest, selected files, and WASM in IndexedDB.
It rechecks file lengths and SHA-256 hashes before reuse.
Offline restarts make zero asset requests while the host application can start offline.
Storage eviction or corruption can require downloads again.
Downloads share in-flight requests and allow at most two attempts per file.
Each request has a 30-second deadline. Invalid or unavailable files reject initialization.
Remote delivery reduces bundle size, not selected model memory.

Advanced `createJudge` options also accept a directory URL or `{ wasm, packs }` bases.

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
Node rejects `"remote"`, `"jsdelivr"`, and URL sources.

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

### Reduced Node deployments

```sh
pnpm exec blasphem-export --locales en,es --output ./vendor
```

The new directory contains a runtime, selected data, configuration, and required notices.
It includes the installed compatible native addon, or its local WASM fallback.
Use `--no-detect` to omit detection files.
Run the application from `vendor`, or copy its configuration and `node_modules` into your deployment.
The output runs without the full data catalog. Existing output directories are rejected.
Native addons retain their operating-system and CPU requirements.

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
pnpm add link:/path/to/blasphem/packages/javascript
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

### Build without Rust

Vercel uses the committed WASM binary and its three generated binding files.
The build checks their SHA-256 hashes and the Rust source digest.
Missing, stale, or changed artifacts stop the build with a rebuild command.
TypeScript compilation, core copying, and version generation still run.

Run the same website build from the repository root:

```sh
env BLASPHEM_WASM_PREBUILT=1 pnpm exec turbo run build --filter=web
```

After Rust source, toolchain, or WASM build script changes, regenerate the artifacts:

```sh
env -u BLASPHEM_WASM_PREBUILT pnpm --filter blasphem build
pnpm --filter blasphem prebuilt:check
```

Commit the four generated `src/blasphem*` files and `src/blasphem.prebuilt.json` together.
CI checks the committed artifacts before rebuilding them.
The default package build always compiles Rust and generates new bindings.

[Contribute](../../CONTRIBUTING.md) · [CLI guide](../cli/README.md) · [WASM bindings](../../crates/blasphem-wasm/README.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
