import APIClient
import Foundation

/// Backs the Échanges tab: one page-by-page `GET /trades` feed, split into the "En cours" and
/// "Historique" segments the mockup shows.
///
/// The split is done client-side rather than with the endpoint's `status` filter: the two
/// segments together are the whole list, so one feed serves both and switching tabs costs
/// nothing.
@MainActor
@Observable
final class TradesViewModel {
    enum Segment: String, CaseIterable, Identifiable {
        case ongoing
        case past

        var id: Self {
            self
        }

        var label: String {
            switch self {
            case .ongoing: "En cours"
            case .past: "Historique"
            }
        }
    }

    private static let pageSize: Int32 = 20

    private let api: any APIProtocol

    init(api: any APIProtocol) {
        self.api = api
    }

    var segment: Segment = .ongoing

    private(set) var trades: [TradeSummary] = []
    private(set) var total = 0
    private(set) var isLoading = false
    private(set) var isLoadingMore = false
    /// French message of the last failed load, ready to show.
    private(set) var loadError: String?

    /// Next page to request; doubles as a generation counter so a stale `loadMore` bails.
    private var nextPage: Int32 = 0

    var hasMore: Bool {
        trades.count < total
    }

    var ongoing: [TradeSummary] {
        trades.filter { TradeStatus(apiValue: $0.status).isOngoing }
    }

    var past: [TradeSummary] {
        trades.filter { !TradeStatus(apiValue: $0.status).isOngoing }
    }

    var visibleTrades: [TradeSummary] {
        segment == .ongoing ? ongoing : past
    }

    /// Reloads from page 0 — the first load, pull-to-refresh, and the return from a trade
    /// whose status may just have changed.
    func reload() async {
        isLoading = true
        loadError = nil
        nextPage = 0
        do {
            let response = try await fetchPage(0)
            trades = response.items
            total = Int(response.total)
            nextPage = 1
        } catch {
            trades = []
            total = 0
            loadError = APIError.read(error).message
        }
        isLoading = false
    }

    func loadMoreIfNeeded(displaying trade: TradeSummary) async {
        guard !isLoading, !isLoadingMore, hasMore, loadError == nil, trades.suffix(4).contains(trade) else { return }

        let page = nextPage
        isLoadingMore = true
        defer { isLoadingMore = false }
        do {
            let response = try await fetchPage(page)
            guard nextPage == page else { return }
            trades.append(contentsOf: response.items)
            total = Int(response.total)
            nextPage = page + 1
        } catch {
            loadError = APIError.read(error).message
        }
    }

    private func fetchPage(_ page: Int32) async throws -> Components.Schemas.PaginatedTradesResponse {
        let query = Operations.list_trades.Input.Query(page: page, page_size: Self.pageSize)
        return try await api.list_trades(query: query).ok.body.json
    }
}
