/// One verdict for one message.
public struct Judgement: Equatable, Sendable {
    /// True when no nudge is due. Unroutable text is safe; the nudge fails open.
    public let safe: Bool
    /// Ordinal risk from 0 through 1. Not a probability.
    public let score: Double
    /// The locale that produced the score, or nil.
    public let locale: String?
    /// The masked text when `grawlix` was requested, otherwise nil.
    public let grawlix: String?

    public init(safe: Bool, score: Double, locale: String?, grawlix: String?) {
        self.safe = safe
        self.score = score
        self.locale = locale
        self.grawlix = grawlix
    }
}
