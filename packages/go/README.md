# blasphem for Go

Local toxicity checks through the Rust engine and an embedded WebAssembly module.
[wazero](https://wazero.io/) runs the module.
Applications need Go 1.25 or later and no C compiler.

## Installation

```sh
go get github.com/sospedra/blasphem/packages/go
```

For changes from a local checkout, add a replacement in your application's `go.mod`:

```go
replace github.com/sospedra/blasphem/packages/go => /path/to/blasphem/packages/go
```

Prepare a directory with `manifest.json` and the required language files.
See [the pack guide](../packs/README.md#build-from-source).
Pass that directory through `Options.Assets`.

## Example

```go
package main

import (
	"fmt"
	"log"

	blasphem "github.com/sospedra/blasphem/packages/go"
)

func main() {
	err := blasphem.Init(blasphem.Options{
		Locales: []string{"en", "es"},
		Assets:  "/path/to/blasphem/packages/packs/dist",
		Grawlix: true,
	})
	if err != nil {
		log.Fatal(err)
	}
	defer blasphem.Close()

	verdict := blasphem.Judge("you are a stupid loser")
	fmt.Printf("%+v\n", verdict)
}
```

## API

| Function | Purpose |
| --- | --- |
| `Init(Options) error` | Build the package judge |
| `Judge(string) Judgement` | Check one message synchronously |
| `Ready() bool` | Report whether the package judge is initialized |
| `Close()` | Release the package judge |
| `New(Options) (*Instance, error)` | Build an independent judge |

Reuse a judge across messages.
Both package and instance methods support concurrent callers.
Calls within one instance run under a mutex.

Before `Init` and after `Close`, `Judge` returns a safe verdict.
A closed `Instance` also returns a safe verdict.

## Options

| Field | Default | Meaning |
| --- | --- | --- |
| `Locales` | Required | Supported locale codes |
| `Assets` | Empty | Directory containing the manifest and packs |
| `Packs` | `nil` | An `fs.FS` containing the same files |
| `DisableDetection` | `false` | Score every loaded locale |
| `Grawlix` | `false` | Return masked text |

`Packs` takes precedence over `Assets`.
Its root must contain `manifest.json`.
An `embed.FS` can include only the language files your application needs.

Use `id` for Indonesian and `ms` for Malay.
Both resolve to the `ms` model profile.
See [all locale codes](../packs/README.md#locales).

## Results and errors

`Judgement` contains `Safe bool`, `Score float64`, `Locale string`, and `Grawlix string`.
The score is ordinal, between 0 and 1.
An empty locale means the text did not route.
An empty grawlix means no masked text was requested.

`Init` and `New` return `*blasphem.Error` on construction failures.
Use `errors.As` and its `Code` field to inspect an error.
See [the error constants](errors.go) and [API source](blasphem.go).

## Development

Run the existing example from this package directory:

```sh
go run ./example ../packs/dist
```

Rebuild the embedded engine from the repository root after Rust changes:

```sh
cargo build --release --locked -p blasphem-ffi --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/blasphem_ffi.wasm packages/go/blasphem_ffi.wasm
```

Verify the package from `packages/go`:

```sh
env CGO_ENABLED=0 go test ./...
go vet ./...
```

[Contribute](../../CONTRIBUTING.md)

## License

Code uses [Apache-2.0](../../LICENSE).
Language data retains the terms recorded in [NOTICE](../../NOTICE).
