// Package blasphem is the multilingual pre-send toxicity nudge over the Rust
// core, through the C ABI in crates/blasphem-ffi.
//
// The contract matches the JavaScript package: Init once with Options, then
// Judge on every message; Judge never fails and fails open before Init or
// after Close. New builds an independent Instance when one judge per module
// is not enough.
//
// Building needs the FFI static library: run `cargo build --release -p
// blasphem-ffi` at the repository root first. The cgo directives below find
// the header and the archive relative to this file.
package blasphem

/*
#cgo CFLAGS: -I${SRCDIR}/../../crates/blasphem-ffi/include
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lblasphem_ffi
#include <stdlib.h>
#include "blasphem.h"
*/
import "C"

import (
	"runtime"
	"sync"
	"unsafe"
)

// Judgement is one verdict for one message.
type Judgement struct {
	// Safe is true when no nudge is due. Unroutable text is safe; the nudge fails open.
	Safe bool
	// Score is ordinal risk from 0 through 1. Not a probability.
	Score float64
	// Locale is the lowercase code that produced the score, or "" when nothing routed the text.
	Locale string
	// Grawlix is the masked text when Options.Grawlix is set, otherwise "".
	Grawlix string
}

func failOpen() Judgement {
	return Judgement{Safe: true}
}

// Instance is one judge over a fixed set of locales. It is safe for
// concurrent use. Build it with New; release it with Close.
type Instance struct {
	mu      sync.RWMutex
	engine  *C.blasphem_engine
	locales []string
}

// New loads the requested locales and builds a judge.
//
// Errors are *Error values whose Code names the failure, such as
// BLASPHEM_LOCALE_UNSUPPORTED or BLASPHEM_DIGEST_MISMATCH.
func New(options Options) (*Instance, error) {
	sources, err := loadSources(options)
	if err != nil {
		return nil, err
	}
	builder := C.blasphem_builder_new(C.bool(!options.DisableDetection), C.bool(options.Grawlix))
	if builder == nil {
		return nil, &Error{Code: CodePackInvalid, Message: "the native builder could not be created"}
	}
	for _, source := range sources {
		if err := add(builder, source); err != nil {
			C.blasphem_builder_free(builder)
			return nil, err
		}
	}
	engine := C.blasphem_builder_build(builder)
	if engine == nil {
		err := builderError(builder)
		C.blasphem_builder_free(builder)
		return nil, err
	}
	instance := &Instance{engine: engine, locales: engineLocales(engine)}
	runtime.SetFinalizer(instance, (*Instance).Close)
	return instance, nil
}

func add(builder *C.blasphem_builder, source source) error {
	if len(source.pack) == 0 {
		return &Error{Code: CodePackInvalid, Message: source.locale + ".pack is empty"}
	}
	locale := C.CString(source.locale)
	defer C.free(unsafe.Pointer(locale))
	packSha := C.CString(source.packSha256)
	defer C.free(unsafe.Pointer(packSha))
	var detect *C.uint8_t
	var detectLen C.size_t
	var detectSha *C.char
	if len(source.detect) > 0 {
		detect = (*C.uint8_t)(unsafe.Pointer(&source.detect[0]))
		detectLen = C.size_t(len(source.detect))
		detectSha = C.CString(source.detectSha256)
		defer C.free(unsafe.Pointer(detectSha))
	}
	status := C.blasphem_builder_add(builder, locale,
		(*C.uint8_t)(unsafe.Pointer(&source.pack[0])), C.size_t(len(source.pack)), packSha,
		detect, detectLen, detectSha)
	if status != 0 {
		return builderError(builder)
	}
	return nil
}

func engineLocales(engine *C.blasphem_engine) []string {
	count := int(C.blasphem_engine_locale_count(engine))
	locales := make([]string, 0, count)
	for index := 0; index < count; index++ {
		code := C.blasphem_engine_locale(engine, C.size_t(index))
		if code != nil {
			locales = append(locales, C.GoString(code))
			C.blasphem_text_free(code)
		}
	}
	return locales
}

// Judge scores one message. It never fails; after Close it fails open.
func (i *Instance) Judge(text string) Judgement {
	i.mu.RLock()
	defer i.mu.RUnlock()
	if i.engine == nil {
		return failOpen()
	}
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))
	verdict := C.blasphem_engine_judge(i.engine, cText)
	defer C.blasphem_judgement_free(verdict)
	return Judgement{
		Safe:    bool(verdict.safe),
		Score:   float64(verdict.score),
		Locale:  optionalText(verdict.locale),
		Grawlix: optionalText(verdict.grawlix),
	}
}

// Locales are the loaded codes in registry order.
func (i *Instance) Locales() []string {
	return append([]string(nil), i.locales...)
}

// Close releases the packs. Judge fails open afterwards. Safe to call twice.
func (i *Instance) Close() {
	i.mu.Lock()
	defer i.mu.Unlock()
	if i.engine != nil {
		C.blasphem_engine_free(i.engine)
		i.engine = nil
	}
}

func optionalText(text *C.char) string {
	if text == nil {
		return ""
	}
	return C.GoString(text)
}

func builderError(builder *C.blasphem_builder) error {
	message := C.blasphem_builder_error(builder)
	if message == nil {
		message = C.blasphem_last_error()
	}
	if message == nil {
		return &Error{Code: CodePackInvalid, Message: "unknown native error"}
	}
	return parseError(C.GoString(message))
}
