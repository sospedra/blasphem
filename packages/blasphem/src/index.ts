import { load } from "./load.js";

// Top-level await means an importer of this module waits for the
// WebAssembly module before its own body runs. That is what lets judge()
// stay synchronous at the call site.
await load();

export { judge } from "./judge.js";
export type { JudgeOptions, Judgement } from "./judge.js";
export { load } from "./load.js";
