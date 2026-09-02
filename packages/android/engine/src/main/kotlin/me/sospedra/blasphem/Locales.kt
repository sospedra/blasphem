package me.sospedra.blasphem

private val canonical: Map<String, String> =
    LOCALES.flatMap { (code, aliases) -> (aliases + code).map { it to code } }.toMap()

private val registryOrder: Map<String, Int> =
    LOCALES.withIndex().associate { (index, entry) -> entry.first to index }

/** Lowercases, resolves aliases, rejects unknown codes, and returns registry order without repeats. */
internal fun normalizeLocales(requested: List<String>): List<String> {
    if (requested.isEmpty()) {
        throw BlasphemException(BlasphemException.Code.LOCALES_EMPTY, "pass at least one locale, such as listOf(\"en\")")
    }
    val codes = requested.map { raw ->
        canonical[raw.trim().lowercase()]
            ?: throw BlasphemException(BlasphemException.Code.LOCALE_UNSUPPORTED, "unsupported locale \"$raw\"")
    }
    return codes.distinct().sortedBy { registryOrder.getValue(it) }
}
