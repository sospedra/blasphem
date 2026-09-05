import Foundation
import CryptoKit
import CoreFoundation

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("BlasphemAssets: \(message)\n".utf8))
    exit(1)
}

do {
let arguments = CommandLine.arguments
guard arguments.count == 4 else { fail("Expected configuration, output, and build system") }
let fm = FileManager.default
let input = URL(fileURLWithPath: arguments[1])
let output = URL(fileURLWithPath: arguments[2])
let resourceRoot = Bundle.module.resourceURL!
let resources = fm.fileExists(atPath: resourceRoot.appendingPathComponent("version.txt").path)
    ? resourceRoot : resourceRoot.appendingPathComponent("Resources")
let config = try JSONSerialization.jsonObject(with: Data(contentsOf: input)) as? [String: Any] ?? [:]
let registry = locales.map(\.code)
let aliases = Dictionary(uniqueKeysWithValues: locales.flatMap { entry in
    ([entry.code] + entry.aliases).map { ($0, entry.code) }
})
let requested: [String]
if config["locales"] as? String == "all" { requested = registry }
else if let values = config["locales"] as? [String], !values.isEmpty {
    requested = values.map {
        let code = $0.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard let canonical = aliases[code] else { fail("Unsupported locale: \(code)") }
        return canonical
    }
} else { fail("locales must be a nonempty array or all") }
guard requested.allSatisfy(registry.contains) else { fail("Unsupported locale") }
let selectedLocales = registry.filter(Set(requested).contains)
let requestedAssets = config["assets"] as? String ?? "bundled"
let assets = requestedAssets == "jsdelivr" ? "remote" : requestedAssets
if let value = config["assets"], !(value is String) { fail("assets must be a string") }
guard ["bundled", "remote"].contains(assets) else { fail("assets must be bundled or remote") }
if let value = config["detectLanguage"] {
    guard let number = value as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else { fail("detectLanguage must be boolean") }
}
let detection = config["detectLanguage"] as? Bool ?? true
let files = selectedLocales.flatMap { detection ? ["\($0).pack", "\($0).detect"] : ["\($0).pack"] }
let version = try String(contentsOf: resources.appendingPathComponent("version.txt"), encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines)
let manifest = try Data(contentsOf: resources.appendingPathComponent("manifest.json"))
let records = try JSONSerialization.jsonObject(with: manifest) as? [String: Any]
guard records?["formatVersion"] as? Int == 1,
      let entries = records?["files"] as? [String: [String: Any]] else { fail("Invalid release manifest") }
for file in files {
    let data = try Data(contentsOf: resources.appendingPathComponent(file))
    guard entries[file]?["bytes"] as? Int == data.count,
          entries[file]?["sha256"] as? String == SHA256.hash(data: data).map({ String(format: "%02x", $0) }).joined() else {
        fail("Invalid release asset: \(file)")
    }
}
let bundle: [String: Any] = ["formatVersion": 1, "engineVersion": version, "dataVersion": version,
    "locales": selectedLocales, "assets": assets, "detectLanguage": detection, "files": files,
    "manifest": ["bytes": manifest.count, "sha256": SHA256.hash(data: manifest).map { String(format: "%02x", $0) }.joined()]]
try fm.createDirectory(at: output, withIntermediateDirectories: true)
// This directory belongs exclusively to this plugin invocation.
for previous in try fm.contentsOfDirectory(at: output, includingPropertiesForKeys: nil) { try fm.removeItem(at: previous) }
let destination = output.appendingPathComponent("BlasphemAssets.bundle")
try fm.createDirectory(at: destination, withIntermediateDirectories: true)
try JSONSerialization.data(withJSONObject: bundle, options: [.sortedKeys]).write(to: destination.appendingPathComponent("bundle.json"), options: .atomic)
if assets == "bundled" {
    for file in files + ["NOTICE"] { try fm.copyItem(at: resources.appendingPathComponent(file), to: destination.appendingPathComponent(file)) }
}
let accessor = arguments[3] == "xcode" ? "Bundle(for: BlasphemResourceAnchor.self)" : "Bundle.module"
let source = """
import Foundation
import Blasphem
private final class BlasphemResourceAnchor {}
extension Judge {
    public static func create(grawlix: Bool = false) async throws -> Judge {
        guard let directory = \(accessor).url(forResource: "BlasphemAssets", withExtension: "bundle") else {
            throw NSError(domain: "Blasphem", code: 1)
        }
        return try await Judge.create(configurationDirectory: directory, grawlix: grawlix)
    }
}
"""
try source.write(to: output.appendingPathComponent("BlasphemConfiguration.generated.swift"), atomically: true, encoding: .utf8)

} catch { fail(error.localizedDescription) }
