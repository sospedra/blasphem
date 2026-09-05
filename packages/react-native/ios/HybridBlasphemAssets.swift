import Foundation
import NitroModules

/// Reads `manifest.json`, `<code>.pack`, and `<code>.detect` from the app
/// bundle. Add the files to the target as a folder reference named `blasphem`,
/// or at the bundle root.
class HybridBlasphemAssets: HybridBlasphemAssetsSpec {
  func readBundled(name: String) throws -> Promise<ArrayBuffer> {
    return Promise.async {
      let url = Bundle.main.url(forResource: name, withExtension: nil, subdirectory: "blasphem")
        ?? Bundle.main.url(forResource: name, withExtension: nil)
      guard let url else {
        throw RuntimeError.error(withMessage: "BLASPHEM_FETCH_FAILED: \(name) is not in the app bundle")
      }
      let data = try Data(contentsOf: url)
      return try ArrayBuffer.copy(data: data)
    }
  }
}
