import Observation

/// App-wide navigation state a widget tap can drive: the selected tab, and a trade to push
/// once the Échanges tab is on screen.
@MainActor
@Observable
package final class AppRouter {
    /// The tabs of the root navigation, named so it does not shadow SwiftUI's `Tab` view type.
    package enum Destination: Hashable {
        case collection
        case trades
        case search
    }

    package var tab: Destination = .collection

    /// Consumed by `TradesView`, which owns the stack the trade is pushed onto.
    package var pendingTrade: TradeDetailRoute?

    package init() {}

    package func open(_ link: HomeWidgetLink) {
        switch link {
        case .collection:
            tab = .collection
            pendingTrade = nil
        case .trades:
            tab = .trades
            pendingTrade = nil
        case let .trade(route):
            tab = .trades
            pendingTrade = route
        }
    }
}
