import Foundation

/// The two home-screen widgets. The raw value is the WidgetKit `kind`, and the key of the
/// widget's last snapshot in `HomeWidgetCache`.
enum HomeWidgetKind: String, CaseIterable {
    case collection = "CollectionWidget"
    case trades = "TradesWidget"
}

/// What a widget has to display.
enum HomeWidgetContent<Snapshot: Equatable>: Equatable {
    case snapshot(Snapshot)
    /// Nobody is signed in to the app.
    case signedOut
    /// The call failed and no earlier snapshot is on the device.
    case unavailable
}

/// The widgets' last good snapshots, kept in the App Group so a failed refresh (offline, an
/// expired session) keeps showing them instead of an error.
struct HomeWidgetCache {
    let defaults: UserDefaults

    init(defaults: UserDefaults = SharedContainer.defaults) {
        self.defaults = defaults
    }

    func save(_ snapshot: some Encodable, for kind: HomeWidgetKind) {
        guard let data = try? JSONEncoder().encode(snapshot) else { return }
        defaults.set(data, forKey: Self.key(kind))
    }

    func load<Snapshot: Decodable>(_: Snapshot.Type, for kind: HomeWidgetKind) -> Snapshot? {
        defaults.data(forKey: Self.key(kind)).flatMap { try? JSONDecoder().decode(Snapshot.self, from: $0) }
    }

    func clear() {
        for kind in HomeWidgetKind.allCases {
            defaults.removeObject(forKey: Self.key(kind))
        }
    }

    static func key(_ kind: HomeWidgetKind) -> String {
        "home_widget_snapshot.\(kind.rawValue)"
    }
}

enum HomeWidgetLoader {
    /// Fetches a fresh snapshot and remembers it, or falls back to the last one remembered.
    ///
    /// Signed out, the API is not called at all: the widget asks to sign in in the app.
    static func load<Snapshot: Codable & Equatable>(
        _ kind: HomeWidgetKind,
        cache: HomeWidgetCache,
        isSignedIn: Bool,
        fetch: () async throws -> Snapshot
    ) async -> HomeWidgetContent<Snapshot> {
        guard isSignedIn else { return .signedOut }
        do {
            let snapshot = try await fetch()
            cache.save(snapshot, for: kind)
            return .snapshot(snapshot)
        } catch {
            return cache.load(Snapshot.self, for: kind).map { .snapshot($0) } ?? .unavailable
        }
    }
}
