package me.sospedra.blasphem

/** One verdict for one message. */
data class Judgement(
    /** True when no nudge is due. Unroutable text is safe; the nudge fails open. */
    val safe: Boolean,
    /** Ordinal risk from 0 through 1. Not a probability. */
    val score: Double,
    /** The locale that produced the score, or null. */
    val locale: String?,
    /** Masked text for unsafe verdicts when requested, otherwise null. */
    val grawlix: String?,
)
