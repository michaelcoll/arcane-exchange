import APIClient
import Foundation

/// The widgets' API calls: the same endpoints as the web home page's two tiles. They take the
/// client as `any APIProtocol` (ADR 0020), so tests run them against `FakeAPI`.
enum HomeWidgetFetch {
    /// The trade statuses the web tile lists — the active trades.
    private static let activeStatuses: [TradeStatusParam] = [.PENDING, .ONE_ACCEPTED, .FULLY_ACCEPTED]

    static func collection(api: any APIProtocol) async throws -> CollectionSnapshot {
        let range = CollectionSnapshot.historyRange(endingAt: .now)
        async let history = api.get_collection_price_history(
            query: .init(start_date: range.start, end_date: range.end)
        )
        async let stats = api.get_collection_stats()

        let entries: [PriceHistoryEntry] = try await history.ok.body.json
        let counts = try await stats.ok.body.json
        return CollectionSnapshot(
            history: entries,
            totalCards: Int(counts.total_cards),
            uniqueCards: Int(counts.unique_cards)
        )
    }

    static func trades(api: any APIProtocol) async throws -> TradesSnapshot {
        let query = Operations.list_trades.Input.Query(
            page: 0,
            page_size: Int32(TradesSnapshot.rowCount),
            status: activeStatuses
        )
        let page = try await api.list_trades(query: query).ok.body.json
        return TradesSnapshot(
            total: Int(page.total),
            trades: page.items.map { trade in
                ActiveTrade(
                    id: trade.id,
                    partnerUsername: trade.partner_username,
                    myCardCount: Int(trade.my_card_count),
                    partnerCardCount: Int(trade.partner_card_count),
                    status: TradeStatus(apiValue: trade.status)
                )
            }
        )
    }
}
