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

    /// What the search is scoped to, which is also all the set facet is asked for: the filters
    /// never reach it, so the drawer's list stays put while the user filters. A text search asks
    /// for the cards any player offers (the Offre axis), a player's collection for the cards
    /// that player offers. `nil` for a target the endpoints do not serve.
    var setsQuery: Operations.search_card_sets.Input.Query? {
        switch self {
        case let .card(text): .init(q: text, offered: true)
        case let .player(username): .init(offered_by: username)
        case .decklist: nil
        }
    }

    /// One page of results for `filters` within the search's scope, or `nil` for a target the
    /// endpoint does not serve.
    func query(
        filters: CollectionFilters,
        page: Int32,
        pageSize: Int32
    ) -> Operations.search_cards.Input.Query? {
        guard let scope = setsQuery else { return nil }
        return .init(
            page: page,
            page_size: pageSize,
            sort_dir: filters.sortDir,
            q: scope.q,
            offered: scope.offered,
            offered_by: scope.offered_by,
            rarity: filters.rarityQuery,
            sets: filters.setsQuery
        )
    }
}
