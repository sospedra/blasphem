package com.margelo.nitro.blasphem

import com.facebook.proguard.annotations.DoNotStrip
import com.margelo.nitro.core.ArrayBuffer
import com.margelo.nitro.core.Promise

/**
 * Reads `manifest.json`, `<code>.pack`, and `<code>.detect` from the app's
 * assets under `assets/blasphem/`.
 */
@DoNotStrip
class HybridBlasphemAssets : HybridBlasphemAssetsSpec() {
  override fun readBundled(name: String): Promise<ArrayBuffer> {
    return Promise.parallel {
      require(name == java.io.File(name).name) { "BLASPHEM_FETCH_FAILED: Invalid bundled filename" }
      val bytes = BlasphemFileIO.context().assets.open("blasphem/$name").use { it.readBytes() }
      buffer(bytes)
    }
  }

  override fun readManifest(url: String, refresh: Boolean): Promise<ArrayBuffer> =
    Promise.parallel { buffer(BlasphemDownloadStore.manifest(url, refresh)) }

  override fun commitManifest(url: String, bytes: ArrayBuffer): Promise<Unit> {
    val source = bytes.getBuffer(true).duplicate()
    val copy = ByteArray(source.remaining())
    source.get(copy)
    return Promise.parallel { BlasphemDownloadStore.commit(url, copy) }
  }

  override fun readDownloaded(url: String, expected: DownloadIntegrity): Promise<ArrayBuffer> =
    Promise.parallel { buffer(BlasphemDownloadStore.downloaded(url, expected)) }

  private fun buffer(bytes: ByteArray): ArrayBuffer =
    ArrayBuffer.allocate(bytes.size).also { it.getBuffer(false).put(bytes) }
}
