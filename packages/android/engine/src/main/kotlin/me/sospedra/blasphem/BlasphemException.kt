package me.sospedra.blasphem

/** Every failure [Judge] throws. [code] is one of the nine contract codes and [message] is the detail. */
class BlasphemException(val code: Code, message: String) : RuntimeException(message) {
    enum class Code(val wire: String) {
        LOCALES_EMPTY("BLASPHEM_LOCALES_EMPTY"),
        LOCALE_UNSUPPORTED("BLASPHEM_LOCALE_UNSUPPORTED"),
        LOCALE_MISSING("BLASPHEM_LOCALE_MISSING"),
        ASSETS_REQUIRED("BLASPHEM_ASSETS_REQUIRED"),
        FETCH_FAILED("BLASPHEM_FETCH_FAILED"),
        DIGEST_MISMATCH("BLASPHEM_DIGEST_MISMATCH"),
        FORMAT_VERSION("BLASPHEM_FORMAT_VERSION"),
        PACK_INVALID("BLASPHEM_PACK_INVALID"),
        CLOSED("BLASPHEM_CLOSED"),
    }

    override fun toString(): String = "${code.wire}: $message"

    internal companion object {
        private val byWire = Code.entries.associateBy { it.wire }

        /** Parses the `CODE: detail` text the engine reports. Anything else is a malformed pack. */
        fun fromEngine(text: String): BlasphemException {
            val separator = text.indexOf(": ")
            val code = if (separator == -1) null else byWire[text.substring(0, separator)]
            if (code == null) return BlasphemException(Code.PACK_INVALID, text)
            return BlasphemException(code, text.substring(separator + 2))
        }
    }
}
