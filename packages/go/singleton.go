package blasphem

import (
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"sort"
	"strings"
	"sync"
)

var (
	singleton    sync.RWMutex
	current      *Instance
	currentKey   string
	initializing sync.Mutex
)

func optionsKey(options Options) string {
	locales := make([]string, 0, len(options.Locales))
	for _, locale := range options.Locales {
		locales = append(locales, fmt.Sprintf("%s:%x:%s:%x:%s",
			strings.ToLower(strings.TrimSpace(locale.Code)),
			sha256.Sum256(locale.Pack), locale.PackSHA256,
			sha256.Sum256(locale.Detect), locale.DetectSHA256))
	}
	for _, code := range options.LocaleCodes {
		locales = append(locales, strings.ToLower(strings.TrimSpace(code)))
	}
	sort.Strings(locales)
	key, _ := json.Marshal(struct {
		Locales  []string
		Assets   string
		HasPacks bool
		NoDetect bool
		Grawlix  bool
	}{locales, options.Assets, options.Packs != nil, options.DisableDetection, options.Grawlix})
	return string(key)
}

// Init loads the locales and installs the package judge. The same options
// again reuse it. Different options build a new judge first and retire the
// old one after, so Judge has no gap. A failed Init keeps the previous judge.
func Init(options Options) error {
	initializing.Lock()
	defer initializing.Unlock()
	key := optionsKey(options)
	singleton.RLock()
	same := current != nil && currentKey == key && options.Packs == nil && options.Assets == ""
	singleton.RUnlock()
	if same {
		return nil
	}
	instance, err := New(options)
	if err != nil {
		return err
	}
	singleton.Lock()
	previous := current
	current = instance
	currentKey = key
	singleton.Unlock()
	if previous != nil {
		previous.Close()
	}
	return nil
}

// Judge scores one message with the package judge. Before Init and after
// Close it returns the fail-open verdict. It never fails.
func Judge(text string) Judgement {
	singleton.RLock()
	instance := current
	singleton.RUnlock()
	if instance == nil {
		return failOpen()
	}
	return instance.Judge(text)
}

// Ready reports whether Init has installed a judge that Close has not released.
func Ready() bool {
	singleton.RLock()
	defer singleton.RUnlock()
	return current != nil
}

// Close releases the package judge. Judge fails open until the next Init.
func Close() {
	singleton.Lock()
	previous := current
	current = nil
	currentKey = ""
	singleton.Unlock()
	if previous != nil {
		previous.Close()
	}
}
