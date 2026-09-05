# blasphem for Go

Local toxicity checks through the Rust engine and an embedded WebAssembly module.
[wazero](https://wazero.io/) runs the module.
Applications need Go 1.25 or later and no C compiler.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

## Installation

```sh
go get github.com/sospedra/blasphem/packages/go/v2
```

For changes from a local checkout, add a replacement in your application's `go.mod`:

```go
replace github.com/sospedra/blasphem/packages/go/v2 => /path/to/blasphem/packages/go
```

Import locale descriptors from `locales/<code>`.
Each subpackage embeds its model, lexicon, and detection slice.
Only referenced locale assets enter the executable.
The module download contains every locale.
The shared WASM engine includes common rule code, without embedded locale models or detection payloads.
Use `locales/all` and `all.Locales` to include every release locale.

## Example

```go
package main

import (
	"fmt"
	"log"

	blasphem "github.com/sospedra/blasphem/packages/go/v2"
	"github.com/sospedra/blasphem/packages/go/v2/locales/en"
	"github.com/sospedra/blasphem/packages/go/v2/locales/es"
)

func main() {
	err := blasphem.Init(blasphem.Options{
		Locales: []blasphem.Locale{en.Locale, es.Locale},
		Grawlix: true,
	})
	if err != nil {
		log.Fatal(err)
	}
	defer blasphem.Close()

	verdict := blasphem.Judge("you are a stupid loser")
	if verdict.Grawlix != nil {
		fmt.Println(*verdict.Grawlix)
	}
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
| `Locales` | Required for embedded data | Typed locale descriptors |
| `LocaleCodes` | Required for filesystem data | Locale codes from `Assets` or `Packs` |
| `Assets` | Empty | Directory containing the manifest and packs |
| `Packs` | `nil` | An `fs.FS` containing the same files |
| `DisableDetection` | `false` | Score every loaded locale |
| `Grawlix` | `false` | Return masked text for unsafe verdicts |

For advanced filesystem sources, use `LocaleCodes: []string{"en", "es"}` with `Assets` or `Packs`.
Do not combine filesystem sources with embedded `Locales`.
`Packs` takes precedence over `Assets`.
Its root must contain `manifest.json`.
An `embed.FS` can include only the language files your application needs.

Use `id` for Indonesian and `ms` for Malay.
See [all 16 supported languages](../javascript-packs/README.md#locales).

## Results and errors

`Judgement` contains `Safe bool`, `Score float64`, `Locale string`, and `Grawlix *string`.
The score is ordinal, between 0 and 1.
An empty locale means the text did not route.
`Grawlix` contains masked text for unsafe verdicts when requested, otherwise `nil`.
Check `Grawlix != nil` before reading `*Grawlix`.

`Init` and `New` return `*blasphem.Error` on construction failures.
Use `errors.As` and its `Code` field to inspect an error.
See [the error constants](errors.go) and [API source](blasphem.go).

## Development

Run the existing example from this package directory:

```sh
go run ./example
```

Rebuild the embedded engine from the repository root after Rust changes:

```sh
node packages/go/scripts/build-wasm.mjs
node packages/go/scripts/build-locales.mjs
node packages/go/scripts/build-locales.mjs --check
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
