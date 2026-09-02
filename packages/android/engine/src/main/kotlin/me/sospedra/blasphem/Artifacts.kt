package me.sospedra.blasphem

import android.content.res.AssetManager
import java.io.File
import java.io.FileNotFoundException
import java.io.IOException

/** The two files a locale ships: its pack and its language-detection slice. */
internal enum class ArtifactKind(private val extension: String, private val artifactPrefix: String) {
    PACK("pack", "blasphem-pack-"),
    DETECT("detect", "blasphem-detect-"),
    ;

    fun file(code: String): String = "$code.$extension"

    /** The Maven artifact that carries `<code>.<extension>`, such as `me.sospedra.blasphem:blasphem-pack-es`. */
    fun artifact(code: String): String = "me.sospedra.blasphem:$artifactPrefix$code"
}

internal typealias ReadArtifact = (code: String, kind: ArtifactKind) -> ByteArray

/** Reads `assets/blasphem/<code>.<kind>`, the path every `blasphem-pack-*` and `blasphem-detect-*` AAR merges into the app. */
internal fun assetReader(assets: AssetManager): ReadArtifact = { code, kind ->
    val name = "blasphem/${kind.file(code)}"
    try {
        assets.open(name).use { it.readBytes() }
    } catch (missing: FileNotFoundException) {
        throw BlasphemException(BlasphemException.Code.LOCALE_MISSING, "add ${kind.artifact(code)}")
    } catch (failure: IOException) {
        throw BlasphemException(BlasphemException.Code.FETCH_FAILED, "assets/$name: ${failure.message}")
    }
}

/** Reads `<code>.<kind>` from a folder. Tests and command-line hosts use it. */
internal fun directoryReader(directory: File): ReadArtifact = { code, kind ->
    val file = directory.resolve(kind.file(code))
    if (!file.isFile) {
        throw BlasphemException(BlasphemException.Code.LOCALE_MISSING, "${kind.file(code)} is not in ${directory.path}")
    }
    try {
        file.readBytes()
    } catch (failure: IOException) {
        throw BlasphemException(BlasphemException.Code.FETCH_FAILED, "${file.path}: ${failure.message}")
    }
}
