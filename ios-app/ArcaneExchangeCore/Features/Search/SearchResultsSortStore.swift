import APIClient
import Foundation

/// Local, device-only memory of the sort picked on the search results screens, restored at
/// launch and wiped by `SignOutCleanup`.
///
/// Two independent settings: one for text searches, one for a player's collection — the same
/// for every player. Only the sort is kept: the filters start empty on each new search.
package struct SearchResultsSortStore {
    private struct Sort: Codable {
        var sortBy: SortField
        var sortDir: SortDirection
    }

    private let memory: DeviceMemory

    package init(defaults: UserDefaults = .standard) {
        memory = DeviceMemory(defaults: defaults)
    }

    /// `target`'s defaults with the remembered sort, if any. A remembered criterion the screen
    /// does not offer falls back to the screen's default.
    package func load(for target: SearchResultsRoute.Target) -> CollectionFilters {
        var filters = target.defaultFilters
        guard let key = Self.key(for: target),
              let sort = memory.read(Sort.self, for: key)
        else {
            return filters
        }
        filters.sortBy = target.offeredSort(sort.sortBy)
        filters.sortDir = sort.sortDir
        return filters
    }

    package func save(_ filters: CollectionFilters, for target: SearchResultsRoute.Target) {
        guard let key = Self.key(for: target) else { return }
        memory.write(Sort(sortBy: filters.sortBy, sortDir: filters.sortDir), for: key)
    }

    private static func key(for target: SearchResultsRoute.Target) -> PlayerKey? {
        switch target {
        case .card: .searchSortText
        case .player: .searchSortPlayerCollection
        case .decklist: nil
        }
    }
}
