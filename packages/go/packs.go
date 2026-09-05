package blasphem

import (
	"encoding/json"
	"io/fs"
	"os"
	"regexp"
	"sort"
	"strings"
)

const manifestFormatVersion = 1

var hex64 = regexp.MustCompile(`^[0-9a-f]{64}$`)

type manifest struct {
	FormatVersion int `json:"formatVersion"`
	Files         map[string]struct {
		Bytes  int    `json:"bytes"`
		Sha256 string `json:"sha256"`
	} `json:"files"`
}

type source struct {
	locale       string
	pack         []byte
	packSha256   string
	detect       []byte
	detectSha256 string
}

var (
	canonical = map[string]string{}
	order     = map[string]int{}
)

func init() {
	for index, entry := range locales {
		canonical[entry.code] = entry.code
		order[entry.code] = index
		for _, alias := range entry.aliases {
			canonical[alias] = entry.code
		}
	}
}

// normalizeLocales lowercases, resolves aliases, rejects unknown codes, and
// returns registry order without repeats.
func normalizeLocales(requested []string) ([]string, error) {
	if len(requested) == 0 {
		return nil, &Error{Code: CodeLocalesEmpty, Message: "set Options.Locales to at least one code, such as \"en\""}
	}
	seen := map[string]bool{}
	var codes []string
	for _, raw := range requested {
		code, ok := canonical[strings.ToLower(strings.TrimSpace(raw))]
		if !ok {
			return nil, &Error{Code: CodeLocaleUnsupported, Message: "unsupported locale " + strconvQuote(raw)}
		}
		if !seen[code] {
			seen[code] = true
			codes = append(codes, code)
		}
	}
	sort.Slice(codes, func(left, right int) bool { return order[codes[left]] < order[codes[right]] })
	return codes, nil
}

func strconvQuote(value string) string {
	quoted, _ := json.Marshal(value)
	return string(quoted)
}

func openPacks(options Options) (fs.FS, error) {
	if options.Packs != nil {
		return options.Packs, nil
	}
	if strings.TrimSpace(options.Assets) != "" {
		return os.DirFS(options.Assets), nil
	}
	return nil, &Error{Code: CodeAssetsRequired, Message: "set Options.Assets to the packs directory or Options.Packs to an fs.FS"}
}

func readManifest(packs fs.FS) (*manifest, error) {
	bytes, err := fs.ReadFile(packs, "manifest.json")
	if err != nil {
		return nil, &Error{Code: CodeFetchFailed, Message: "manifest.json: " + err.Error()}
	}
	var parsed manifest
	if err := json.Unmarshal(bytes, &parsed); err != nil {
		return nil, &Error{Code: CodePackInvalid, Message: "manifest.json is not valid JSON: " + err.Error()}
	}
	if parsed.FormatVersion != manifestFormatVersion {
		return nil, &Error{Code: CodeFormatVersion, Message: "manifest.json has format version " + itoa(parsed.FormatVersion) + ", this build accepts 1"}
	}
	for name, file := range parsed.Files {
		if !hex64.MatchString(file.Sha256) {
			return nil, &Error{Code: CodePackInvalid, Message: "manifest.json entry " + strconvQuote(name) + " needs a 64-character sha256"}
		}
	}
	return &parsed, nil
}

func itoa(value int) string {
	text, _ := json.Marshal(value)
	return string(text)
}

func readFile(packs fs.FS, parsed *manifest, name, locale string) ([]byte, string, error) {
	file, ok := parsed.Files[name]
	if !ok {
		return nil, "", &Error{Code: CodeLocaleMissing, Message: "manifest.json lists no " + name + "; the packs do not include " + locale}
	}
	bytes, err := fs.ReadFile(packs, name)
	if err != nil {
		return nil, "", &Error{Code: CodeFetchFailed, Message: name + ": " + err.Error()}
	}
	return bytes, file.Sha256, nil
}

// loadSources reads every file a judge needs. Digests travel with the bytes;
// the native side verifies them before it parses anything.
func loadSources(options Options) ([]source, error) {
	if len(options.Locales) != 0 {
		return embeddedSources(options)
	}
	codes, err := normalizeLocales(options.LocaleCodes)
	if err != nil {
		return nil, err
	}
	packs, err := openPacks(options)
	if err != nil {
		return nil, err
	}
	parsed, err := readManifest(packs)
	if err != nil {
		return nil, err
	}
	sources := make([]source, 0, len(codes))
	for _, code := range codes {
		entry := source{locale: code}
		if entry.pack, entry.packSha256, err = readFile(packs, parsed, code+".pack", code); err != nil {
			return nil, err
		}
		if !options.DisableDetection {
			if entry.detect, entry.detectSha256, err = readFile(packs, parsed, code+".detect", code); err != nil {
				return nil, err
			}
		}
		sources = append(sources, entry)
	}
	return sources, nil
}
