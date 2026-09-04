import Foundation

private let canonical: [String: String] = Dictionary(
    uniqueKeysWithValues: locales.flatMap { entry in
        ([entry.code] + entry.aliases).map { ($0, entry.code) }
    }
)

private let registryOrder: [String: Int] = Dictionary(
    uniqueKeysWithValues: locales.enumerated().map { ($1.code, $0) }
)

/// Lowercases, resolves aliases, rejects unknown codes, and returns registry order without repeats.
func normalizeLocales(_ requested: [String]) throws -> [String] {
    guard !requested.isEmpty else {
        throw BlasphemError(code: .localesEmpty, message: "pass at least one locale, such as [\"en\"]")
    }
    let codes = try requested.map { raw -> String in
        guard let code = canonical[raw.trimmingCharacters(in: .whitespaces).lowercased()] else {
            throw BlasphemError(code: .localeUnsupported, message: "unsupported locale \"\(raw)\"")
        }
        return code
    }
    return Array(Set(codes)).sorted { (registryOrder[$0] ?? 0) < (registryOrder[$1] ?? 0) }
}
