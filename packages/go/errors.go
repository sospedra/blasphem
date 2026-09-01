package blasphem

import "strings"

// Error codes, shared with the JavaScript contract.
const (
	CodeLocalesEmpty      = "BLASPHEM_LOCALES_EMPTY"
	CodeLocaleUnsupported = "BLASPHEM_LOCALE_UNSUPPORTED"
	CodeLocaleMissing     = "BLASPHEM_LOCALE_MISSING"
	CodeAssetsRequired    = "BLASPHEM_ASSETS_REQUIRED"
	CodeFetchFailed       = "BLASPHEM_FETCH_FAILED"
	CodeDigestMismatch    = "BLASPHEM_DIGEST_MISMATCH"
	CodeFormatVersion     = "BLASPHEM_FORMAT_VERSION"
	CodePackInvalid       = "BLASPHEM_PACK_INVALID"
)

var knownCodes = map[string]bool{
	CodeLocalesEmpty: true, CodeLocaleUnsupported: true, CodeLocaleMissing: true, CodeAssetsRequired: true,
	CodeFetchFailed: true, CodeDigestMismatch: true, CodeFormatVersion: true, CodePackInvalid: true,
}

// Error is every failure New and Init return. Code is one of the Code constants.
type Error struct {
	Code    string
	Message string
}

func (e *Error) Error() string {
	return e.Code + ": " + e.Message
}

// parseError splits the "CODE: detail" text the native side reports.
func parseError(text string) *Error {
	if code, detail, found := strings.Cut(text, ": "); found && knownCodes[code] {
		return &Error{Code: code, Message: detail}
	}
	return &Error{Code: CodePackInvalid, Message: text}
}
