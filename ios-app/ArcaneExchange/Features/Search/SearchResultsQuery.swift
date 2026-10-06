import APIClient

/// How each search results screen sorts, and what it asks `GET /search/card` and
/// `GET /search/card/sets` for.
///
/// Kept off the views on purpose: a `View` is `@MainActor`, and pure logic hung on one crashes
/// the non-isolated tests calling it.
extension SearchResultsRoute.Target {
    /// The criteria the sort menu offers. A text search sorts by value only, because the API
    /// rejects `sort_by=added_at` without `player_username`; a player's collection is a
    /// collection, so it gets the Collection tab's two.
    var sortOptions: [SortField] {
        switch self {
        case .player: SortField.collectionOptions
        case .card, .decklist: [.trend]
        }
    }

    /// The sort a screen opens with: the most valuable cards first for a text search, the
    /// latest additions first for a player's collection.
    var defaultFilters: CollectionFilters {
        switch self {
        case .player: CollectionFilters(sortBy: .added_at, sortDir: .desc)
        case .card, .decklist: CollectionFilters(sortBy: .trend, sortDir: .desc)
        }
    }

    /// `sortBy` if the screen offers it, the screen's default otherwise — so a criterion the
    /// screen does not offer never reaches the API, where it could be a 400.
    func offeredSort(_ sortBy: SortField) -> SortField {
        sortOptions.contains(sortBy) ? sortBy : defaultFilters.sortBy
    }

    /// What the search is scoped to, which is also all the set facet is asked for: the filters
    /// never reach it, so the drawer's list stays put while the user filters. `nil` for a
    /// target the endpoints do not serve.
    var setsQuery: Operations.search_card_sets.Input.Query? {
        switch self {
        case let .card(text): .init(q: text)
        case let .player(username): .init(player_username: username)
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
            sort_by: offeredSort(filters.sortBy),
            sort_dir: filters.sortDir,
            q: scope.q,
            // Schema order rather than the set's, so the same selection always gives the same URL.
            rarity: filters.rarities.isEmpty ? nil : RarityCode.allCases.filter(filters.rarities.contains),
            sets: filters.sets.isEmpty ? nil : filters.sets.sorted().joined(separator: ","),
            player_username: scope.player_username
        )
    }
}
