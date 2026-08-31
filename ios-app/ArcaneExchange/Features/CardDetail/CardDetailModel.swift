import APIClient
import Foundation

/// Short names for the generated schema types the card-detail screens work with.
typealias PriceHistoryEntry = Components.Schemas.PriceHistoryEntryResponse
typealias CardOffer = Components.Schemas.CardOfferResponse
typealias CollectionEntry = Components.Schemas.CollectionEntryResponse

/// Singular French rarity names, the way the card-detail header spells them
/// (`RarityCode.label` carries the plural forms the filter sheet lists).
enum RarityName {
    static func singular(_ code: String) -> String {
        switch code.uppercased() {
        case "C": "commune"
        case "U": "peu commune"
        case "R": "rare"
        case "M": "mythique"
        case "S": "spéciale"
        default: code.lowercased()
        }
    }
}

/// One day of the Cardmarket price guide, in euros — the shape `PriceHistoryChart` plots.
struct PricePoint: Identifiable, Equatable {
    let date: Date
    let low: Double
    let avg: Double
    let trend: Double

    var id: Date {
        date
    }
}

/// Turns `GET /card/{id}/price-history` (cents, `YYYY-MM-DD` strings) into chart-ready
/// points and the values the chart derives from them. Free of SwiftUI so it stays testable.
enum PriceHistorySeries {
    private static let dateParser: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter
    }()

    /// Chart points, oldest first. Entries whose date does not parse are dropped rather
    /// than aborting the whole series.
    static func points(from entries: [PriceHistoryEntry]) -> [PricePoint] {
        entries
            .compactMap { entry -> PricePoint? in
                guard let date = dateParser.date(from: entry.date) else { return nil }
                return PricePoint(
                    date: date,
                    low: Double(entry.low) / 100,
                    avg: Double(entry.avg) / 100,
                    trend: Double(entry.trend) / 100
                )
            }
            .sorted { $0.date < $1.date }
    }

    /// Y range covering every plotted value — `low`, `avg` *and* `trend`, since the trend line
    /// is not bounded by the low/avg band and would otherwise run off the chart — padded 12 %
    /// top and bottom (a little slack also absorbs Catmull-Rom overshoot).
    static func yDomain(for points: [PricePoint]) -> ClosedRange<Double> {
        let values = points.flatMap { [$0.low, $0.avg, $0.trend] }
        guard let low = values.min(), let high = values.max(), high >= low else {
            return 0 ... 1
        }
        let span = high - low
        let pad = span > 0 ? span * 0.12 : max(1, high * 0.1)
        let bottom = max(0, low - pad)
        let top = high + pad
        return bottom ... top
    }

    /// The point nearest a scrub position, for the selection tooltip.
    static func nearest(to date: Date, in points: [PricePoint]) -> PricePoint? {
        points.min {
            abs($0.date.timeIntervalSince(date)) < abs($1.date.timeIntervalSince(date))
        }
    }
}
