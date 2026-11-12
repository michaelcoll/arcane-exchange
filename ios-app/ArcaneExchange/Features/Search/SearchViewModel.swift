import APIClient
import ArcaneCore
import Foundation

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

    private let api: any APIProtocol
    private let memory: DeviceMemory

    init(api: any APIProtocol, defaults: UserDefaults = .standard) {
        self.api = api
        memory = DeviceMemory(defaults: defaults)
        recentQueries = memory.read([String].self, for: PlayerKey.searchRecentCardQueries) ?? []
        recentPlayers = memory.read([UserSuggestion].self, for: PlayerKey.searchRecentPlayers) ?? []
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
            let output = try await api.search_cards(query: .init(
                page: 0,
                page_size: Self.previewPageSize,
                sort_by: .trend,
                sort_dir: .desc,
                q: query
            ))
            cardPreview = try output.ok.body.json.items
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
            playerSuggestions = try await api.autocomplete_user(query: .init(q: query)).ok.body.json
        } catch {
            playerSuggestions = []
            playerLookupFailed = true
        }
    }

    // MARK: Recents

    func rememberQuery(_ query: String) {
        updateRecentQueries { $0 = SearchRecents.adding(query, to: $0) }
    }

    func removeQueries(_ toRemove: Set<String>) {
        updateRecentQueries { queries in queries.removeAll { toRemove.contains($0) } }
    }

    func clearQueries() {
        updateRecentQueries { $0 = [] }
    }

    /// The one place `recentQueries` changes and is remembered.
    private func updateRecentQueries(_ change: (inout [String]) -> Void) {
        change(&recentQueries)
        memory.write(recentQueries, for: PlayerKey.searchRecentCardQueries)
    }

    func rememberPlayer(_ player: UserSuggestion) {
        recentPlayers = RecentPlayers.adding(player, to: recentPlayers)
        memory.write(recentPlayers, for: PlayerKey.searchRecentPlayers)
    }
}
