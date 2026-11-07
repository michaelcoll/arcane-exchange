import Foundation

/// A Card found, and which candidate it comes from.
struct LookupMatch: Equatable, Sendable {
    let card: ScryfallCard
    /// Found in the token set (`t` + set) rather than the Card set.
    let isToken: Bool
    /// The English printing is shown because the language read has none.
    let isEnglishPrinting: Bool
}

/// What the screen shows for a retained reading.
enum LookupState: Equatable, Sendable {
    case loading
    /// Every Card found: one or two (Card set and token set can both exist).
    case found([LookupMatch])
    case notFound
    case failed(String)
}

/// Network glue (no unit tests): runs the plan, one request at a time, at most 10 per second
/// (Scryfall's limit for these endpoints), caching the outcome per reading in memory.
actor ScryfallClient {
    private var cache: [CollectorReading: LookupState] = [:]
    private var lastRequest = ContinuousClock.now
    private let minimumGap = Duration.milliseconds(120)

    func lookup(_ reading: CollectorReading) async -> LookupState {
        if let cached = cache[reading] { return cached }
        var matches: [LookupMatch] = []
        var failure: String?
        for target in ScryfallLookup.plan(for: reading) {
            switch await run(target) {
            case let .found(found): matches += found
            case let .failed(message): failure = message
            default: break
            }
        }
        let outcome: LookupState = !matches.isEmpty ? .found(matches) : failure.map(LookupState.failed) ?? .notFound
        if case .failed = outcome {} else { cache[reading] = outcome }
        return outcome
    }

    private func run(_ target: LookupTarget) async -> LookupState {
        var outcome = LookupState.notFound
        for step in target.steps {
            outcome = await fetch(step, isToken: target.isToken)
            if outcome != .notFound { break }
        }
        return outcome
    }

    private func fetch(_ step: LookupStep, isToken: Bool) async -> LookupState {
        let wait = lastRequest + minimumGap
        if ContinuousClock.now < wait { try? await Task.sleep(until: wait) }
        lastRequest = .now

        var request = URLRequest(url: step.url)
        request.setValue("ArcaneExchangePrototype/0.1", forHTTPHeaderField: "User-Agent")
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            let status = (response as? HTTPURLResponse)?.statusCode ?? 0
            if status == 404 { return .notFound }
            if status == 429 { return .failed("limite Scryfall atteinte (429) : attendre 30 s") }
            guard let card = ScryfallLookup.decode(data) else { return .failed("réponse inattendue (HTTP \(status))") }
            return .found([LookupMatch(card: card, isToken: isToken, isEnglishPrinting: step.isEnglishPrinting)])
        } catch {
            return .failed(error.localizedDescription)
        }
    }
}
