// Package blasphem is the multilingual pre-send toxicity nudge over the Rust
// core. The core is crates/blasphem-ffi compiled to WebAssembly and embedded
// in this package; wazero runs it. A build needs no C compiler, CGO_ENABLED=0
// works, and so does cross-compiling.
//
// Blasphem hashes word and character n-grams into sparse feature vectors.
// A linear classifier trained offline scores them with 16-bit weights.
// Lexicons and context rules contribute to the verdict.
// Detection runs locally without neural networks or cloud inference.
//
// The contract matches the JavaScript package: Init once with Options, then
// Judge on every message; Judge never fails and fails open before Init or
// after Close. New builds an independent Instance when one judge per module
// is not enough.
package blasphem

import (
	"context"
	"runtime"
	"sync"
)

// Judgement is one verdict for one message.
type Judgement struct {
	// Safe is true when no nudge is due. Unroutable text is safe; the nudge fails open.
	Safe bool
	// Score is ordinal risk from 0 through 1. Not a probability.
	Score float64
	// Locale is the lowercase code that produced the score, or "" when nothing routed the text.
	Locale string
	// Grawlix contains masked text for unsafe verdicts when requested, otherwise nil.
	Grawlix *string
}

func failOpen() Judgement {
	return Judgement{Safe: true}
}

// Instance is one judge over a fixed set of locales. It is safe for
// concurrent use: one mutex serializes calls into its engine. Build it with
// New; release it with Close.
type Instance struct {
	mu      sync.Mutex
	engine  *module // nil after Close
	handle  uint32  // the blasphem_engine inside the engine's memory
	verdict region  // one blasphem_judgement the engine writes into
	text    region  // the message under judgement, NUL-terminated
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
	engine, err := instantiate(context.Background())
	if err != nil {
		return nil, nativeError("the engine could not start", err)
	}
	instance, err := build(engine, options, sources)
	if err != nil {
		engine.close()
		return nil, err
	}
	runtime.SetFinalizer(instance, (*Instance).Close)
	return instance, nil
}

// build feeds the sources to a builder inside the engine and wraps the result.
func build(engine *module, options Options, sources []source) (*Instance, error) {
	builder, err := engine.call("blasphem_builder_new", boolArg(!options.DisableDetection), boolArg(options.Grawlix))
	if err != nil || builder == 0 {
		return nil, nativeError("the native builder could not be created", err)
	}
	for _, entry := range sources {
		if err := add(engine, uint32(builder), entry); err != nil {
			_, _ = engine.call("blasphem_builder_free", builder)
			return nil, err
		}
	}
	handle, err := engine.call("blasphem_builder_build", builder)
	if err != nil || handle == 0 {
		failure := builderError(engine, uint32(builder), err)
		_, _ = engine.call("blasphem_builder_free", builder)
		return nil, failure
	}
	return newInstance(engine, uint32(handle))
}

// newInstance reserves the verdict buffer every judgement reuses and lists the locales.
func newInstance(engine *module, handle uint32) (*Instance, error) {
	verdict, err := engine.reserve(judgementSize)
	if err != nil {
		return nil, nativeError("the engine could not reserve memory", err)
	}
	locales, err := engineLocales(engine, handle)
	if err != nil {
		return nil, nativeError("the engine could not list its locales", err)
	}
	return &Instance{engine: engine, handle: handle, verdict: verdict, locales: locales}, nil
}

// add stages one locale's files inside the engine and registers them.
func add(engine *module, builder uint32, entry source) error {
	if len(entry.pack) == 0 {
		return &Error{Code: CodePackInvalid, Message: entry.locale + ".pack is empty"}
	}
	var staged staging
	defer staged.release(engine)
	locale := staged.text(engine, entry.locale)
	pack := staged.bytes(engine, entry.pack)
	packSha := staged.text(engine, entry.packSha256)
	var detect, detectSha region
	if len(entry.detect) > 0 {
		detect = staged.bytes(engine, entry.detect)
		detectSha = staged.text(engine, entry.detectSha256)
	}
	if staged.err != nil {
		return nativeError("the engine could not reserve memory", staged.err)
	}
	status, err := engine.call("blasphem_builder_add", uint64(builder),
		uint64(locale.ptr), uint64(pack.ptr), uint64(pack.size), uint64(packSha.ptr),
		uint64(detect.ptr), uint64(detect.size), uint64(detectSha.ptr))
	if err != nil || status != 0 {
		return builderError(engine, builder, err)
	}
	return nil
}

func engineLocales(engine *module, handle uint32) ([]string, error) {
	count, err := engine.call("blasphem_engine_locale_count", uint64(handle))
	if err != nil {
		return nil, err
	}
	locales := make([]string, 0, count)
	for index := uint64(0); index < count; index++ {
		ptr, err := engine.call("blasphem_engine_locale", uint64(handle), index)
		if err != nil {
			return nil, err
		}
		if ptr == 0 {
			continue
		}
		code, err := engine.takeText(uint32(ptr))
		if err != nil {
			return nil, err
		}
		locales = append(locales, code)
	}
	return locales, nil
}

// Judge scores one message. It never fails; after Close it fails open.
func (i *Instance) Judge(text string) Judgement {
	i.mu.Lock()
	defer i.mu.Unlock()
	if i.engine == nil {
		return failOpen()
	}
	verdict, err := i.judge(text)
	if err != nil {
		return failOpen()
	}
	return verdict
}

// judge runs one message through the engine. The caller holds the mutex.
func (i *Instance) judge(text string) (Judgement, error) {
	if err := i.stageMessage(text); err != nil {
		return Judgement{}, err
	}
	engine := i.engine
	_, err := engine.judge.Call(engine.ctx, uint64(i.verdict.ptr), uint64(i.handle), uint64(i.text.ptr))
	if err != nil {
		return Judgement{}, err
	}
	return engine.readJudgement(i.verdict.ptr)
}

// stageMessage writes text and its NUL into the reusable buffer, growing it when needed.
func (i *Instance) stageMessage(text string) error {
	needed := uint32(len(text) + 1)
	if needed > i.text.size {
		i.engine.release(i.text)
		i.text = region{}
		staged, err := i.engine.reserve(max(needed, 2*i.text.size))
		if err != nil {
			return err
		}
		i.text = staged
	}
	memory := i.engine.memory
	if !memory.WriteString(i.text.ptr, text) || !memory.WriteByte(i.text.ptr+uint32(len(text)), 0) {
		return &Error{Code: CodePackInvalid, Message: "the engine refused a memory write"}
	}
	return nil
}

// Locales are the loaded codes in registry order.
func (i *Instance) Locales() []string {
	return append([]string(nil), i.locales...)
}

// Close releases the engine and its memory. Judge fails open afterwards. Safe to call twice.
func (i *Instance) Close() {
	i.mu.Lock()
	defer i.mu.Unlock()
	if i.engine == nil {
		return
	}
	i.engine.close()
	i.engine = nil
}

func boolArg(value bool) uint64 {
	if value {
		return 1
	}
	return 0
}

// nativeError wraps a failure inside the engine or its runtime.
func nativeError(message string, err error) *Error {
	if err != nil {
		message += ": " + err.Error()
	}
	return &Error{Code: CodePackInvalid, Message: message}
}

// builderError reads the failure the engine recorded for builder, or reports err.
func builderError(engine *module, builder uint32, err error) error {
	if err != nil {
		return nativeError("the engine failed", err)
	}
	ptr, _ := engine.call("blasphem_builder_error", uint64(builder))
	if ptr == 0 {
		ptr, _ = engine.call("blasphem_last_error")
	}
	if ptr == 0 {
		return &Error{Code: CodePackInvalid, Message: "unknown native error"}
	}
	message, readErr := engine.cString(uint32(ptr))
	if readErr != nil {
		return nativeError("the engine reported an unreadable error", readErr)
	}
	return parseError(message)
}
