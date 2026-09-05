import Foundation

private final class BlasphemBundleToken: NSObject {}

enum BlasphemBundle {
  static func read(_ name: String) throws -> Data {
    guard name == (name as NSString).lastPathComponent else {
      throw BlasphemFileIO.failure("Invalid bundled filename")
    }
    for bundle in [Bundle.main, Bundle(for: BlasphemBundleToken.self)] {
      if let resource = resourceURL(bundle, name: name) {
        return try Data(contentsOf: resource)
      }
    }
    throw BlasphemFileIO.failure("\(name) is not in the app bundle")
  }

  private static func resourceURL(_ bundle: Bundle, name: String) -> URL? {
    if let location = bundle.url(forResource: "BlasphemLocales", withExtension: "bundle"),
       let locales = Bundle(url: location),
       let resource = locales.url(forResource: name, withExtension: nil) {
      return resource
    }
    return bundle.url(forResource: name, withExtension: nil, subdirectory: "blasphem")
      ?? bundle.url(forResource: name, withExtension: nil)
  }
}
