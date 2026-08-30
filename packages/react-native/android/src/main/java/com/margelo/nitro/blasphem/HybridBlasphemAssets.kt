package com.margelo.nitro.blasphem

import com.facebook.proguard.annotations.DoNotStrip
import com.margelo.nitro.NitroModules
import com.margelo.nitro.core.ArrayBuffer
import com.margelo.nitro.core.Promise

/**
 * Reads `manifest.json`, `<code>.pack`, and `<code>.detect` from the app's
 * assets under `assets/blasphem/`.
 */
@DoNotStrip
class HybridBlasphemAssets : HybridBlasphemAssetsSpec() {
  override fun readBundled(name: String): Promise<ArrayBuffer> {
    return Promise.async {
      val context = NitroModules.applicationContext
        ?: throw Error("BLASPHEM_FETCH_FAILED: $name cannot load without an application context")
      val bytes = try {
        context.assets.open("blasphem/$name").use { it.readBytes() }
      } catch (error: Exception) {
        throw Error("BLASPHEM_FETCH_FAILED: assets/blasphem/$name: ${error.message}")
      }
      val buffer = ArrayBuffer.allocate(bytes.size)
      buffer.getBuffer(false).put(bytes)
      buffer
    }
  }
}
