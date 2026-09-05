import CryptoKit
import Foundation

struct BlasphemIntegrity: Sendable, Codable, Equatable {
  let bytes: Double
  let sha256: String
}

private struct BlasphemManifestEnvelope: Codable {
  let sha256: String
  let data: Data
}

private final class BlasphemHTTPSDelegate: NSObject, URLSessionTaskDelegate {
  func urlSession(_ session: URLSession, task: URLSessionTask,
                  willPerformHTTPRedirection response: HTTPURLResponse,
                  newRequest request: URLRequest,
                  completionHandler: @escaping (URLRequest?) -> Void) {
    completionHandler(request.url?.scheme == "https" ? request : nil)
  }
}

enum BlasphemFileIO {
  private static let session: URLSession = {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.timeoutIntervalForResource = 30
    return URLSession(configuration: configuration, delegate: BlasphemHTTPSDelegate(), delegateQueue: nil)
  }()

  static func failure(_ message: String) -> NSError {
    NSError(domain: "Blasphem", code: 1, userInfo: [NSLocalizedDescriptionKey: "BLASPHEM_FETCH_FAILED: \(message)"])
  }

  private static func digest(_ data: Data) -> String {
    SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
  }

  private static func path(_ url: String) throws -> URL {
    let root = try FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
      .appendingPathComponent("blasphem", isDirectory: true)
      .appendingPathComponent("v1", isDirectory: true)
    try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    return root.appendingPathComponent(digest(Data(url.utf8)))
  }

  static func validate(_ expected: BlasphemIntegrity) throws {
    guard expected.bytes.isFinite, expected.bytes > 0, expected.bytes.rounded() == expected.bytes,
          expected.bytes <= 9_007_199_254_740_991,
          expected.sha256.range(of: "^[0-9a-f]{64}$", options: .regularExpression) != nil else {
      throw failure("Invalid download integrity metadata")
    }
  }

  static func valid(_ data: Data, expected: BlasphemIntegrity) -> Bool {
    Double(data.count) == expected.bytes && digest(data) == expected.sha256
  }

  static func cachedManifest(_ url: String) throws -> Data? {
    guard let data = try? Data(contentsOf: path(url)),
          let envelope = try? JSONDecoder().decode(BlasphemManifestEnvelope.self, from: data),
          digest(envelope.data) == envelope.sha256 else { return nil }
    return envelope.data
  }

  static func commitManifest(_ url: String, data: Data) throws {
    if try cachedManifest(url) == data { return }
    let envelope = BlasphemManifestEnvelope(sha256: digest(data), data: data)
    try JSONEncoder().encode(envelope).write(to: path(url), options: .atomic)
  }

  static func downloaded(_ url: String, expected: BlasphemIntegrity) async throws -> Data {
    let target = try path(url)
    if let cached = try? Data(contentsOf: target), valid(cached, expected: expected) {
      return cached
    }
    let data = try await verifiedDownload(url, expected: expected)
    try Task.checkCancellation()
    // Foundation writes a sibling temporary file and atomically renames it.
    try data.write(to: target, options: .atomic)
    return data
  }

  private static func verifiedDownload(_ url: String, expected: BlasphemIntegrity) async throws -> Data {
    var lastError: Error = failure("Download failed")
    let deadline = Date().addingTimeInterval(30)
    for _ in 0..<2 {
      try Task.checkCancellation()
      do {
        let data = try await fetchOnce(url, timeout: deadline.timeIntervalSinceNow)
        guard valid(data, expected: expected) else { throw failure("Integrity mismatch for \(url)") }
        return data
      } catch { lastError = error }
    }
    throw lastError
  }

  static func fetch(_ url: String) async throws -> Data {
    let deadline = Date().addingTimeInterval(30)
    do { return try await fetchOnce(url) }
    catch {
      try Task.checkCancellation()
      return try await fetchOnce(url, timeout: deadline.timeIntervalSinceNow)
    }
  }

  private static func fetchOnce(_ address: String, timeout: TimeInterval = 30) async throws -> Data {
    guard timeout > 0 else { throw failure("Download deadline exceeded") }
    guard let url = URL(string: address), url.scheme == "https", url.host != nil else {
      throw failure("Expected an HTTPS URL")
    }
    let request = URLRequest(url: url, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: timeout)
    let (data, response) = try await withThrowingTaskGroup(of: (Data, URLResponse).self) { group in
      group.addTask { try await session.data(for: request) }
      group.addTask {
        try await Task.sleep(nanoseconds: UInt64(timeout * 1_000_000_000))
        throw failure("Download deadline exceeded")
      }
      defer { group.cancelAll() }
      return try await group.next()!
    }
    guard let http = response as? HTTPURLResponse, http.statusCode == 200,
          http.url?.scheme == "https" else {
      throw failure("Download failed for \(address)")
    }
    return data
  }
}
