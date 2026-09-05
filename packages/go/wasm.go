package blasphem

import (
	"bytes"
	"context"
	_ "embed"
	"encoding/binary"
	"errors"
	"fmt"
	"math"
	"sync"
	"sync/atomic"

	"github.com/tetratelabs/wazero"
	"github.com/tetratelabs/wazero/api"
)

// engineWasm is crates/blasphem-ffi compiled for wasm32-unknown-unknown. It
// carries no packs. Rebuild it at the repository root after a Rust change:
//
//	node packages/go/scripts/build-wasm.mjs
//
// CI rebuilds it and fails when the bytes differ from this file.
//
//go:embed blasphem_ffi.wasm
var engineWasm []byte

// Layout of blasphem_judgement on wasm32: bool at 0, f64 at 8, then two
// 32-bit pointers. The wasm32 C ABI returns the struct through a pointer the
// caller passes as the first argument.
const (
	judgementSize    = 24
	judgementScore   = 8
	judgementLocale  = 16
	judgementGrawlix = 20
)

var (
	compileOnce  sync.Once
	compiledIn   wazero.Runtime
	compiledCode wazero.CompiledModule
	compileError error
	moduleSerial atomic.Uint64
)

// compile turns the engine into machine code once per process. The first call
// costs about 150 ms; every Instance after that starts in under a millisecond.
func compile(ctx context.Context) (wazero.Runtime, wazero.CompiledModule, error) {
	compileOnce.Do(func() {
		compiledIn = wazero.NewRuntime(ctx)
		compiledCode, compileError = compiledIn.CompileModule(ctx, engineWasm)
	})
	return compiledIn, compiledCode, compileError
}

// region is memory inside the engine that blasphem_alloc handed out.
type region struct{ ptr, size uint32 }

// module is one running engine with its own linear memory. Its calls are not
// safe for concurrent use; Instance serializes them behind a mutex.
type module struct {
	ctx      context.Context
	instance api.Module
	memory   api.Memory
	alloc    api.Function
	free     api.Function
	judge    api.Function
	textFree api.Function
}

func instantiate(ctx context.Context) (*module, error) {
	runtime, code, err := compile(ctx)
	if err != nil {
		return nil, err
	}
	name := fmt.Sprintf("blasphem-%d", moduleSerial.Add(1))
	instance, err := runtime.InstantiateModule(ctx, code, wazero.NewModuleConfig().WithName(name))
	if err != nil {
		return nil, err
	}
	m := &module{
		ctx:      ctx,
		instance: instance,
		memory:   instance.Memory(),
		alloc:    instance.ExportedFunction("blasphem_alloc"),
		free:     instance.ExportedFunction("blasphem_free"),
		judge:    instance.ExportedFunction("blasphem_engine_judge"),
		textFree: instance.ExportedFunction("blasphem_text_free"),
	}
	if err := m.checkABI(); err != nil {
		m.close()
		return nil, err
	}
	return m, nil
}

// checkABI confirms the exports this file calls directly exist and that judge
// returns its struct through a pointer parameter, as the wasm32 C ABI does.
func (m *module) checkABI() error {
	exports := []struct {
		name string
		fn   api.Function
	}{
		{"blasphem_alloc", m.alloc},
		{"blasphem_free", m.free},
		{"blasphem_engine_judge", m.judge},
		{"blasphem_text_free", m.textFree},
	}
	for _, export := range exports {
		if export.fn == nil {
			return errors.New("the engine does not export " + export.name)
		}
	}
	definition := m.judge.Definition()
	if len(definition.ParamTypes()) != 3 || len(definition.ResultTypes()) != 0 {
		return errors.New("the engine's judge export does not follow the wasm32 C ABI")
	}
	return nil
}

func (m *module) close() {
	_ = m.instance.Close(m.ctx)
}

// call runs an export by name and returns its first result, or 0 without one.
func (m *module) call(name string, args ...uint64) (uint64, error) {
	fn := m.instance.ExportedFunction(name)
	if fn == nil {
		return 0, errors.New("the engine does not export " + name)
	}
	return first(fn.Call(m.ctx, args...))
}

func first(results []uint64, err error) (uint64, error) {
	if err != nil || len(results) == 0 {
		return 0, err
	}
	return results[0], nil
}

// reserve asks the engine for size bytes. Zero bytes reserve the null pointer.
func (m *module) reserve(size uint32) (region, error) {
	if size == 0 {
		return region{}, nil
	}
	ptr, err := first(m.alloc.Call(m.ctx, uint64(size)))
	if err != nil {
		return region{}, err
	}
	if ptr == 0 {
		return region{}, errors.New("the engine is out of memory")
	}
	return region{ptr: uint32(ptr), size: size}, nil
}

// stage copies data into memory the engine reserves for it.
func (m *module) stage(data []byte) (region, error) {
	staged, err := m.reserve(uint32(len(data)))
	if err != nil || staged.ptr == 0 {
		return staged, err
	}
	if !m.memory.Write(staged.ptr, data) {
		m.release(staged)
		return region{}, errors.New("the engine refused a memory write")
	}
	return staged, nil
}

// release returns a region to the engine. The null region is a no-op.
func (m *module) release(staged region) {
	if staged.ptr != 0 {
		_, _ = m.free.Call(m.ctx, uint64(staged.ptr), uint64(staged.size))
	}
}

// cString copies the NUL-terminated string at ptr out of engine memory.
func (m *module) cString(ptr uint32) (string, error) {
	size := m.memory.Size()
	if ptr == 0 || ptr >= size {
		return "", errors.New("the engine returned a bad string pointer")
	}
	var out []byte
	for ptr < size {
		chunk := min(uint32(256), size-ptr)
		view, ok := m.memory.Read(ptr, chunk)
		if !ok {
			return "", errors.New("the engine refused a memory read")
		}
		if end := bytes.IndexByte(view, 0); end >= 0 {
			return string(append(out, view[:end]...)), nil
		}
		out = append(out, view...)
		ptr += chunk
	}
	return "", errors.New("the engine returned an unterminated string")
}

// takeText copies a string the engine handed over and frees it. Null reads as "".
func (m *module) takeText(ptr uint32) (string, error) {
	if ptr == 0 {
		return "", nil
	}
	text, err := m.cString(ptr)
	_, _ = m.textFree.Call(m.ctx, uint64(ptr))
	return text, err
}

// takeOptionalText preserves the difference between a null pointer and an empty string.
func (m *module) takeOptionalText(ptr uint32) (*string, error) {
	if ptr == 0 {
		return nil, nil
	}
	text, err := m.takeText(ptr)
	if err != nil {
		return nil, err
	}
	return &text, nil
}

// readJudgement decodes the struct the engine wrote at ptr and frees its strings.
func (m *module) readJudgement(ptr uint32) (Judgement, error) {
	view, ok := m.memory.Read(ptr, judgementSize)
	if !ok {
		return Judgement{}, errors.New("the engine refused a memory read")
	}
	verdict := Judgement{
		Safe:  view[0] != 0,
		Score: math.Float64frombits(binary.LittleEndian.Uint64(view[judgementScore:])),
	}
	localePtr := binary.LittleEndian.Uint32(view[judgementLocale:])
	grawlixPtr := binary.LittleEndian.Uint32(view[judgementGrawlix:])
	var err error
	if verdict.Locale, err = m.takeText(localePtr); err != nil {
		return Judgement{}, err
	}
	if verdict.Grawlix, err = m.takeOptionalText(grawlixPtr); err != nil {
		return Judgement{}, err
	}
	return verdict, nil
}

// staging tracks the regions one call borrows so one release returns them all.
type staging struct {
	regions []region
	err     error
}

func (s *staging) bytes(engine *module, data []byte) region {
	if s.err != nil {
		return region{}
	}
	staged, err := engine.stage(data)
	s.err = err
	s.regions = append(s.regions, staged)
	return staged
}

// text stages a NUL-terminated copy.
func (s *staging) text(engine *module, text string) region {
	buf := make([]byte, len(text)+1)
	copy(buf, text)
	return s.bytes(engine, buf)
}

func (s *staging) release(engine *module) {
	for _, staged := range s.regions {
		engine.release(staged)
	}
}
