# blasphem (Go)

Multilingual pre-send toxicity nudge over the Rust core. The core is
`crates/blasphem-ffi` compiled to WebAssembly and embedded in the module;
[wazero](https://wazero.io) runs it. No cgo, no C compiler, `CGO_ENABLED=0`
builds, and cross-compiling works. Go 1.25 or later. Same contract as the
JavaScript package.

```go
import blasphem "github.com/sospedra/blasphem/packages/go"

err := blasphem.Init(blasphem.Options{Locales: []string{"en", "es"}, Assets: "/srv/blasphem-packs", Grawlix: true})
verdict := blasphem.Judge("you are a stupid loser")
// {Safe:false Score:0.64 Locale:en Grawlix:you are a @#$%&! loser}
```

`Init` loads the locales once and installs the package judge. `Judge` is
synchronous and never fails: before `Init` and after `Close` it returns the
fail-open verdict. `Ready` tells which. `Init` with the same options is free;
with other options it builds a new judge first and retires the old one after.

`New(Options)` returns an `*Instance` for several judges at once. Both are safe
for concurrent use.

## Packs

`Options.Assets` names the directory with `manifest.json` and the `.pack` and
`.detect` files. `Options.Packs` takes any `fs.FS` instead, so an app can
`//go:embed` exactly the locales it ships. Every file is verified against the
manifest before it parses.

`Options.DisableDetection` scores every loaded locale and reports the highest.
Detection is on by default.

## Errors

`Init` and `New` return `*blasphem.Error`; `Code` is one of `CodeLocalesEmpty`,
`CodeLocaleUnsupported`, `CodeLocaleMissing`, `CodeAssetsRequired`,
`CodeFetchFailed`, `CodeDigestMismatch`, `CodeFormatVersion`, `CodePackInvalid`.

## Engine

`blasphem_ffi.wasm` is the compiled core, committed next to the Go files. It
carries no packs. Every `Instance` runs its own copy behind one mutex. The
first `Instance` in a process spends about 150 ms turning the engine into
machine code; the ones after start in under a millisecond.

Rebuild the file after a Rust change, at the repository root:

```bash
cargo build --release --locked -p blasphem-ffi --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/blasphem_ffi.wasm packages/go/
```

CI runs the same build and fails when the bytes differ from the committed file.

## Try

```bash
cd packages/go && go run ./example ../../packages/packs/dist
```
