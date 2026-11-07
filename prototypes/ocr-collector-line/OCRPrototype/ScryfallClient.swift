import Foundation

/// The Card found for a reading.
struct LookupMatch: Equatable, Sendable {
    let card: ScryfallCard
    /// The English printing is shown because the language read has none.
    let isEnglishPrinting: Bool
}

/// What the screen shows for a retained reading.
enum LookupState: Equatable, Sendable {
    case loading
    case found(LookupMatch)
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
        var outcome = LookupState.notFound
        for step in ScryfallLookup.plan(for: reading) {
            outcome = await fetch(step)
            if outcome != .notFound { break }
        }
        if case .failed = outcome {} else { cache[reading] = outcome }
        return outcome
    }

    private func fetch(_ step: LookupStep) async -> LookupState {
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
            return .found(LookupMatch(card: card, isEnglishPrinting: step.isEnglishPrinting))
        } catch {
            return .failed(error.localizedDescription)
        }
    }
}
