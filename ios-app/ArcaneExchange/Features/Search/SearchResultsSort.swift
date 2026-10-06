import APIClient

/// How each search results screen sorts, and what it asks `GET /search/card` for.
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

    /// One page of results for `filters`, or `nil` for a target the endpoint does not serve.
    ///
    /// A criterion the screen does not offer falls back to the screen's default rather than
    /// reaching the API, where it could be a 400.
    func query(
        filters: CollectionFilters,
        page: Int32,
        pageSize: Int32
    ) -> Operations.search_cards.Input.Query? {
        let sortBy = sortOptions.contains(filters.sortBy) ? filters.sortBy : defaultFilters.sortBy
        switch self {
        case let .card(text):
            return .init(page: page, page_size: pageSize, sort_by: sortBy, sort_dir: filters.sortDir, q: text)
        case let .player(username):
            return .init(
                page: page,
                page_size: pageSize,
                sort_by: sortBy,
                sort_dir: filters.sortDir,
                player_username: username
            )
        case .decklist:
            return nil
        }
    }
}
