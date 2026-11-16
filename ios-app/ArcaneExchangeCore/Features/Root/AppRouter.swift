import Observation

/// App-wide navigation state a widget tap or an App Intent can drive: the selected tab, and a
/// trade or an impression to push once its tab is on screen.
@MainActor
@Observable
package final class AppRouter {
    /// The tabs of the root navigation, named so it does not shadow SwiftUI's `Tab` view type.
    package enum Destination: Hashable {
        case collection
        case scanner
        case trades
        case search
    }

    package var tab: Destination = .collection

    /// Consumed by `TradesView`, which owns the stack the trade is pushed onto.
    package var pendingTrade: TradeDetailRoute?

    /// An impression to show, and which of its pages.
    package enum PendingImpression: Equatable {
        case detail(ImpressionKey)
        case offers(ImpressionKey)

        package var key: ImpressionKey {
            switch self {
            case let .detail(key), let .offers(key): key
            }
        }
    }

    /// Consumed by `SearchView`, which looks the impression up again and pushes its page.
    package var pendingImpression: PendingImpression?

    package init() {}

    package func open(_ link: HomeWidgetLink) {
        pendingTrade = nil
        pendingImpression = nil
        switch link {
        case .collection:
            tab = .collection
        case .trades:
            tab = .trades
        case let .trade(route):
            tab = .trades
            pendingTrade = route
        case let .impression(key):
            tab = .search
            pendingImpression = .detail(key)
        case let .impressionOffers(key):
            tab = .search
            pendingImpression = .offers(key)
        }
    }
}
