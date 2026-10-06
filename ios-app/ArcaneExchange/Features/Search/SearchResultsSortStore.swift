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
        filters.sortBy = target.offeredSort(sort.sortBy)
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
