import APIClient

/// How each search results screen sorts, and what it asks `GET /search/card` and
/// `GET /search/card/sets` for.
///
/// Kept off the views on purpose: a `View` is `@MainActor`, and pure logic hung on one crashes
/// the non-isolated tests calling it.
package extension SearchResultsRoute.Target {
    /// The criteria the sort menu offers: the search sorts by reference price only. The date
    /// added belongs to the Collection, which is not a search.
    var sortOptions: [SortField] {
        [.trend]
    }

    /// The sort a screen opens with: the most valuable cards first. The API sorts by price in
    /// the direction asked for (and, with a text, by name match first): it takes no sort field.
    var defaultFilters: CollectionFilters {
        CollectionFilters(sortBy: .trend, sortDir: .desc)
    }

    /// `sortBy` if the screen offers it, the screen's default otherwise: a sort remembered from
    /// an earlier version (the date added) falls back to the price.
    func offeredSort(_ sortBy: SortField) -> SortField {
        sortOptions.contains(sortBy) ? sortBy : defaultFilters.sortBy
    }

    /// Whether the screen lets the user pick the Offre axis: a text search does (the offered
    /// impressions or the whole Catalogue), a player's collection has the player for it.
    var offersOfferAxis: Bool {
        if case .card = self {
            true
        } else {
            false
        }
    }

    /// What the search is scoped to, which is also all the set facet is asked for: the text and
    /// the two axes. The filters never reach it, so the drawer's list stays put while the user
    /// filters. A text search asks for the cards any player offers, or for the whole Catalogue
    /// (the Offre axis), a player's collection for the cards that player offers. `nil` for a
    /// target the endpoints do not serve.
    func setsQuery(axes: SearchAxes = SearchAxes()) -> Operations.search_card_sets.Input.Query? {
        let owned = axes.possession.ownedQuery
        return switch self {
        case let .card(text): .init(q: text, owned: owned, offered: axes.offer == .offered ? true : nil)
        case let .player(username): .init(owned: owned, offered_by: username)
        case .decklist: nil
        }
    }

    /// One page of results for `filters` and the `axes` within the search's scope, or `nil` for
    /// a target the endpoint does not serve.
    func query(
        filters: CollectionFilters,
        axes: SearchAxes = SearchAxes(),
        page: Int32,
        pageSize: Int32
    ) -> Operations.search_cards.Input.Query? {
        guard let scope = setsQuery(axes: axes) else { return nil }
        return .init(
            page: page,
            page_size: pageSize,
            sort_dir: filters.sortDir,
            q: scope.q,
            owned: scope.owned,
            offered: scope.offered,
            offered_by: scope.offered_by,
            rarity: filters.rarityQuery,
            sets: filters.setsQuery
        )
    }
}
