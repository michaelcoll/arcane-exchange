#if DEBUG
    import Foundation

    /// Fixtures shared by the Échanges previews — one trade per status, so a preview shows the
    /// whole spread rather than a single happy path.
    extension TradeSummary {
        static let previewFeed: [TradeSummary] = [
            TradeSummary(
                id: "1",
                my_card_count: 2,
                partner_card_count: 1,
                partner_username: "mizzix_42",
                status: TradeStatus.pending.rawValue,
                updated_at: previewDate(minutesAgo: 30)
            ),
            TradeSummary(
                id: "2",
                my_card_count: 1,
                partner_card_count: 3,
                partner_username: "tanguy_a",
                status: TradeStatus.oneAccepted.rawValue,
                updated_at: previewDate(minutesAgo: 60 * 5)
            ),
            TradeSummary(
                id: "3",
                my_card_count: 4,
                partner_card_count: 4,
                partner_username: "lena_planeswalker",
                status: TradeStatus.fullyAccepted.rawValue,
                updated_at: previewDate(minutesAgo: 60 * 26)
            ),
            TradeSummary(
                id: "4",
                my_card_count: 1,
                partner_card_count: 1,
                partner_username: "sarkhan_vol",
                status: TradeStatus.closed.rawValue,
                updated_at: previewDate(minutesAgo: 60 * 24 * 9)
            ),
            TradeSummary(
                id: "5",
                my_card_count: 3,
                partner_card_count: 2,
                partner_username: "kaya_ghost",
                status: TradeStatus.abandoned.rawValue,
                updated_at: previewDate(minutesAgo: 60 * 24 * 21)
            )
        ]

        /// The RFC 3339 shape `TradeTimestamp` parses.
        private static func previewDate(minutesAgo: Int) -> String {
            let date = Date(timeIntervalSinceNow: -Double(minutesAgo) * 60)
            return ISO8601DateFormatter().string(from: date)
        }
    }
#endif
