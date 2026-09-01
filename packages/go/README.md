# blasphem (Go)

Multilingual pre-send toxicity nudge over the Rust core, through cgo and the C
ABI in `crates/blasphem-ffi`. Same contract as the JavaScript package.

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

## Build and try

```bash
cargo build --release -p blasphem-ffi        # repository root
cd packages/go && go run ./example ../../packages/packs/dist
```

The cgo directives find the header and `libblasphem_ffi.a` under `target/release`
relative to this directory. A published module vendors one archive per platform
and points `#cgo LDFLAGS` at it.
