import SwiftUI

/// Routes pushed onto whichever `NavigationStack` shows a card list — Collection today,
/// Search once it exists. Both carry the whole card: the list already holds it, so the
/// detail and owners screens open without another round-trip.
struct CardDetailRoute: Hashable {
    let card: CollectionCard
}

struct CardOffersRoute: Hashable {
    let card: CollectionCard
}

extension View {
    /// Registers the card-browsing chain — a player or free-text search, a card, its owners —
    /// on the enclosing `NavigationStack`.
    ///
    /// Shared by the Rechercher and Échanges tabs, which both reach it. The Collection tab
    /// keeps its own registration: its card destination carries a zoom transition from the
    /// grid, which has no meaning from a rail or a search result.
    func cardBrowsingDestinations() -> some View {
        navigationDestination(for: SearchResultsRoute.self) { SearchResultsView(route: $0) }
            .navigationDestination(for: CardDetailRoute.self) { CardDetailView(card: $0.card) }
            .navigationDestination(for: CardOffersRoute.self) { CardOffersView(card: $0.card) }
    }
}
