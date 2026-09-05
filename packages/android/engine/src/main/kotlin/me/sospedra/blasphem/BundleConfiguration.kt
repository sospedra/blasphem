package me.sospedra.blasphem

import android.content.Context
import org.json.JSONObject

internal data class BundleConfiguration(
    val locales: List<String>, val detectLanguage: Boolean, val remote: Boolean, val files: List<String>,
) {
    fun sources(context: Context): ReadArtifact {
        if (!remote) return assetReader(context.assets)
        val base = "https://cdn.jsdelivr.net/npm/@blasphem/packs@$RELEASE_VERSION/dist"
        val manifestBytes = BlasphemRemoteStore.read(context, "$base/manifest.json", StoreIntegrity(MANIFEST_BYTES, MANIFEST_SHA256))
        val manifest = JSONObject(String(manifestBytes, Charsets.UTF_8))
        require(manifest.getInt("formatVersion") == 1) { "Unsupported data format" }
        val entries = manifest.getJSONObject("files")
        // Validate every selected entry before reading any data or constructing a native builder.
        val integrity = files.associateWith { file ->
            val entry = entries.getJSONObject(file)
            require(entry.get("bytes") is Number && entry.getDouble("bytes") == entry.getLong("bytes").toDouble())
            StoreIntegrity(entry.getLong("bytes"), entry.getString("sha256"))
        }
        val data = integrity.mapValues { (file, expected) -> BlasphemRemoteStore.read(context, "$base/$file", expected) }
        return { code, kind -> data.getValue(kind.file(code)) }
    }

    companion object {
        fun read(context: Context): BundleConfiguration {
            val config = JSONObject(context.assets.open("blasphem/bundle.json").bufferedReader().use { it.readText() })
            require(config.getInt("formatVersion") == 1) { "Unsupported bundle format" }
            require(config.getString("engineVersion") == RELEASE_VERSION && config.getString("dataVersion") == RELEASE_VERSION) {
                "Bundle and engine releases must match"
            }
            val manifest = config.getJSONObject("manifest")
            require(manifest.getLong("bytes") == MANIFEST_BYTES && manifest.getString("sha256") == MANIFEST_SHA256) {
                "Bundle manifest integrity differs from the trusted release"
            }
            val input = config.getJSONArray("locales")
            val codes = normalizeLocales(List(input.length()) { input.getString(it) })
            val detection = config.get("detectLanguage") as? Boolean ?: error("detectLanguage must be boolean")
            val files = codes.flatMap { if (detection) listOf("$it.pack", "$it.detect") else listOf("$it.pack") }.distinct()
            val declared = config.getJSONArray("files")
            require(files == List(declared.length()) { declared.getString(it) }) { "Bundle file selection differs from locales" }
            val remote = when (config.getString("assets")) {
                "bundled" -> false
                "remote", "jsdelivr" -> true
                else -> error("Unsupported asset delivery")
            }
            return BundleConfiguration(codes, detection, remote, files)
        }
    }
}
