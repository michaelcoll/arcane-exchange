import APIClient
import Foundation
import OpenAPIRuntime

/// Backs the Rechercher tab: a debounced live preview for card and player search, plus the
/// two device-only recents lists. The full paginated card grid is `SearchResultsViewModel`,
/// one per pushed results screen.
@MainActor
@Observable
final class SearchViewModel {
    /// How many cards the as-you-type preview asks for. The full list lives behind a submit.
    private static let previewPageSize: Int32 = 8

    /// Delay both live lookups wait out before hitting the network.
    private static let debounce: Duration = .milliseconds(300)

    /// Waits out the debounce window from a `.task(id:)` context. Returns `false` when a newer
    /// keystroke has already superseded this call (its task was cancelled during the sleep).
    private static func settled() async -> Bool {
        try? await Task.sleep(for: debounce)
        return !Task.isCancelled
    }

    private(set) var cardPreview: [CollectionCard] = []
    private(set) var isLoadingCardPreview = false
    private(set) var cardPreviewFailed = false

    private(set) var playerSuggestions: [UserSuggestion] = []
    private(set) var isLoadingPlayers = false
    private(set) var playerLookupFailed = false

    private(set) var recentQueries: [String]
    private(set) var recentPlayers: [UserSuggestion]

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        recentQueries = Self.decode([String].self, from: defaults, key: SearchRecents.storageKey) ?? []
        recentPlayers = Self.decode([UserSuggestion].self, from: defaults, key: RecentPlayers.storageKey) ?? []
    }

    // MARK: Live card preview

    /// Debounced: called from `.task(id:)`, so a new keystroke cancels the pending sleep.
    func previewCards(matching raw: String) async {
        let query = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            cardPreview = []
            cardPreviewFailed = false
            return
        }
        guard await Self.settled() else { return }

        isLoadingCardPreview = true
        cardPreviewFailed = false
        defer { isLoadingCardPreview = false }

        do {
            let output = try await APIClientProvider.shared.search_cards(query: .init(
                page: 0,
                page_size: Self.previewPageSize,
                sort_by: .trend,
                sort_dir: .desc,
                q: query
            ))
            switch output {
            case let .ok(response):
                cardPreview = try response.body.json.items
            case .unauthorized, .badRequest, .undocumented:
                cardPreview = []
                cardPreviewFailed = true
            }
        } catch {
            cardPreview = []
            cardPreviewFailed = true
        }
    }

    // MARK: Player autocomplete

    /// Debounced: called from `.task(id:)`, so a new keystroke cancels the pending sleep.
    func lookUpPlayers(matching raw: String) async {
        let query = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !query.isEmpty else {
            playerSuggestions = []
            playerLookupFailed = false
            return
        }
        guard await Self.settled() else { return }

        isLoadingPlayers = true
        playerLookupFailed = false
        defer { isLoadingPlayers = false }

        do {
            switch try await APIClientProvider.shared.autocomplete_user(query: .init(q: query)) {
            case let .ok(response):
                playerSuggestions = try response.body.json
            case .undocumented:
                playerSuggestions = []
                playerLookupFailed = true
            }
        } catch {
            playerSuggestions = []
            playerLookupFailed = true
        }
    }

    // MARK: Recents

    func rememberQuery(_ query: String) {
        recentQueries = SearchRecents.adding(query, to: recentQueries)
        persist(recentQueries, key: SearchRecents.storageKey)
    }

    func removeQueries(_ toRemove: Set<String>) {
        recentQueries.removeAll { toRemove.contains($0) }
        persist(recentQueries, key: SearchRecents.storageKey)
    }

    func clearQueries() {
        recentQueries = []
        persist(recentQueries, key: SearchRecents.storageKey)
    }

    func rememberPlayer(_ player: UserSuggestion) {
        recentPlayers = RecentPlayers.adding(player, to: recentPlayers)
        persist(recentPlayers, key: RecentPlayers.storageKey)
    }

    // MARK: Persistence

    private func persist(_ value: some Encodable, key: String) {
        defaults.set(try? JSONEncoder().encode(value), forKey: key)
    }

    private static func decode<T: Decodable>(_: T.Type, from defaults: UserDefaults, key: String) -> T? {
        guard let data = defaults.data(forKey: key) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }
}
