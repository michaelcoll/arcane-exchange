import APIClient
import Foundation

/// Local, device-only memory of the sort picked on the search results screens, restored at
/// launch and wiped by `SignOutCleanup`.
///
/// Two independent settings: one for text searches, one for a player's collection — the same
/// for every player. Only the sort is kept: the filters start empty on each new search.
struct SearchResultsSortStore {
    static let textSearchKey = "search.sort.text"
    static let playerCollectionKey = "search.sort.player"

    private struct Sort: Codable {
        var sortBy: SortField
        var sortDir: SortDirection
    }

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    /// `target`'s defaults with the remembered sort, if any. A remembered criterion the screen
    /// does not offer falls back to the screen's default.
    func load(for target: SearchResultsRoute.Target) -> CollectionFilters {
        var filters = target.defaultFilters
        guard let key = Self.key(for: target),
              let data = defaults.data(forKey: key),
              let sort = try? JSONDecoder().decode(Sort.self, from: data)
        else {
            return filters
        }
        if target.sortOptions.contains(sort.sortBy) {
            filters.sortBy = sort.sortBy
        }
        filters.sortDir = sort.sortDir
        return filters
    }

    func save(_ filters: CollectionFilters, for target: SearchResultsRoute.Target) {
        guard let key = Self.key(for: target) else { return }
        let sort = Sort(sortBy: filters.sortBy, sortDir: filters.sortDir)
        defaults.set(try? JSONEncoder().encode(sort), forKey: key)
    }

    func clear() {
        defaults.removeObject(forKey: Self.textSearchKey)
        defaults.removeObject(forKey: Self.playerCollectionKey)
    }

    private static func key(for target: SearchResultsRoute.Target) -> String? {
        switch target {
        case .card: textSearchKey
        case .player: playerCollectionKey
        case .decklist: nil
        }
    }
}

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
        // Schema order rather than the set's, so the same selection always gives the same URL.
        let rarity = filters.rarities.isEmpty ? nil : RarityCode.allCases.filter(filters.rarities.contains)
        switch self {
        case let .card(text):
            return .init(
                page: page,
                page_size: pageSize,
                sort_by: sortBy,
                sort_dir: filters.sortDir,
                q: text,
                rarity: rarity
            )
        case let .player(username):
            return .init(
                page: page,
                page_size: pageSize,
                sort_by: sortBy,
                sort_dir: filters.sortDir,
                rarity: rarity,
                player_username: username
            )
        case .decklist:
            return nil
        }
    }
}
