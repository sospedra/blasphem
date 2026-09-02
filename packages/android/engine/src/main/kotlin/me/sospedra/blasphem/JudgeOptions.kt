package me.sospedra.blasphem

import java.io.File

/** Options for one judge. [locales] is required. */
data class JudgeOptions(
    /** Lowercase codes to load, such as `listOf("en", "es")`. `id` is an alias for `ms`. Empty throws. */
    val locales: List<String>,
    /** Route by detected language. Every locale then needs its `blasphem-detect-<code>` artifact. */
    val detectLanguage: Boolean = true,
    /** Populate [Judgement.grawlix]. */
    val grawlix: Boolean = false,
    /** Read `<code>.pack` and `<code>.detect` from this folder instead of the app assets. */
    val packsDirectory: File? = null,
)
