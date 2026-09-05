import Foundation

/// Process-wide flights share work across native callers and Nitro bridges.
actor BlasphemDownloadStore {
  static let shared = BlasphemDownloadStore()
  private var flights: [String: Task<Data, Error>] = [:]
  private var pendingManifests: [String: Data] = [:]

  func manifest(_ url: String, refresh: Bool) async throws -> Data {
    if !refresh, let cached = try BlasphemFileIO.cachedManifest(url) {
      return cached
    }
    if !refresh, let pending = pendingManifests[url] {
      return pending
    }
    let flightKey = refresh ? "manifest-refresh:\(url)" : "manifest:\(url)"
    return try await share(flightKey) {
      let data = try await BlasphemFileIO.fetch(url)
      await self.remember(url, data: data)
      return data
    }
  }

  private func remember(_ url: String, data: Data) {
    pendingManifests[url] = data
  }

  func commit(_ url: String, data: Data) throws {
    try BlasphemFileIO.commitManifest(url, data: data)
    if pendingManifests[url] == data { pendingManifests.removeValue(forKey: url) }
  }

  func downloaded(_ url: String, expected: BlasphemIntegrity) async throws -> Data {
    try BlasphemFileIO.validate(expected)
    let key = "\(url):\(expected.bytes):\(expected.sha256)"
    return try await share(key) {
      try await BlasphemFileIO.downloaded(url, expected: expected)
    }
  }

  private func share(_ key: String, operation: @escaping @Sendable () async throws -> Data) async throws -> Data {
    if let flight = flights[key] {
      let data = try await flight.value
      try Task.checkCancellation()
      return data
    }
    let flight = Task { try await operation() }
    flights[key] = flight
    defer { flights.removeValue(forKey: key) }
    let data = try await flight.value
    try Task.checkCancellation()
    return data
  }
}
