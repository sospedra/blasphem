import BlasphemFFI
import Foundation

/// A judge built once and called on every keystroke.
///
/// Construction reads the packs from disk, so call it off the main thread.
/// `judge(_:)` is synchronous and safe from several threads. `close()`
/// releases the engine; `deinit` closes too.
public final class Judge: @unchecked Sendable {
    /// The loaded locales, in registry order.
    public let locales: [String]

    private var engine: OpaquePointer?
    private let lock = ReadWriteLock()

    /// Loads `locales` and builds the engine.
    ///
    /// - Parameters:
    ///   - locales: lowercase codes such as `["en", "es"]`, `id` (Indonesian), or `ms` (Malay). Empty throws.
    ///   - detectLanguage: route by detected language. Every locale then needs its detection slice.
    ///   - grawlix: populate `Judgement.grawlix` for unsafe verdicts.
    ///   - packsDirectory: read `<code>.pack` and `<code>.detect` from this folder instead of the app bundle.
    public init(
        locales requested: [String],
        detectLanguage: Bool = true,
        grawlix: Bool = false,
        packsDirectory: URL? = nil
    ) throws {
        let codes = try normalizeLocales(requested)
        let sources = try codes.map { code in
            try Source(code: code, detectLanguage: detectLanguage, packsDirectory: packsDirectory)
        }
        let engine = try buildEngine(sources, detectLanguage: detectLanguage, grawlix: grawlix)
        self.engine = engine
        self.locales = loadedLocales(engine)
    }

    deinit {
        close()
    }

    /// Scores one message. Never fails while the judge is open; throws `.closed` after `close()`.
    public func judge(_ text: String) throws -> Judgement {
        try lock.read {
            guard let engine else {
                throw BlasphemError(code: .closed, message: "the judge was closed")
            }
            let verdict = blasphem_engine_judge(engine, text)
            defer { blasphem_judgement_free(verdict) }
            return Judgement(
                safe: verdict.safe,
                score: verdict.score,
                locale: verdict.locale.map { String(cString: $0) },
                grawlix: verdict.grawlix.map { String(cString: $0) }
            )
        }
    }

    /// Releases the engine. Later `judge(_:)` calls throw `.closed`. Calling it twice is harmless.
    public func close() {
        lock.write {
            guard let engine else { return }
            blasphem_engine_free(engine)
            self.engine = nil
        }
    }
}

private struct Source {
    let code: String
    let pack: Data
    let detect: Data?

    init(code: String, detectLanguage: Bool, packsDirectory: URL?) throws {
        self.code = code
        pack = try readArtifact(code: code, kind: .pack, packsDirectory: packsDirectory)
        detect = try detectLanguage ? readArtifact(code: code, kind: .detect, packsDirectory: packsDirectory) : nil
    }
}

private func buildEngine(_ sources: [Source], detectLanguage: Bool, grawlix: Bool) throws -> OpaquePointer {
    guard let builder = blasphem_builder_new(detectLanguage, grawlix) else {
        throw BlasphemError(code: .packInvalid, message: "the native builder could not be created")
    }
    do {
        for source in sources {
            try add(source, to: builder)
        }
        guard let engine = blasphem_builder_build(builder) else {
            throw builderError(builder)
        }
        return engine
    } catch {
        blasphem_builder_free(builder)
        throw error
    }
}

private func add(_ source: Source, to builder: OpaquePointer) throws {
    let status = withBytes(source.pack) { pack, packCount in
        withBytes(source.detect) { detect, detectCount in
            blasphem_builder_add(builder, source.code, pack, packCount, nil, detect, detectCount, nil)
        }
    }
    guard status == 0 else {
        throw builderError(builder)
    }
}

private func withBytes<Result>(_ data: Data?, _ body: (UnsafePointer<UInt8>?, Int) -> Result) -> Result {
    guard let data else { return body(nil, 0) }
    return data.withUnsafeBytes { raw in
        body(raw.bindMemory(to: UInt8.self).baseAddress, raw.count)
    }
}

private func builderError(_ builder: OpaquePointer) -> BlasphemError {
    guard let text = blasphem_builder_error(builder) else {
        return BlasphemError(code: .packInvalid, message: "the native builder failed without a message")
    }
    return BlasphemError.fromEngine(String(cString: text))
}

private func loadedLocales(_ engine: OpaquePointer) -> [String] {
    (0..<blasphem_engine_locale_count(engine)).compactMap { index in
        guard let text = blasphem_engine_locale(engine, index) else { return nil }
        defer { blasphem_text_free(text) }
        return String(cString: text)
    }
}
