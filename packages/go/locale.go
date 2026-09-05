package blasphem

// Locale describes one embedded locale. Import descriptors from locales/<code>.
// The root package embeds no locale assets.
type Locale struct {
	Code         string
	Pack         []byte
	PackSHA256   string
	Detect       []byte
	DetectSHA256 string
}

func embeddedSources(options Options) ([]source, error) {
	if len(options.LocaleCodes) != 0 || options.Assets != "" || options.Packs != nil {
		return nil, &Error{Code: CodePackInvalid, Message: "do not combine embedded Locales with filesystem sources"}
	}
	requested := make([]string, 0, len(options.Locales))
	entries := make(map[string]Locale, len(options.Locales))
	for _, locale := range options.Locales {
		codes, err := normalizeLocales([]string{locale.Code})
		if err != nil {
			return nil, err
		}
		code := codes[0]
		if _, exists := entries[code]; exists {
			return nil, &Error{Code: CodePackInvalid, Message: "duplicate locale " + code}
		}
		entries[code] = locale
		requested = append(requested, code)
	}
	codes, err := normalizeLocales(requested)
	if err != nil {
		return nil, err
	}
	result := make([]source, 0, len(codes))
	for _, code := range codes {
		locale := entries[code]
		if !hex64.MatchString(locale.PackSHA256) {
			return nil, &Error{Code: CodePackInvalid, Message: code + ".pack needs a 64-character sha256"}
		}
		entry := source{locale: code, pack: locale.Pack, packSha256: locale.PackSHA256}
		if !options.DisableDetection {
			if !hex64.MatchString(locale.DetectSHA256) || len(locale.Detect) == 0 {
				return nil, &Error{Code: CodePackInvalid, Message: code + ".detect is required"}
			}
			entry.detect, entry.detectSha256 = locale.Detect, locale.DetectSHA256
		}
		result = append(result, entry)
	}
	return result, nil
}
