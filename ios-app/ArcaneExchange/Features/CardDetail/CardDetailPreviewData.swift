#if DEBUG
    import ArcaneCore
    import Foundation

    /// Fixtures shared by the card-detail previews.
    extension PricePoint {
        /// Thirty days of a gently rising trend, with a low/avg band around it.
        static let previewSeries: [PricePoint] = {
            let start = Date(timeIntervalSince1970: 1_767_225_600) // 2026-01-01T00:00:00Z
            return (0 ..< 30).map { day in
                let trend = 12 + Double(day) * 0.05 + sin(Double(day) / 4)
                return PricePoint(
                    date: start.addingTimeInterval(Double(day) * 86400),
                    low: trend - 1.5 - Double(day % 5) * 0.2,
                    avg: trend + 1.2,
                    trend: trend
                )
            }
        }()
    }

    extension PriceGuide {
        static let preview = PriceGuide(avg: 1350, low: 1000, trend: 1240)
    }

    extension CollectionEntryDetail {
        static let preview = CollectionEntryDetail(
            added_at: "2026-01-05T10:00:00Z",
            foil: false,
            id: "preview-entry",
            purchase_price: 980,
            quantity: 3
        )
    }
#endif
