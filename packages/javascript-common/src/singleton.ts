import type { Judge, InitOptions, Judgement } from "./contract.js";

/** The verdict for text nothing judges: the nudge fails open. */
export function failOpen(): Judgement {
  return { safe: true, score: 0, locale: null, grawlix: null };
}

/** One judge per module: `init` once, then call `judge` on every keystroke. */
export interface Singleton {
  /**
   * Loads the locales and installs the judge. Same options again reuse the
   * judge; different options build a new one and retire the old one only
   * after the new one is ready, so `judge` never has a gap. Rejects with the
   * coded errors of `createJudge` and leaves the previous judge in place.
   */
  init(options?: InitOptions): Promise<void>;
  /** Synchronous. Before `init` resolves, or after `close`, returns the fail-open verdict. Never throws. */
  judge(text: string): Judgement;
  /** True once `init` has resolved and until `close`. */
  ready(): boolean;
  /** Releases the packs. `judge` fails open until the next `init`. */
  close(): void;
}

function keyOf(options: InitOptions): string {
  const locales = Array.isArray(options.locales) ? [...options.locales].map(String).sort() : options.locales;
  return JSON.stringify({ locales, assets: options.assets ?? null, detectLanguage: options.detectLanguage ?? null, grawlix: options.grawlix ?? false });
}

/** Wraps a runtime's `createJudge` in module state. Every function is a closure, so callers may destructure them. */
export function createSingleton(create: (options: InitOptions) => Promise<Judge>): Singleton {
  let current: Judge | null = null;
  let currentKey: string | null = null;
  let pending: { key: string; promise: Promise<void> } | null = null;

  const init = (options: InitOptions = {}): Promise<void> => {
    const key = keyOf(options);
    if (current !== null && currentKey === key) {
      pending = null;
      return Promise.resolve();
    }
    if (pending !== null && pending.key === key) return pending.promise;
    const promise: Promise<void> = Promise.resolve().then(() => create(options)).then(
      (judge) => {
        // A newer init or a close superseded this one.
        if (pending?.promise !== promise) {
          judge.close();
          return;
        }
        const previous = current;
        current = judge;
        currentKey = key;
        pending = null;
        previous?.close();
      },
      (error: unknown) => {
        if (pending?.promise === promise) pending = null;
        throw error;
      },
    );
    pending = { key, promise };
    return promise;
  };

  const judge = (text: string): Judgement => (current === null ? failOpen() : current.judge(text));
  const ready = (): boolean => current !== null;
  const close = (): void => {
    pending = null;
    const previous = current;
    current = null;
    currentKey = null;
    previous?.close();
  };

  return { init, judge, ready, close };
}
