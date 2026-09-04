export type ErrorCode =
  | "BLASPHEM_LOCALES_EMPTY"
  | "BLASPHEM_LOCALE_UNSUPPORTED"
  | "BLASPHEM_LOCALE_MISSING"
  | "BLASPHEM_ASSETS_REQUIRED"
  | "BLASPHEM_FETCH_FAILED"
  | "BLASPHEM_DIGEST_MISMATCH"
  | "BLASPHEM_FORMAT_VERSION"
  | "BLASPHEM_PACK_INVALID"
  | "BLASPHEM_CLOSED";

export type CodedError = Error & { code: ErrorCode };

const CODES: readonly ErrorCode[] = [
  "BLASPHEM_LOCALES_EMPTY",
  "BLASPHEM_LOCALE_UNSUPPORTED",
  "BLASPHEM_LOCALE_MISSING",
  "BLASPHEM_ASSETS_REQUIRED",
  "BLASPHEM_FETCH_FAILED",
  "BLASPHEM_DIGEST_MISMATCH",
  "BLASPHEM_FORMAT_VERSION",
  "BLASPHEM_PACK_INVALID",
  "BLASPHEM_CLOSED",
];

/** A plain Error with a `code`. No subclass: the core is inlined per package, so `instanceof` could never hold across packages. */
export function fail(code: ErrorCode, message: string): CodedError {
  const error = new Error(`${code}: ${message}`) as CodedError;
  error.code = code;
  return error;
}

function isCode(value: string): value is ErrorCode {
  return (CODES as readonly string[]).includes(value);
}

/** Engines report `CODE: detail`. Anything else is a malformed pack. */
export function fromEngineError(error: unknown): CodedError {
  const text = error instanceof Error ? error.message : String(error);
  const separator = text.indexOf(": ");
  const head = separator === -1 ? "" : text.slice(0, separator);
  if (isCode(head)) return fail(head, text.slice(separator + 2));
  return fail("BLASPHEM_PACK_INVALID", text);
}
