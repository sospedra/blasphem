package blasphem

import "io/fs"

// Options configures one judge. Locales is required.
type Options struct {
	// Locales are embedded descriptors imported from locales/<code>.
	Locales []Locale
	// LocaleCodes selects codes from Assets or Packs for explicit filesystem sources.
	LocaleCodes []string
	// Assets holds manifest.json and packs selected by LocaleCodes. Ignored when Packs is set.
	Assets string
	// Packs serves manifest.json and the packs, for example an embed.FS. Takes precedence over Assets.
	Packs fs.FS
	// DisableDetection scores every loaded locale and reports the highest instead of routing by detected language.
	DisableDetection bool
	// Grawlix fills Judgement.Grawlix with masked text for unsafe verdicts.
	Grawlix bool
}
