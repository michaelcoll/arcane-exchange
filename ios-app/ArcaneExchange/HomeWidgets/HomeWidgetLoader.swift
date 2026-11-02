import Foundation

/// The two home-screen widgets. The raw value is the WidgetKit `kind`; the key of the widget's
/// last snapshot comes from `PlayerKey`.
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
    private let memory: DeviceMemory

    init(defaults: UserDefaults = SharedContainer.defaults) {
        memory = DeviceMemory(defaults: defaults)
    }

    func save(_ snapshot: some Encodable, for kind: HomeWidgetKind) {
        memory.write(snapshot, for: kind.key)
    }

    func load<Snapshot: Decodable>(_: Snapshot.Type, for kind: HomeWidgetKind) -> Snapshot? {
        memory.read(Snapshot.self, for: kind.key)
    }
}

private extension HomeWidgetKind {
    var key: PlayerKey {
        switch self {
        case .collection: .collectionWidgetSnapshot
        case .trades: .tradesWidgetSnapshot
        }
    }
}

enum HomeWidgetLoader {
    /// Fetches a fresh snapshot and remembers it, or falls back to the last one remembered.
    ///
    /// Signed out, the API is not called at all: the widget asks to sign in in the app. A 401
    /// gives the same state.
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
            // The session expired: the widget asks to sign in again, it never signs the app out.
            if APIError.read(error).code == "unauthorized" {
                return .signedOut
            }
            return cache.load(Snapshot.self, for: kind).map { .snapshot($0) } ?? .unavailable
        }
    }
}
