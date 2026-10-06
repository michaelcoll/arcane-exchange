import APIClient
import Foundation
import OpenAPIRuntime

/// One page-by-page `GET /search/card` feed, scoped either to a free-text query or to a
/// single player's tradable collection. Modelled on `CollectionViewModel`.
@MainActor
@Observable
final class SearchResultsViewModel {
    enum LoadError: Equatable {
        case unauthorized
        case network
        case http(Int)
        case unexpected

        var message: String {
            switch self {
            case .unauthorized:
                "Session expirée. Reconnecte-toi pour lancer une recherche."
            case .network:
                "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange."
            case let .http(status):
                "Le serveur a répondu \(status)."
            case .unexpected:
                "Réponse inattendue du serveur."
            }
        }
    }

    private static let pageSize: Int32 = 20

    let target: SearchResultsRoute.Target

    /// The sort picked on this screen. It lives as long as the model, i.e. while the screen
    /// stays on the navigation stack, so a round trip to a card detail keeps it.
    var filters: CollectionFilters

    private(set) var cards: [CollectionCard] = []
    private(set) var total = 0
    private(set) var isLoading = false
    private(set) var isLoadingMore = false
    private(set) var loadError: LoadError?

    /// Next page to request; doubles as a generation counter so a stale `loadMore` bails.
    private var nextPage: Int32 = 0

    var hasMore: Bool {
        cards.count < total
    }

    init(target: SearchResultsRoute.Target) {
        self.target = target
        filters = target.defaultFilters
    }

    /// First load only. The view's `.task` re-runs each time the screen re-appears (e.g. after
    /// popping a card detail); reloading then would throw away the loaded pages and the scroll
    /// position.
    func loadInitiallyIfNeeded() async {
        guard cards.isEmpty, !isLoading, loadError == nil else { return }
        await load()
    }

    /// Reloads from page 0. Called for the first load and whenever `filters` changes.
    func load() async {
        isLoading = true
        loadError = nil
        nextPage = 0
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

    func loadMoreIfNeeded(displaying card: CollectionCard) async {
        guard !isLoading, !isLoadingMore, hasMore, loadError == nil else { return }
        guard cards.suffix(4).contains(card) else { return }

        let page = nextPage
        isLoadingMore = true
        defer { isLoadingMore = false }
        do {
            let response = try await fetchPage(page)
            guard nextPage == page else { return }
            cards.append(contentsOf: response.items)
            total = Int(response.total)
            nextPage = page + 1
            ArtworkPipeline.prefetch(CardArtwork.urls(for: response.items))
        } catch {
            loadError = Self.loadError(from: error)
        }
    }

    private func fetchPage(_ page: Int32) async throws -> Components.Schemas.PaginatedCollectionResponse {
        guard let query = target.query(filters: filters, page: page, pageSize: Self.pageSize) else {
            // The view never instantiates this model for a decklist target.
            return .init(items: [], page: 0, page_size: Self.pageSize, total: 0)
        }

        switch try await APIClientProvider.shared.search_cards(query: query) {
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
        let underlying = (error as? ClientError)?.underlyingError ?? error
        return underlying is URLError ? .network : .unexpected
    }
}
