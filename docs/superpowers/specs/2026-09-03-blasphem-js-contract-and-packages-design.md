# Blasphem JavaScript contract and package layout design

## Status

Rubén chose the package layout, the Node transport, the missing-locale behavior, and the digest policy in chat on 2026-09-03. He raised no objection to the section "Contract".

Rubén approved implementation of the whole document in chat on 2026-09-03 ("implement it all"). See "Implementation notes" for what the build refined.

This document supersedes the "Npm package" and "WASM delivery" sections of `docs/superpowers/specs/2026-09-02-blasphem-public-package-and-corpus-design.md` where they conflict. These statements from that document are retired: browsers only, `await init()`, `new BlasphemDetector(language)`, `AUTO`, `free()` on exported classes, and all 15 models in the default package.

## Goals

The project shall ship one JavaScript contract for browsers, Node.js, and React Native.

A client shall download only the code and the locales it asked for.

The core binary shall carry no language data.

Node.js shall run native code when a prebuilt binary exists for the platform, and wasm otherwise.

`blasphem` and `@blasphem/packs` shall release independently.

The published packages shall carry one license each.

## Non-goals

The `.pack` and `.detect` byte layouts. They get their own specification. This document names only what the JavaScript side reads from them.

The React Native native module internals, and the build step that copies chosen locales into the app bundle.

A bundler plugin that copies packs into a web application's public directory.

Publishing. Every package stays `private: true` until Rubén lifts it, per the 2026-09-02 specification.

Digest provenance. The manifest proves integrity in transit only.

A hosted CDN of our own. Every runtime reads packs from disk, from the application's own origin, or, since 2026-09-04, from the jsDelivr npm CDN when the application opts in with `assets: "jsdelivr"`.

## Measurements

Measured on 2026-09-03 with `stat -f%z` and `ls -la`.

| Part | Bytes | MB |
| --- | --- | --- |
| `blasphem_bg.wasm`, everything embedded | 10,760,231 | 10.26 |
| `blasphem-language-15-v2.bin` | 5,048,468 | 4.81 |
| 15 sparse lexica `*-v2.bin` | 1,966,672 | 1.88 |
| 15 HurtLex TSV | 2,561,810 | 2.44 |
| Code, by subtraction | 1,183,281 | 1.13 |

An EN-only client needs 1.13 MB of code, 0.42 MB of EN pack input, and one EN detect slice. The detect slice is unmeasured because its writer does not exist. Arithmetic on the v2 model sections puts the average language near 0.23 MB. EN-only lands near 1.78 MB against 10.26 MB today.

Node transport spike, 2026-09-03, throwaway code in `/tmp`. Node v24.18.0, arm64, same source for both binaries, 3,000 corpus rows at 200 per language, one warm-up pass, five timed rounds, best round. Two repetitions agreed within 5 percent. Verdicts were identical on every row.

| 15 locales | wasm | napi |
| --- | --- | --- |
| calls per second | 14,045 | 21,393 |
| p50 | 58.2 µs | 39.8 µs |
| p99 | 247 µs | 144 µs |
| construct | 335 ms | 201 ms |
| RSS delta | 83.38 MB | 46.86 MB |

| EN only | wasm | napi |
| --- | --- | --- |
| calls per second | 155,861 | 253,590 |
| p50 | 2.9 µs | 1.6 µs |
| p99 | 76 µs | 49.5 µs |
| construct | 48.6 ms | 23.0 ms |
| RSS delta | 64.52 MB | 17.19 MB |

Rubén ruled the napi gain worth a prebuild matrix.

## Packages

| Package | Contents | Reaches the application |
| --- | --- | --- |
| `blasphem` | shared core inlined, the wasm, the Node loader | bundler, or `import` on Node |
| `@blasphem/node-<os>-<cpu>` | one `.node` binary each | `optionalDependencies` of `blasphem`; npm installs the matching one |
| `@blasphem/react-native` | Nitro module over the Rust core, podspec, Gradle project | autolinking |
| `@blasphem/packs` | 15 `.pack`, 15 `.detect`, `manifest.json` | Node reads `node_modules`. Web and React Native copy chosen locales |
| `packages/core` | types, validation, locale table, load policy | never published; inlined into each `dist` at build |

`blasphem` shall stay unscoped. It is the package people type.

The initial native set shall be `darwin-arm64`, `darwin-x64`, `linux-x64-gnu`, `linux-arm64-gnu`, `linux-x64-musl`, `linux-arm64-musl`, and `win32-x64-msvc`. `@napi-rs/cli` shall generate the platform packages. Each platform package shall set `os` and `cpu`, and `libc` where it applies.

`@blasphem/react-native` shall not depend on `blasphem`. Hermes cannot run wasm, so the dependency would be dead bytes.

`@blasphem/react-native` shall list `blasphem` as an optional peer dependency and route its `browser` export condition to it. One import line then serves iOS, Android, and Expo web.

`@blasphem/packs` shall carry the license of its lexica, not the Apache-2.0 of the code. `resources/datasets/source-lock-v1.json` records `CC-BY-SA-4.0` and `CC-BY-4.0` sources today. The clean-room lexicon plan (`docs/superpowers/plans/2026-09-03-clean-room-lexicon.md`) may change that license. The package boundary holds either way.

### Rejected layouts

One package for everything. Autolinking scans `node_modules/*/package.json` and does not read the import graph, so every web install would link the native library.

`@blasphem/web` beside `@blasphem/node`. An Astro or Next.js application renders on the server and hydrates in the browser from one module. Two packages force it to alias by condition.

A published `@blasphem/core`. The shared surface is types, a locale table, and a load policy. Publishing it adds a version to keep in lockstep on every release for a package nobody installs by hand.

`@blasphem/runtime` as the main name. It reads as a helper bundle, and it spends the unscoped name on nothing.

## Shared core

`packages/core` shall be a private workspace package. Each published package's build shall inline it, so no published package depends on it.

The core shall own the TypeScript types, option validation, the locale table, `manifest.json` parsing, the load policy, and the judge cache.

A build step shall generate the locale table from `Language::ALL` in the Rust crate. The table shall be the only JavaScript source for locale codes and aliases. `apps/web/src/lib/languages.ts:11-27` hand-copies the list today, including the `ID` to `MS` alias at line 29 that mirrors `src/embedded.rs:17`. That copy shall consume the generated table for codes and keep only display names and text direction.

Anything that knows the pack byte layout shall stay in Rust and reach JavaScript through a transport. Digest verification shall happen in Rust, where `sha2` is already a dependency, so the core needs no WebCrypto and React Native needs no polyfill.

## Contract

One source file in `packages/core`. Identical on all transports.

```ts
export interface JudgeOptions {
  locales: string[];          // required. [] throws at construction
  assets?: string;            // browser: required base path. Node: optional. React Native: ignored
  detectLanguage?: boolean;   // default true
  grawlix?: boolean;          // default false
}

export interface Judgement {
  safe: boolean;
  score: number;              // ordinal 0..1, not a probability
  locale: string | null;
  grawlix: string | null;
}

export interface Judge {
  readonly locales: readonly string[];
  readonly transport: "wasm" | "native";
  judge(text: string): Judgement;
  close(): void;
}

export function createJudge(options: JudgeOptions): Promise<Judge>;
```

Construction shall be asynchronous. Judging shall be synchronous. The nudge runs on every keystroke. On React Native this requires JSI, so `@blasphem/react-native` shall build on Nitro Modules (`react-native-nitro-modules`). Rubén chose Nitro over `uniffi-bindgen-react-native` on 2026-09-03. The legacy bridge is out. How Nitro reaches the Rust core, through Swift and Kotlin or through a C++ HybridObject over the C shim, belongs to the React Native specification.

`locales` shall be required. `createJudge({})` and `createJudge({ locales: [] })` shall throw before any byte loads.

`judge()` shall never throw on an open judge. Fail-open lives at call time only.

`close()` shall release the wasm or native memory. `judge()` on a closed judge shall throw. Today `packages/blasphem/src/judge.ts:25` caches judges in a `Map` for the process lifetime.

`transport` shall report which engine answered. It exists for tests and diagnostics.

The free function `judge(text, options)` at `packages/blasphem/src/judge.ts:47` shall be removed. `createJudge` shall be the only entry.

The top-level `await load()` at `packages/blasphem/src/index.ts:6` shall be removed. Importing `blasphem` shall load no bytes.

`BlasphemDetector`, `BlasphemResult`, and the `AUTO` selection at `crates/blasphem-wasm/src/lib.rs:165-244` shall be removed from the wasm crate. The web playground shall migrate to `createJudge`.

## Resolution

`blasphem` `package.json` shall carry these export conditions for `.`: `types`, `browser`, `node`, `default`. `browser` and `default` shall resolve to the wasm loader. `node` shall resolve to the Node loader.

`blasphem` shall keep `main`, `module`, and `types` top-level fields for tools that ignore export conditions.

`blasphem` shall keep `"sideEffects": false`.

The Node loader shall try `@blasphem/node-<platform>-<arch>` and its `libc` variant through `createRequire`, catch a resolution failure, and fall back to the wasm. `transport` shall report which one loaded.

Code reachable through the `browser` condition shall not use `new URL(x, import.meta.url)`. It breaks in Next.js server bundles, Turbopack, and `@vercel/nft`, as recorded on 2026-09-03.

The Node loader shall locate its own wasm with `import.meta.resolve("blasphem/blasphem_bg.wasm")`. `blasphem` shall export `./blasphem_bg.wasm` for that purpose, as it does today. The README shall tell Next.js users to list `blasphem` in `serverExternalPackages`, which is what `sharp` and `onnxruntime-node` require.

`@blasphem/react-native` `package.json` shall carry `react-native` and `browser` conditions. `react-native` shall resolve to the native module. `browser` shall re-export from the optional peer `blasphem`.

## Loading

Every transport shall end in one constructor call, made by the core:

```ts
new Engine(
  entries: Array<{
    locale: string;
    pack: Uint8Array;
    packSha256: string;
    detect: Uint8Array | null;
    detectSha256: string | null;
  }>,
  detectLanguage: boolean,
  grawlix: boolean,
)
```

The engine shall verify each digest before parsing. The core shall pass bytes and expected digests and never hash.

| Runtime | Source of bytes | `assets` |
| --- | --- | --- |
| browser | `fetch(\`${assets}/manifest.json\`)`, then `fetch(\`${assets}/${locale}.pack\`)`, plus `.detect` when detection is on | required. The application copies `node_modules/@blasphem/packs/*` into its public directory |
| Node | `fs.readFile` under the directory of `import.meta.resolve("@blasphem/packs/package.json")` | optional. Overrides the packs directory only |
| React Native | the native module reads the application bundle | ignored |

The core shall read `manifest.json` first. A requested locale absent from the manifest shall throw before any pack loads.

All files for one judge shall load in parallel. Construction shall wait for all of them. No partial judge shall exist.

`detectLanguage: false` shall skip every `.detect`.

The browser loader shall throw when `assets` is missing. It shall never guess a path.

## Compatibility

Every `.pack` and `.detect` shall open with a magic and a format version. The engine shall reject an unknown version at construction, naming the file and the version it accepts. This is a construction error, never fail-open.

The format version shall decouple the two packages. `blasphem` at one release shall read every `@blasphem/packs` release that writes a format version it knows.

`@blasphem/packs` shall ship `manifest.json` with this shape:

```json
{
  "formatVersion": 1,
  "files": {
    "en.pack":   { "bytes": 0, "sha256": "<64 hex chars>" },
    "en.detect": { "bytes": 0, "sha256": "<64 hex chars>" }
  }
}
```

The manifest shall list every file the package ships. The core shall pass each file's `sha256` to the engine. A mismatch shall throw at construction, naming the file, the expected digest, and the actual digest.

`blasphem` shall never pin pack digests. Rubén chose the manifest over pinning on 2026-09-03 so a pack retrain does not force a core release.

## Error behavior

Construction shall throw a plain `Error` with a `code` property. No shared error class exists because the core is inlined, so `instanceof` across packages cannot hold.

| `code` | When |
| --- | --- |
| `BLASPHEM_LOCALES_EMPTY` | `locales` missing or empty |
| `BLASPHEM_LOCALE_UNSUPPORTED` | a code the generated locale table does not know |
| `BLASPHEM_LOCALE_MISSING` | a known code absent from `manifest.json` |
| `BLASPHEM_ASSETS_REQUIRED` | browser without `assets` |
| `BLASPHEM_FETCH_FAILED` | a fetch or read failed; names the file |
| `BLASPHEM_DIGEST_MISMATCH` | bytes do not match the manifest; names file, expected, actual |
| `BLASPHEM_FORMAT_VERSION` | unknown pack or detect version; names file, found, accepted |
| `BLASPHEM_PACK_INVALID` | the engine rejected the bytes after the digest passed |
| `BLASPHEM_CLOSED` | `judge()` after `close()` |

Call time shall fail open. Text that detection routes to a locale the judge did not load shall return `{ safe: true, score: 0, locale: null, grawlix: null }`. Rubén chose this on 2026-09-03: do not score it. The cost is known: with only EN and ES loaded, 19 of 5,011 foreign rows misroute, 0.4 percent.

Text that detection cannot route at all shall return the same value. That matches `unknown_result` at `crates/blasphem-wasm/src/lib.rs:151-162` today.

## Migration

`packages/blasphem/src/index.ts`, `judge.ts`, and `load.ts` shall be replaced by the inlined core plus two loaders.

The web playground at `apps/web/src/scripts/playground.ts:85` shall call `createJudge` with `assets` set to the hashed directory the Astro integration already produces. The integration shall copy all 15 packs and detect slices beside the wasm, because the playground offers every language.

`turbo.json` shall gain `@blasphem/packs#build` with the `data/raw-v1/hurtlex/**` and `resources/models/**` inputs that `blasphem#build` lists at `turbo.json:17` and `:18` today. `blasphem#build` shall drop them. `web#build` shall depend on both.

## Sequencing

This document depends on step 1 of the 2026-09-03 plan, the Rust core. `src/judge.rs:89-101` reads only embedded data today, and `crates/blasphem-train` has no pack writer. The JavaScript loader has nothing to load until these exist:

1. A bytes-in constructor on the Rust `Judge` that accepts the `Engine` entries above, digests included.
2. The `.pack` and `.detect` containers with magic and format version.
3. A writer for both, plus `manifest.json`, in `blasphem-train`.

The wasm crate and the napi crate shall expose that constructor unchanged. The spike in `/tmp` showed a napi wrapper over the current `Judge` is 50 lines.

## Tests

New test files are opt-in in this repository. The statements below name behavior. Where a statement needs a new file, the implementation plan shall ask first.

`packages/blasphem/scripts/node-smoke.mjs` and `browser-smoke.mjs` shall move to `createJudge` and keep their case list in `tests/cases.mjs`.

The browser smoke shall record network requests and shall fail when a session that asked for `["en"]` requested any other locale's file.

The Node smoke shall run once with the platform package present and once with it removed, and shall assert `transport` and identical verdicts.

`packages/blasphem/scripts/pack-check.mjs` shall check the new export conditions and the absence of pack files from the `blasphem` archive.

## Implementation notes, 2026-09-03

The implementation landed the same day. These decisions were made while building and refine the sections above.

**Browser wasm location.** The browser loader fetches `${assets}/blasphem_bg.wasm`. The application copies the wasm beside the packs. The glue is generated with `wasm-bindgen --omit-default-module-path`, so no `import.meta.url` reference exists anywhere on the browser path; `scripts/pack-check.mjs` asserts it.

**Engine API is a builder.** Bindings cannot marshal an array of objects cheaply, so every binding exposes `EngineBuilder(detectLanguage, grawlix)`, `add(locale, pack, packSha256, detect, detectSha256)`, and `build()`. Nitro creates objects without constructor arguments, so its builder takes `configure(detectLanguage, grawlix)` first. All three wrap `blasphem::Engine` in `src/engine.rs`.

**Error text.** Rust `Display` for `JudgeError`, `PackError`, and `EngineError` starts with the contract code. Bindings pass the text through; `packages/core/src/errors.ts` splits on the first `: `.

**Feature gates.** `blasphem/embedded` (default on) and `blasphem-language/embedded-model` (default on) hold every `include_bytes!` of language data. The wasm, napi, and ffi crates turn them off. The registry keeps only rule identity and profiles; `src/embedded.rs` owns the compiled artifacts.

**Pack layout, format 1.** 24-byte header: magic `BLSPHPCK`, u32 version, two-byte lowercase code, u16 rule-pack version, u32 artifact length, u32 lexicon length; then the sparse artifact and the raw lexicon TSV. `src/pack.rs`.

**Detect slice layout, format 1.** 68-byte header: magic `BLSPHDET`, u32 version, two-byte code, u16 zero, u32 table length, u32 entry count, f32 average, 40-byte upstream commit; then 12 bytes per entry: u32 slot, u32 fingerprint, u32 `weight_bits | run_offset`. Entries sorted by (fingerprint, slot). The run offset is the distance from the start of the entry's occupied run, so merged slices reproduce the full table's probe result without the 0.25 MB occupancy bitmap. The longest run in the committed model is 98 slots. `crates/blasphem-language/src/slice.rs`. The 20,224-byte Unicode tables are compiled into `blasphem-language` as `data/eld-tables-v1.bin`; a test pins them to the committed model.

**Slice sizes.** ar 0.19 MB, de 0.43, en 0.33, es 1.04, fr 0.41, hi 0.02, it 0.36, ja 0.01, ko 0.00, ms 0.75, pt 0.41, ru 0.22, tr 0.45, vi 0.11, zh 0.13. All 30 files: 9.19 MB.

**Measured.** wasm 1.15 MB (was 10.26). napi `.node` for darwin-arm64 1.66 MB. Browser, Brotli: EN-only judge with detection 0.65 MB, all locales 5.46 MB. Both smokes pass: Node 58 cases on native and on wasm with identical verdicts; Chromium 151 and WebKit 26.5, 58 cases, EN-only requests exactly `manifest.json`, `en.pack`, `en.detect`.

**Packs license.** `@blasphem/packs` declares `CC-BY-NC-SA-4.0`, the license `packages/blasphem/NOTICE` already asserted for HurtLex. `blasphem`'s NOTICE now lists only the ELDC Unicode tables.

**Test wiring.** `blasphem` lists `@blasphem/packs` as a devDependency only, so the Node smoke exercises the default `import.meta.resolve("@blasphem/packs/manifest.json")` path inside the workspace. Consumers install both.

**React Native.** `packages/react-native` holds the Nitro spec, the C++ HybridObjects over `crates/blasphem-ffi` (`cpp/blasphem.h`), a Swift and a Kotlin `BlasphemAssets` reader for the app bundle, the podspec, Gradle, and CMake. Verified: nitrogen generates, the Rust archives build for iOS device, iOS simulator, and three Android ABIs, the C++ compiles against Nitro's headers, the TypeScript compiles. Not verified: an iOS or Android application build.

**Turbo.** `blasphem#build` depends on `@blasphem/core#check` and inlines `packages/core/src`. `@blasphem/packs#build` owns the model and lexicon inputs. `web#build` depends on both.

## Revision, 2026-09-04

**jsDelivr beside self-hosting.** `assets` accepts three forms in the browser: one path that serves the wasm and the packs together, the literal `"jsdelivr"`, or `{ wasm, packs }`. The preset resolves to `https://cdn.jsdelivr.net/npm/blasphem@<version>/dist` and `https://cdn.jsdelivr.net/npm/@blasphem/packs@<version>/dist`, with both versions baked in at build (`dist/version.generated.js`, exported as `VERSIONS`). Exact pins keep verdicts reproducible; a caller who wants a floating range builds the bases with `jsdelivrBases`. On Node, `"jsdelivr"` throws `BLASPHEM_ASSETS_REQUIRED`; Node reads files. Verified with HEAD requests on 2026-09-04: jsDelivr sends `access-control-allow-origin: *`, serves `.wasm` as `application/wasm`, caches exact versions for one year and ranges for seven days. The browser smoke routes `cdn.jsdelivr.net` to the local build and asserts the preset requests exactly `manifest.json`, `en.pack`, and `en.detect` under the pinned packs version, and that split bases work.

**Content Security Policy.** The browser path needs `script-src 'wasm-unsafe-eval'` and the `assets` origins in `connect-src`, nothing else: `grep` of `dist/blasphem.js`, `browser.js`, `wasm-engine.js`, and `core/*.js` finds no `eval`, `new Function`, `Worker`, or `blob:`. The README carries the exact policies for self-hosting and for the preset. jsDelivr answers `Cross-Origin-Resource-Policy: cross-origin` (HEAD request, 2026-09-04), so the preset also works on a page that sets `Cross-Origin-Embedder-Policy: require-corp`.

**Setup collapsed to one config line, 2026-09-04.** Rubén called the six-file Next.js setup "insanely complex", and it was. Three changes removed it. (1) The browser defaults `assets` to the jsDelivr preset when the option is omitted; this reverses "the browser loader shall throw when `assets` is missing". `BLASPHEM_ASSETS_REQUIRED` now covers only an unusable shape, or Node without packs and without a directory. (2) The Node entry reads packs through `@blasphem/packs/files`, a generated module of literal `new URL(name, import.meta.url)` entries, requires the native binary by a literal name per platform, and locates its own wasm with `new URL("./blasphem_bg.wasm", import.meta.url)`. `@vercel/nft` 1.11.0 traced the result: 32 pack files, the native binary, and the wasm, from `dist/node.js` alone, against 0 of each before the change. This corrects the 2026-09-03 note that nft does not handle `new URL(x, import.meta.url)`: it does, on real Node modules. The rule stands for the browser path, which stays free of it; `pack-check` still asserts that. (3) `blasphem-assets <directory>`, a `bin` in the package, copies the wasm and the packs for self-hosting. Next.js therefore needs `serverExternalPackages: ["blasphem", "@blasphem/packs"]` and nothing else; `outputFileTracingIncludes` is gone. Verified: Node smoke 58 cases native and wasm, browser smoke in Chromium 151 and WebKit 26.5 with the omitted-assets default routed to the pinned jsDelivr URLs, `blasphem-assets` copied 32 files, 10.34 MB.

**`init` and `judge` replace `createJudge` as the primary API, 2026-09-04.** Rubén rejected `createJudge` as the thing an application calls. Every runtime entry now exports `init(options): Promise<void>`, `judge(text): Judgement`, `ready(): boolean`, and `close(): void` from one module-level judge, implemented once in `packages/core/src/singleton.ts` over that runtime's `createJudge`. `judge()` never throws: before `init` resolves and after `close()` it returns the fail-open verdict, which is the nudge's own semantics while packs load. `init` with the same options is idempotent; with different options it builds the new judge first and retires the old one after, so `judge()` has no gap; a rejected `init` keeps the previous judge. `createJudge` stays exported for applications that need several judges at once, and the web playground uses it. The name is `init`, not `blasphemInit`: named ESM imports alias freely, and `init` is the convention of Sentry, i18next, and wasm-bindgen. The shared cases cover the singleton on Node (native and wasm) and in both browsers.

**napi WASI rejected for the browser.** `napi build --target wasm32-wasip1-threads` would let one crate serve native and wasm, but the threads target needs `SharedArrayBuffer`, Web Workers, and COOP/COEP headers on the embedding page (napi-rs v3 announcement), and both WASI targets add `@napi-rs/wasm-runtime` (0.74 MB unpacked) plus emnapi marshalling. The browser keeps wasm-bindgen; Node keeps the same wasm as its fallback, so no second wasm artifact exists. Binary size and call overhead of a WASI build are unmeasured; measuring needs `rustup target add wasm32-wasip1-threads`.

## Acceptance criteria

An EN-only browser session downloads the loader, the wasm, `manifest.json`, `en.pack`, and `en.detect`, and nothing else.

`createJudge({})` throws `BLASPHEM_LOCALES_EMPTY` before any request.

On a platform with its `@blasphem/node-*` package installed, `transport` is `"native"`. With that package removed, `transport` is `"wasm"` and every verdict matches.

A pack whose bytes differ from `manifest.json` throws `BLASPHEM_DIGEST_MISMATCH` at construction.

A judge built with `["en"]` returns `{ safe: true, score: 0, locale: null, grawlix: null }` for Spanish text.

`judge()` after `close()` throws `BLASPHEM_CLOSED`.

No published package depends on `packages/core`, and `@blasphem/react-native` does not depend on `blasphem`.
