import APIClient
import Foundation

/// The value of a scan batch: the trend prices of its lines, each in its finish and times its
/// quantity. A line with no trend price is left out of `cents` and counted in `unpricedLines`.
package struct BatchValue: Equatable, Sendable {
    package let cents: Int
    package let unpricedLines: Int
}

/// The trend prices of the cards of a scan batch, asked of `POST /catalog/trend-prices`. Prices are
/// per card (set, number), both finishes: a change of finish or quantity of a line reprices it with
/// no new request. Without the network the prices are hidden, with no
/// error: the scan goes on.
@MainActor
@Observable
package final class ScanBatchPrices {
    private struct CardKey: Hashable {
        let set: String
        let number: String
    }

    private struct Trend {
        let normal: Int?
        let foil: Int?
    }

    /// The API caps a request at 500 cards.
    private static let maxCards = 500

    /// Nil until a request succeeded, and again after one failed: the prices are hidden.
    private var trends: [CardKey: Trend]?
    private let api: (any APIProtocol)?

    package init(api: (any APIProtocol)?) {
        self.api = api
    }

    /// Asks for the cards of these lines. The most recent lines come first, so they are the ones
    /// kept past the cap of a request.
    package func refresh(for lines: [ScanBatchLine]) async {
        guard let api, !lines.isEmpty else { return }
        var seen = Set<CardKey>()
        let cards = lines.map { CardKey(set: $0.set, number: $0.number) }
            .filter { seen.insert($0).inserted }
            .prefix(Self.maxCards)
        let body = Components.Schemas.TrendPricesRequest(
            cards: cards.map { .init(collector_number: $0.number, set_code: $0.set) }
        )
        let answer = try? await api.get_trend_prices(.init(body: .json(body))).ok.body.json
        // A request cancelled by a newer one says nothing about the network.
        guard !Task.isCancelled else { return }
        guard let response = answer else {
            trends = nil
            return
        }
        trends = Dictionary(
            response.prices.map {
                (CardKey(set: $0.set_code.uppercased(), number: $0.collector_number),
                 Trend(normal: $0.trend_cents.map(Int.init), foil: $0.foil_trend_cents.map(Int.init)))
            },
            uniquingKeysWith: { first, _ in first }
        )
    }

    /// What a line is worth in its finish, times its quantity; nil when hidden or unknown.
    package func cents(of line: ScanBatchLine) -> Int? {
        guard let trend = trends?[CardKey(set: line.set, number: line.number)],
              let unit = line.foil ? trend.foil : trend.normal
        else { return nil }
        return unit * line.quantity
    }

    /// The value of the batch; nil when the prices are hidden.
    package func value(of batch: ScanBatch) -> BatchValue? {
        guard trends != nil else { return nil }
        let priced = batch.lines.compactMap { cents(of: $0) }
        return BatchValue(cents: priced.reduce(0, +), unpricedLines: batch.lines.count - priced.count)
    }
}
