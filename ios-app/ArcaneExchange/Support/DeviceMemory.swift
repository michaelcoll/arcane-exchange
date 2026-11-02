import Foundation

/// A key the device remembers for the signed-in player: sign-out forgets every case, so adding
/// one is all it takes to have it erased. Stored names are pinned by `DeviceMemoryTests`.
enum PlayerKey: String, CaseIterable {
    case collectionFilters = "collection.filters"
    case searchSortText = "search.sort.text"
    case searchSortPlayerCollection = "search.sort.player"
    case searchRecentCardQueries = "search.recentCardQueries"
    case searchRecentPlayers = "search.recentPlayers"
    // Kept in the App Group, which the widgets extension reads with the same codec.
    case collectionWidgetSnapshot = "home_widget_snapshot.CollectionWidget"
    case tradesWidgetSnapshot = "home_widget_snapshot.TradesWidget"
}

/// The one place local data is read, written and forgotten, as JSON in an injected
/// `UserDefaults` (the standard one, or the App Group's). Foundation only, so the widgets
/// extension can compile it too.
///
/// A value that is absent or unreadable reads as `nil`, and a failed encoding leaves the previous
/// value in place.
struct DeviceMemory {
    let defaults: UserDefaults

    func read<T: Decodable>(_: T.Type, for key: PlayerKey) -> T? {
        guard let data = defaults.data(forKey: key.rawValue) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }

    func write(_ value: some Encodable, for key: PlayerKey) {
        guard let data = try? JSONEncoder().encode(value) else { return }
        defaults.set(data, forKey: key.rawValue)
    }

    func remove(_ key: PlayerKey) {
        defaults.removeObject(forKey: key.rawValue)
    }

    /// Removes the keys one by one: a container also holds device-scope and library keys. Removing
    /// a key that lives in the other container is a no-op.
    func forgetPlayerScope() {
        for key in PlayerKey.allCases {
            remove(key)
        }
    }
}
