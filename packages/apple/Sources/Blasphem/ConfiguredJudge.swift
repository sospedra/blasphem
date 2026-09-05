import Foundation
import CryptoKit

private struct AssetConfiguration: Decodable {
    let formatVersion: Int
    let engineVersion: String
    let dataVersion: String
    let locales: [String]
    let assets: String
    let detectLanguage: Bool
    let files: [String]
    let manifest: BlasphemIntegrity
}

private struct AssetManifest: Decodable {
    let formatVersion: Int
    let files: [String: BlasphemIntegrity]
}

extension Judge {
    /// The build plugin supplies the consuming target's resource directory.
    public static func create(configurationDirectory: URL, grawlix: Bool = false) async throws -> Judge {
        do {
            return try await createConfigured(configurationDirectory: configurationDirectory, grawlix: grawlix)
        } catch is CancellationError {
            throw CancellationError()
        } catch let error as BlasphemError {
            throw error
        } catch {
            throw BlasphemError(code: .fetchFailed, message: error.localizedDescription)
        }
    }

    private static func createConfigured(configurationDirectory: URL, grawlix: Bool) async throws -> Judge {
        let bytes = try Data(contentsOf: configurationDirectory.appendingPathComponent("bundle.json"))
        let config = try JSONDecoder().decode(AssetConfiguration.self, from: bytes)
        let assets = config.assets == "jsdelivr" ? "remote" : config.assets
        let codes = try normalizeLocales(config.locales)
        let selected = codes.flatMap { config.detectLanguage ? ["\($0).pack", "\($0).detect"] : ["\($0).pack"] }
        guard config.formatVersion == 1, config.engineVersion == blasphemEngineVersion, config.engineVersion == config.dataVersion,
              config.dataVersion.range(of: "^[0-9]+\\.[0-9]+\\.[0-9]+([+-][A-Za-z0-9.-]+)?$", options: .regularExpression) != nil,
              codes == config.locales, selected == config.files,
              ["bundled", "remote"].contains(assets) else {
            throw BlasphemError(code: .formatVersion, message: "Invalid bundle configuration or engine release")
        }
        var directory = configurationDirectory
        if assets == "remote" {
            let base = "https://cdn.jsdelivr.net/npm/@blasphem/packs@\(config.dataVersion)/dist"
            let manifestBytes = try await BlasphemDownloadStore.shared.downloaded("\(base)/manifest.json", expected: config.manifest)
            let manifest = try JSONDecoder().decode(AssetManifest.self, from: manifestBytes)
            guard manifest.formatVersion == config.formatVersion else { throw BlasphemFileIO.failure("Invalid manifest format") }
            let root = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
            let key = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
            directory = root.appendingPathComponent("blasphem/v\(config.formatVersion)/\(config.dataVersion)/\(key)")
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            for name in selected {
                guard let expected = manifest.files[name] else { throw BlasphemFileIO.failure("Missing manifest entry: \(name)") }
                let data = try await BlasphemDownloadStore.shared.downloaded("\(base)/\(name)", expected: expected)
                try Task.checkCancellation()
                let target = directory.appendingPathComponent(name)
                if (try? Data(contentsOf: target)) != data { try data.write(to: target, options: .atomic) }
            }
        }
        try Task.checkCancellation()
        return try Judge(locales: codes, detectLanguage: config.detectLanguage, grawlix: grawlix, packsDirectory: directory)
    }
}
