package me.sospedra.blasphem

import android.content.Context
import java.util.concurrent.locks.ReentrantReadWriteLock
import kotlin.concurrent.read
import kotlin.concurrent.write
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.suspendCancellableCoroutine

/**
 * A judge built once and called on every keystroke.
 *
 * [create] reads the packs from the app assets, so call it off the main
 * thread. [judge] is synchronous and safe from several threads. [close]
 * releases the engine.
 */
class Judge private constructor(handle: Long) : AutoCloseable {
    /** The loaded locales, in registry order. */
    val locales: List<String> = Native.engineLocales(handle).toList()

    private var engine: Long = handle
    private val lock = ReentrantReadWriteLock()

    /** Scores one message. Never throws while the judge is open; throws [BlasphemException.Code.CLOSED] after [close]. */
    fun judge(text: String): Judgement = lock.read {
        if (engine == 0L) throw BlasphemException(BlasphemException.Code.CLOSED, "the judge was closed")
        Native.engineJudge(engine, text)
    }

    /** Releases the engine. Later [judge] calls throw. Calling it twice is harmless. */
    override fun close() = lock.write {
        if (engine == 0L) return@write
        Native.engineFree(engine)
        engine = 0L
    }

    companion object {
        /** Reads the Gradle configuration and obtains the complete selected release. */
        @JvmStatic
        suspend fun create(context: Context): Judge {
            val (config, sources) = withContext(Dispatchers.IO) {
                val config = try { BundleConfiguration.read(context) }
                catch (failure: Exception) {
                    throw BlasphemException(BlasphemException.Code.FETCH_FAILED, "bundle.json: ${failure.message}")
                }
                val read = try { config.sources(context) }
                catch (failure: Exception) {
                    throw BlasphemException(BlasphemException.Code.FETCH_FAILED, "language data: ${failure.message}")
                }
                val sources = config.locales.map { Source(it, read, config.detectLanguage) }
                config to sources
            }
            return suspendCancellableCoroutine { continuation ->
                if (continuation.isActive) {
                    val judge = Judge(buildEngine(sources, JudgeOptions(config.locales, config.detectLanguage)))
                    continuation.resume(judge) { _, value, _ -> value.close() }
                }
            }
        }

        /**
         * Loads `options.locales` from `context.assets`, or from `options.packsDirectory`
         * when set, and builds the engine. Blocks on file reads.
         */
        @JvmStatic
        fun create(context: Context, options: JudgeOptions): Judge {
            val codes = normalizeLocales(options.locales)
            val read = options.packsDirectory?.let(::directoryReader) ?: assetReader(context.assets)
            val sources = codes.map { code -> Source(code, read, options.detectLanguage) }
            return Judge(buildEngine(sources, options))
        }
    }
}

private class Source(val code: String, read: ReadArtifact, detectLanguage: Boolean) {
    val pack: ByteArray = read(code, ArtifactKind.PACK)
    val detect: ByteArray? = if (detectLanguage) read(code, ArtifactKind.DETECT) else null
}

private fun buildEngine(sources: List<Source>, options: JudgeOptions): Long {
    val builder = Native.builderNew(options.detectLanguage, options.grawlix)
    try {
        for (source in sources) Native.builderAdd(builder, source.code, source.pack, source.detect)
        return Native.builderBuild(builder)
    } catch (failure: RuntimeException) {
        Native.builderFree(builder)
        throw BlasphemException.fromEngine(failure.message ?: "the native builder failed without a message")
    }
}
