import Foundation

/// The two files a locale ships: its pack and its language-detection slice.
enum ArtifactKind {
    case pack
    case detect

    var fileExtension: String {
        switch self {
        case .pack: return "pack"
        case .detect: return "detect"
        }
    }

    /// The SwiftPM product that carries `<code>.<extension>`, such as `BlasphemPackES`.
    func product(_ code: String) -> String {
        switch self {
        case .pack: return "BlasphemPack\(code.uppercased())"
        case .detect: return "BlasphemDetect\(code.uppercased())"
        }
    }
}

/// The bytes of one artifact, from `packsDirectory` when set, otherwise from the
/// SwiftPM resource bundle `Blasphem_<product>.bundle` the app carries.
func readArtifact(code: String, kind: ArtifactKind, packsDirectory: URL?) throws -> Data {
    let file = "\(code).\(kind.fileExtension)"
    if let packsDirectory {
        let url = packsDirectory.appendingPathComponent(file)
        return try readFile(url, missing: "\(file) is not in \(packsDirectory.path)")
    }
    let product = kind.product(code)
    guard let url = resourceURL(product: product, code: code, fileExtension: kind.fileExtension) else {
        throw BlasphemError(code: .localeMissing, message: "add the product \(product) to the target")
    }
    return try readFile(url, missing: "add the product \(product) to the target")
}

/// The search the generated `Bundle.module` accessor performs, so it holds for
/// apps, app extensions, tests, and executables.
private func resourceURL(product: String, code: String, fileExtension: String) -> URL? {
    let bundleName = "Blasphem_\(product)"
    let candidates = [
        Bundle.main.url(forResource: bundleName, withExtension: "bundle"),
        Bundle(for: Judge.self).resourceURL?.appendingPathComponent("\(bundleName).bundle"),
        Bundle.main.bundleURL.appendingPathComponent("\(bundleName).bundle"),
    ]
    let bundle = candidates.lazy.compactMap { $0 }.compactMap(Bundle.init(url:)).first
    return bundle?.url(forResource: code, withExtension: fileExtension)
}

private func readFile(_ url: URL, missing: String) throws -> Data {
    guard FileManager.default.fileExists(atPath: url.path) else {
        throw BlasphemError(code: .localeMissing, message: missing)
    }
    do {
        return try Data(contentsOf: url)
    } catch {
        throw BlasphemError(code: .fetchFailed, message: "\(url.lastPathComponent): \(error.localizedDescription)")
    }
}
