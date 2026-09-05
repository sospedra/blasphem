import Foundation
import NitroModules

class HybridBlasphemAssets: HybridBlasphemAssetsSpec {
  func readBundled(name: String) throws -> Promise<ArrayBuffer> {
    return Promise.async {
      let data = try BlasphemBundle.read(name)
      return try ArrayBuffer.copy(data: data)
    }
  }

  func readManifest(url: String, refresh: Bool) throws -> Promise<ArrayBuffer> {
    return Promise.async {
      let data = try await BlasphemDownloadStore.shared.manifest(url, refresh: refresh)
      return try ArrayBuffer.copy(data: data)
    }
  }

  func commitManifest(url: String, bytes: ArrayBuffer) throws -> Promise<Void> {
    // Copy before leaving the bridge invocation. JavaScript owns the buffer.
    let data = Data(bytes: bytes.data, count: bytes.size)
    return Promise.async {
      try await BlasphemDownloadStore.shared.commit(url, data: data)
    }
  }

  func readDownloaded(url: String, expected: DownloadIntegrity) throws -> Promise<ArrayBuffer> {
    let integrity = BlasphemIntegrity(bytes: expected.bytes, sha256: expected.sha256)
    return Promise.async {
      let data = try await BlasphemDownloadStore.shared.downloaded(url, expected: integrity)
      return try ArrayBuffer.copy(data: data)
    }
  }
}
