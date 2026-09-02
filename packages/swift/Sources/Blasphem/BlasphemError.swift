import Foundation

/// Every failure `Judge` raises. `code` is one of the nine contract codes and
/// `message` is the detail after `CODE: `.
public struct BlasphemError: Error, Equatable, Sendable {
    public enum Code: String, Sendable, CaseIterable {
        case localesEmpty = "BLASPHEM_LOCALES_EMPTY"
        case localeUnsupported = "BLASPHEM_LOCALE_UNSUPPORTED"
        case localeMissing = "BLASPHEM_LOCALE_MISSING"
        case assetsRequired = "BLASPHEM_ASSETS_REQUIRED"
        case fetchFailed = "BLASPHEM_FETCH_FAILED"
        case digestMismatch = "BLASPHEM_DIGEST_MISMATCH"
        case formatVersion = "BLASPHEM_FORMAT_VERSION"
        case packInvalid = "BLASPHEM_PACK_INVALID"
        case closed = "BLASPHEM_CLOSED"
    }

    public let code: Code
    public let message: String

    public init(code: Code, message: String) {
        self.code = code
        self.message = message
    }

    /// Parses the `CODE: detail` text the engine reports. Anything else is a malformed pack.
    static func fromEngine(_ text: String) -> BlasphemError {
        guard let separator = text.range(of: ": "),
              let code = Code(rawValue: String(text[..<separator.lowerBound]))
        else {
            return BlasphemError(code: .packInvalid, message: text)
        }
        return BlasphemError(code: code, message: String(text[separator.upperBound...]))
    }
}

extension BlasphemError: CustomStringConvertible {
    public var description: String { "\(code.rawValue): \(message)" }
}

extension BlasphemError: LocalizedError {
    public var errorDescription: String? { description }
}
