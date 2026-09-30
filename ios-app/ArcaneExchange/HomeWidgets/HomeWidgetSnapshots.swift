import Foundation

/// What the « Ma collection » widget shows: the collection value over the last 30 days, and
/// the card counts of `GET /collection/stats`. Mirrors the web home page's collection tile.
struct CollectionSnapshot: Codable, Equatable {
    /// The collection value history, oldest first, in euros.
    let points: [PricePoint]
    let totalCards: Int
    let uniqueCards: Int

    init(history: [PriceHistoryEntry], totalCards: Int, uniqueCards: Int) {
        points = PriceHistorySeries.points(from: history)
        self.totalCards = totalCards
        self.uniqueCards = uniqueCards
    }

    /// Today's collection value: the trend of the most recent day, like the web tile.
    var valueCents: Int {
        points.last.map { Self.cents($0.trend) } ?? 0
    }

    /// The web tile draws no graph below two points.
    var hasEnoughHistory: Bool {
        points.count >= 2
    }

    /// Last trend against the first one — the web's `computeVariation`.
    var variation: ValueVariation {
        guard let first = points.first, let last = points.last, points.count >= 2 else {
            return ValueVariation(deltaCents: 0, percent: 0)
        }
        let firstCents = Self.cents(first.trend)
        let lastCents = Self.cents(last.trend)
        let percent = firstCents == 0 ? 0 : Double(lastCents - firstCents) / Double(firstCents) * 100
        return ValueVariation(deltaCents: lastCents - firstCents, percent: percent)
    }

    private static func cents(_ euros: Double) -> Int {
        Int((euros * 100).rounded())
    }
}

/// How much the collection value moved over the period.
struct ValueVariation: Equatable {
    let deltaCents: Int
    let percent: Double

    /// Flat counts as rising, as on the web: the tile never shows a red « 0 € ».
    var isRising: Bool {
        percent >= 0
    }

    /// « ▴ 42,1 € · +3,5 % (30 j) », the web tile's wording.
    var label: String {
        let arrow = isRising ? "▴" : "▾"
        let sign = isRising ? "+" : "−"
        let amount = Price.euros(cents: abs(deltaCents))
        // Half away from zero, like the web's `toLocaleString`: 1,25 reads 1,3, not 1,2.
        let pct = abs(percent).formatted(
            .number
                .precision(.fractionLength(1))
                .rounded(rule: .toNearestOrAwayFromZero)
                .locale(Locale(identifier: "fr_FR"))
        )
        return "\(arrow) \(amount) · \(sign)\(pct) % (30 j)"
    }
}

/// What the « Échanges en cours » widget shows: how many trades are active, and the first few.
struct TradesSnapshot: Codable, Equatable {
    /// Rows that fit a medium widget under the header, subtitle included.
    static let rowCount = 2

    let total: Int
    let trades: [ActiveTrade]
}

/// One row of the trades widget — the fields of `TradeSummaryResponse` it displays.
struct ActiveTrade: Codable, Equatable, Identifiable {
    let id: String
    let partnerUsername: String
    let myCardCount: Int
    let partnerCardCount: Int
    let status: String

    /// « 2 données · 1 reçue », the web row's subtitle.
    var cardCounts: String {
        let given = myCardCount > 1 ? "\(myCardCount) données" : "\(myCardCount) donnée"
        let received = partnerCardCount > 1 ? "\(partnerCardCount) reçues" : "\(partnerCardCount) reçue"
        return "\(given) · \(received)"
    }
}
