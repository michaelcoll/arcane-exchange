import Foundation

/// A key the device remembers for the signed-in player: sign-out forgets every case, so adding
/// one is all it takes to have it erased. Stored names are pinned by `DeviceMemoryTests`.
enum PlayerKey: String, CaseIterable {
    case collectionFilters = "collection.filters"
    case searchSortText = "search.sort.text"
    case searchSortPlayerCollection = "search.sort.player"
    case searchRecentCardQueries = "search.recentCardQueries"
    case searchRecentPlayers = "search.recentPlayers"
    case addCardLanguage = "collection.addCard.language"
    // Kept in the App Group, which the widgets extension reads with the same codec.
    case collectionWidgetSnapshot = "home_widget_snapshot.CollectionWidget"
    case tradesWidgetSnapshot = "home_widget_snapshot.TradesWidget"
}

/// A key that belongs to the device, not to a player: it survives sign-out. Stored names are
/// pinned by `DeviceMemoryTests`.
enum DeviceKey: String, CaseIterable {
    /// Public data: the images of the login screen's showcase.
    case showcaseImagePaths = "showcase_image_paths"
}

/// A device-scope name that `DeviceMemory` cannot reach: its value is a raw string, not JSON,
/// because the iOS Settings app reads and writes it from `Settings.bundle`. It survives
/// sign-out. Read with `UserDefaults` directly (see `AppConfig`); the name is pinned by
/// `DeviceMemoryTests`.
enum RawDeviceKey: String, CaseIterable {
    /// Read in both containers, and also written to the App Group by `AppConfig.shareAPIBaseURL`.
    case apiBaseURL = "api_base_url"
}

/// The one place local data is read, written and forgotten, as JSON in an injected
/// `UserDefaults` (the standard one, or the App Group's). Foundation only, so the widgets
/// extension can compile it too.
///
/// A value that is absent or unreadable reads as `nil`, and a failed encoding leaves the previous
/// value in place.
struct DeviceMemory {
    let defaults: UserDefaults

    func read<T: Decodable>(_ type: T.Type, for key: PlayerKey) -> T? {
        read(type, named: key.rawValue)
    }

    func read<T: Decodable>(_ type: T.Type, for key: DeviceKey) -> T? {
        read(type, named: key.rawValue)
    }

    func write(_ value: some Encodable, for key: PlayerKey) {
        write(value, named: key.rawValue)
    }

    func write(_ value: some Encodable, for key: DeviceKey) {
        write(value, named: key.rawValue)
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

    private func read<T: Decodable>(_: T.Type, named name: String) -> T? {
        guard let data = defaults.data(forKey: name) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }

    private func write(_ value: some Encodable, named name: String) {
        guard let data = try? JSONEncoder().encode(value) else { return }
        defaults.set(data, forKey: name)
    }
}
