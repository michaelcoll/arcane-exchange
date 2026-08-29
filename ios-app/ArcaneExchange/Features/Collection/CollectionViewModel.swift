import APIClient
import Foundation
import OpenAPIRuntime

/// Backs the Collection tab: one page-by-page `GET /collection` feed, plus the set list from
/// `GET /collection/stats` that the filter sheet needs.
@MainActor
@Observable
final class CollectionViewModel {
    /// Everything the screen needs to say about a failed load, in the user's terms.
    enum LoadError: Equatable {
        case unauthorized
        case network
        case http(Int)
        case unexpected

        var message: String {
            switch self {
            case .unauthorized:
                "Session expirée. Reconnecte-toi pour accéder à ta collection."
            case .network:
                "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange."
            case let .http(status):
                "Le serveur a répondu \(status)."
            case .unexpected:
                "Réponse inattendue du serveur."
            }
        }
    }

    /// Matches the mockup's "pagination API de 20 cartes, chargées au défilement".
    private static let pageSize: Int32 = 20

    var filters = CollectionFilters()

    private(set) var cards: [CollectionCard] = []
    private(set) var total = 0
    private(set) var sets: [SetInfo] = []
    private(set) var isLoading = false
    private(set) var isLoadingMore = false
    private(set) var loadError: LoadError?

    /// Index of the page the next `loadMore` should ask for. Doubles as a generation counter:
    /// `reload` resets it to 0, which lets an in-flight `loadMore` notice its page is stale.
    private var nextPage: Int32 = 0

    var hasMore: Bool {
        cards.count < total
    }

    /// Reloads from page 0. Called on appear and whenever `filters` changes.
    func reload() async {
        isLoading = true
        loadError = nil
        nextPage = 0
        // Whatever is still queued was warmed for the previous result set.
        ArtworkPipeline.cancelPrefetching()
        do {
            let response = try await fetchPage(0)
            cards = response.items
            total = Int(response.total)
            nextPage = 1
            ArtworkPipeline.prefetch(CardArtwork.urls(for: response.items))
        } catch {
            cards = []
            total = 0
            loadError = Self.loadError(from: error)
        }
        isLoading = false
    }

    /// Appends the next page once the user scrolls near the end of what is loaded.
    ///
    /// The page doubles as the prefetch window: this fires four cards from the end, so the
    /// twenty that come back are exactly the ones just below the fold, and warming them here
    /// needs no separate viewport tracking.
    func loadMoreIfNeeded(displaying card: CollectionCard) async {
        guard !isLoading, !isLoadingMore, hasMore, loadError == nil else { return }
        guard cards.suffix(4).contains(card) else { return }

        let page = nextPage
        isLoadingMore = true
        defer { isLoadingMore = false }
        do {
            let response = try await fetchPage(page)
            // A filter change may have restarted pagination while this request was in flight.
            guard nextPage == page else { return }
            cards.append(contentsOf: response.items)
            total = Int(response.total)
            nextPage = page + 1
            ArtworkPipeline.prefetch(CardArtwork.urls(for: response.items))
        } catch {
            loadError = Self.loadError(from: error)
        }
    }

    /// Sets owned by the user, for the filter sheet. Loaded once, and silently: the sheet can
    /// live without the list (rarities still work), so a failure here must not blank the grid.
    func loadSetsIfNeeded() async {
        guard sets.isEmpty else { return }
        guard let output = try? await APIClientProvider.shared.get_collection_stats(),
              case let .ok(response) = output,
              let stats = try? response.body.json
        else {
            return
        }
        sets = stats.sets.sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
    }

    private func fetchPage(_ page: Int32) async throws -> Components.Schemas.PaginatedCollectionResponse {
        let query = Operations.get_collection.Input.Query(
            page: page,
            page_size: Self.pageSize,
            sort_by: filters.sortBy,
            sort_dir: filters.sortDir,
            // Repeated `rarity=` params, in the schema's own order for a stable URL.
            rarity: filters.rarities.isEmpty ? nil : RarityCode.allCases.filter(filters.rarities.contains),
            sets: filters.sets.isEmpty ? nil : filters.sets.sorted().joined(separator: ",")
        )
        switch try await APIClientProvider.shared.get_collection(query: query) {
        case let .ok(response):
            return try response.body.json
        case .unauthorized:
            throw APIClientError.unauthorized
        case .badRequest:
            throw APIClientError.undocumented(statusCode: 400)
        case let .undocumented(statusCode, _):
            throw APIClientError.undocumented(statusCode: statusCode)
        }
    }

    private static func loadError(from error: Error) -> LoadError {
        if let apiError = error as? APIClientError {
            switch apiError {
            case .unauthorized: return .unauthorized
            case let .undocumented(statusCode): return .http(statusCode)
            }
        }
        // The transport wraps URLSession failures — a wrong base URL or no network lands here.
        let underlying = (error as? ClientError)?.underlyingError ?? error
        return underlying is URLError ? .network : .unexpected
    }
}
