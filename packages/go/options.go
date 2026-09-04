package blasphem

import "io/fs"

// Options configures one judge. Locales is required.
type Options struct {
	// Locales are lowercase codes to load, such as "en", "es", "id" (Indonesian), and "ms" (Malay).
	Locales []string
	// Assets is the directory that holds manifest.json and the packs. Ignored when Packs is set.
	Assets string
	// Packs serves manifest.json and the packs, for example an embed.FS. Takes precedence over Assets.
	Packs fs.FS
	// DisableDetection scores every loaded locale and reports the highest instead of routing by detected language.
	DisableDetection bool
	// Grawlix fills Judgement.Grawlix with the masked text.
	Grawlix bool
}
