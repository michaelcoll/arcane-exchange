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

    /// Switching to a segment with no row loaded fills it from the following pages, without
    /// reloading page 0.
    var segment: Segment = .ongoing {
        didSet {
            guard segment != oldValue else { return }
            Task { await fillVisibleSegment() }
        }
    }

    /// The loaded trades (both segments) and their loading state.
    let paginator: Paginator<TradeSummary>

    init(api: any APIProtocol) {
        paginator = Paginator(fetch: { page, pageSize in
            let query = Operations.list_trades.Input.Query(page: page, page_size: pageSize)
            let response = try await api.list_trades(query: query).ok.body.json
            return PaginatorPage(items: response.items, total: Int(response.total))
        })
    }

    var ongoing: [TradeSummary] {
        paginator.items.filter { TradeStatus(apiValue: $0.status).isOngoing }
    }

    var past: [TradeSummary] {
        paginator.items.filter { !TradeStatus(apiValue: $0.status).isOngoing }
    }

    var visibleTrades: [TradeSummary] {
        segment == .ongoing ? ongoing : past
    }

    /// Reloads from page 0 — the first load, pull-to-refresh, and the return from a trade
    /// whose status may just have changed.
    func reload() async {
        await paginator.reload()
        await fillVisibleSegment()
    }

    /// Asks for the next page when `trade` just appeared among the last rows of the displayed
    /// segment, which may not be the last rows loaded.
    func loadMore(after trade: TradeSummary) async {
        await paginator.loadMore(after: trade, among: visibleTrades)
        await fillVisibleSegment()
    }

    /// Loads further pages while the displayed segment has no row and the feed has more. Called
    /// after each page applied and on every segment change.
    private func fillVisibleSegment() async {
        while visibleTrades.isEmpty, paginator.state == .idle, paginator.hasMore {
            let loaded = paginator.items.count
            await paginator.loadMore()
            guard paginator.items.count > loaded else { return }
        }
    }
}
