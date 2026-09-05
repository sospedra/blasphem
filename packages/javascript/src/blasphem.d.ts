/* tslint:disable */
/* eslint-disable */

/** One verdict for one message. Safe verdicts never include masked text. */
export type Judgement =
| { safe: true; score: number; locale: string | null; grawlix: null }
| { safe: false; score: number; locale: string | null; grawlix: string | null };



/**
 * The browser-facing engine. `judge` returns a plain object, so callers never free it.
 */
export class BlasphemEngine {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Scores one message and returns `{ safe, score, locale, grawlix }`.
     *
     * # Errors
     *
     * Returns an error only when the host rejects a property write.
     */
    judge(text: string): Judgement;
    /**
     * The loaded locales as lowercase codes.
     */
    readonly locales: string[];
}

/**
 * Collects one locale at a time, then builds the engine.
 */
export class BlasphemEngineBuilder {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Adds one locale's pack, and its detect slice when detection is on.
     *
     * # Errors
     *
     * Returns `BLASPHEM_LOCALE_UNSUPPORTED` or `BLASPHEM_PACK_INVALID` text.
     */
    add(locale: string, pack: Uint8Array, pack_sha256?: string | null, detect?: Uint8Array | null, detect_sha256?: string | null): void;
    /**
     * Verifies every digest, parses every pack, and returns the engine.
     * The builder is consumed.
     *
     * # Errors
     *
     * Returns the first failure, message prefixed by its contract code.
     */
    build(): BlasphemEngine;
    constructor(detect_language: boolean, grawlix: boolean);
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_blasphemengine_free: (a: number, b: number) => void;
    readonly __wbg_blasphemenginebuilder_free: (a: number, b: number) => void;
    readonly blasphemengine_judge: (a: number, b: number, c: number, d: number) => void;
    readonly blasphemengine_locales: (a: number, b: number) => void;
    readonly blasphemenginebuilder_add: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number, j: number, k: number, l: number) => void;
    readonly blasphemenginebuilder_build: (a: number, b: number) => void;
    readonly blasphemenginebuilder_new: (a: number, b: number) => number;
    readonly __wbindgen_export: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
