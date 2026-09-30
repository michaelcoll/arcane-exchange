import Observation

/// App-wide navigation state a widget tap can drive: the selected tab, and a trade to push
/// once the Échanges tab is on screen.
@MainActor
@Observable
final class AppRouter {
    var tab: RootTabView.Destination = .collection

    /// Consumed by `TradesView`, which owns the stack the trade is pushed onto.
    var pendingTrade: TradeDetailRoute?

    func open(_ link: HomeWidgetLink) {
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
