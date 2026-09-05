package com.margelo.nitro.blasphem

import android.content.Context
import android.system.Os
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.CompletableFuture
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ExecutionException
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

internal data class StoreIntegrity(val bytes: Long, val sha256: String) {
    init {
        require(bytes in 1..Int.MAX_VALUE.toLong() && sha256.matches(Regex("[0-9a-f]{64}"))) {
            "BLASPHEM_FETCH_FAILED: Invalid integrity metadata"
        }
    }
    fun matches(data: ByteArray): Boolean = data.size.toLong() == bytes &&
        MessageDigest.getInstance("SHA-256").digest(data).joinToString("") { "%02x".format(it) } == sha256
}

/** Persistent verified storage, independent of JNI and Nitro. */
internal object BlasphemRemoteStore {
    private val flights = ConcurrentHashMap<String, CompletableFuture<ByteArray>>()
    private val deadlines = Executors.newScheduledThreadPool(1) { runnable ->
        Thread(runnable, "blasphem-download-deadline").apply { isDaemon = true }
    }
    private val address = Regex("https://cdn\\.jsdelivr\\.net/npm/@blasphem/packs@([0-9]+\\.[0-9]+\\.[0-9]+(?:-[A-Za-z0-9.-]+)?)/dist/(manifest\\.json|[a-z]+\\.(?:pack|detect))")

    fun read(context: Context, url: String, expected: StoreIntegrity): ByteArray {
        val match = address.matchEntire(url) ?: throw IOException("BLASPHEM_FETCH_FAILED: Invalid exact release URL")
        val directory = File(context.filesDir, "blasphem/${match.groupValues[1]}/1")
        if (!directory.isDirectory && !directory.mkdirs() && !directory.isDirectory) {
            throw IOException("BLASPHEM_FETCH_FAILED: Cannot create persistent storage")
        }
        val target = File(directory, "${match.groupValues[2]}.${expected.bytes}.${expected.sha256}")
        val key = "${target.absolutePath}:$url:${expected.bytes}:${expected.sha256}"
        val flight = CompletableFuture<ByteArray>()
        val existing = flights.putIfAbsent(key, flight)
        if (existing != null) {
            try { return existing.get() }
            catch (error: ExecutionException) { throw error.cause ?: error }
        }
        try {
            val cached = runCatching { target.readBytes() }.getOrNull()
            val bytes = cached?.takeIf(expected::matches) ?: download(url, expected).also { atomicWrite(target, it) }
            flight.complete(bytes)
            return bytes
        } catch (error: Throwable) {
            flight.completeExceptionally(error)
            throw error
        } finally { flights.remove(key, flight) }
    }

    private fun download(url: String, expected: StoreIntegrity): ByteArray {
        var failure: IOException? = null
        repeat(2) {
            try { return fetch(url, expected) }
            catch (error: IOException) { failure = error }
        }
        throw failure!!
    }

    private fun fetch(url: String, expected: StoreIntegrity): ByteArray {
        val connection = URL(url).openConnection() as HttpURLConnection
        connection.connectTimeout = 30_000
        connection.readTimeout = 30_000
        connection.instanceFollowRedirects = false
        connection.useCaches = false
        val expires = System.nanoTime() + TimeUnit.SECONDS.toNanos(30)
        val deadline = deadlines.schedule({ connection.disconnect() }, 30, TimeUnit.SECONDS)
        try {
            if (connection.responseCode != 200) throw IOException("BLASPHEM_FETCH_FAILED: HTTP ${connection.responseCode}")
            val bytes = connection.inputStream.use { input ->
                val output = java.io.ByteArrayOutputStream()
                val buffer = ByteArray(8192)
                while (true) {
                    val size = input.read(buffer)
                    if (System.nanoTime() >= expires) throw IOException("BLASPHEM_FETCH_FAILED: Request deadline")
                    if (size < 0) break
                    if (output.size().toLong() + size > expected.bytes) throw IOException("BLASPHEM_FETCH_FAILED: Invalid byte length")
                    output.write(buffer, 0, size)
                }
                output.toByteArray()
            }
            if (!expected.matches(bytes)) throw IOException("BLASPHEM_FETCH_FAILED: Integrity mismatch")
            return bytes
        } finally {
            deadline.cancel(false)
            connection.disconnect()
        }
    }

    private fun atomicWrite(target: File, bytes: ByteArray) {
        val temporary = File.createTempFile(".blasphem-", ".tmp", target.parentFile)
        try {
            FileOutputStream(temporary).use { stream -> stream.write(bytes); stream.fd.sync() }
            Os.rename(temporary.absolutePath, target.absolutePath)
        } finally { temporary.delete() }
    }
}
