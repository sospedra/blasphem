package com.margelo.nitro.blasphem

/** Nitro adapter over the Android engine's verified persistent store. */
internal object BlasphemDownloadStore {
  private val manifestURL = "https://cdn.jsdelivr.net/npm/@blasphem/packs@$RELEASE_VERSION/dist/manifest.json"
  private val manifestIntegrity = StoreIntegrity(MANIFEST_BYTES, MANIFEST_SHA256)

  fun manifest(url: String, refresh: Boolean): ByteArray {
    require(url == manifestURL) { "BLASPHEM_FETCH_FAILED: Manifest release differs from the engine" }
    // Immutable releases need no refresh. Each read revalidates the cache.
    return BlasphemRemoteStore.read(BlasphemFileIO.context(), url, manifestIntegrity)
  }

  fun commit(url: String, bytes: ByteArray) {
    require(url == manifestURL && manifestIntegrity.matches(bytes)) {
      "BLASPHEM_FETCH_FAILED: Manifest differs from the trusted release"
    }
    // read() already committed verified bytes atomically.
  }

  fun downloaded(url: String, expected: DownloadIntegrity): ByteArray {
    require(expected.bytes.isFinite() && expected.bytes % 1.0 == 0.0 && expected.bytes <= Int.MAX_VALUE) {
      "BLASPHEM_FETCH_FAILED: Invalid byte length"
    }
    require(url.startsWith("https://cdn.jsdelivr.net/npm/@blasphem/packs@$RELEASE_VERSION/dist/")) {
      "BLASPHEM_FETCH_FAILED: Data release differs from the engine"
    }
    return BlasphemRemoteStore.read(BlasphemFileIO.context(), url, StoreIntegrity(expected.bytes.toLong(), expected.sha256))
  }
}
