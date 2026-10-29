import APIClient
import Foundation

/// One page-by-page `GET /search/card` feed, scoped either to a free-text query or to a
/// single player's tradable collection. Modelled on `CollectionViewModel`.
@MainActor
@Observable
final class SearchResultsViewModel {
    private static let pageSize: Int32 = 20

    let target: SearchResultsRoute.Target

    /// The sort picked on this screen. It lives as long as the model, i.e. while the screen
    /// stays on the navigation stack, so a round trip to a card detail keeps it. The sort is
    /// also remembered on the device for the next search of the same kind.
    var filters: CollectionFilters {
        didSet { sortStore.save(filters, for: target) }
    }

    private let api: any APIProtocol
    private let sortStore: SearchResultsSortStore

    private(set) var cards: [CollectionCard] = []
    private(set) var total = 0
    private(set) var isLoading = false
    private(set) var isLoadingMore = false
    /// French message of the last failed load, ready to show.
    private(set) var loadError: String?

    /// What the set drawer lists: the sets of every card this search can return, whatever the
    /// filters. Empty until loaded, which disables the drawer's Sets row.
    private(set) var sets: [SetInfo] = []
    private var isLoadingSets = false

    /// Next page to request.
    private var nextPage: Int32 = 0

    /// Bumped by every `load()`. A response is applied only while its generation is still the
    /// current one, so when the filters change mid-flight the latest load wins, whatever order
    /// the responses come back in.
    private var generation = 0

    var hasMore: Bool {
        cards.count < total
    }

    init(target: SearchResultsRoute.Target, api: any APIProtocol, defaults: UserDefaults = .standard) {
        self.target = target
        self.api = api
        sortStore = SearchResultsSortStore(defaults: defaults)
        filters = sortStore.load(for: target)
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
        generation += 1
        let current = generation
        isLoading = true
        loadError = nil
        nextPage = 0
        ArtworkPipeline.cancelPrefetching()
        do {
            let response = try await fetchPage(0)
            guard generation == current else { return }
            cards = response.items
            total = Int(response.total)
            nextPage = 1
            ArtworkPipeline.prefetch(CardArtwork.urls(for: response.items))
        } catch {
            guard generation == current else { return }
            cards = []
            total = 0
            loadError = APIError.read(error).message
        }
        isLoading = false
    }

    func loadMoreIfNeeded(displaying card: CollectionCard) async {
        guard !isLoading, !isLoadingMore, hasMore, loadError == nil, cards.suffix(4).contains(card) else { return }

        let current = generation
        let page = nextPage
        isLoadingMore = true
        defer { isLoadingMore = false }
        do {
            let response = try await fetchPage(page)
            guard generation == current else { return }
            cards.append(contentsOf: response.items)
            total = Int(response.total)
            nextPage = page + 1
            ArtworkPipeline.prefetch(CardArtwork.urls(for: response.items))
        } catch {
            guard generation == current else { return }
            loadError = APIError.read(error).message
        }
    }

    /// Loads the set list once per screen, since the filters never change it. A failure is
    /// silent: the grid stays usable, and the next appearance tries again.
    func loadSetsIfNeeded() async {
        guard sets.isEmpty, !isLoadingSets, let query = target.setsQuery else { return }
        isLoadingSets = true
        defer { isLoadingSets = false }
        guard let sets = try? await api.search_card_sets(query: query).ok.body.json else { return }
        self.sets = sets
    }

    private func fetchPage(_ page: Int32) async throws -> Components.Schemas.PaginatedCollectionResponse {
        guard let query = target.query(filters: filters, page: page, pageSize: Self.pageSize) else {
            // The view never instantiates this model for a decklist target.
            return .init(items: [], page: 0, page_size: Self.pageSize, total: 0)
        }

        return try await api.search_cards(query: query).ok.body.json
    }
}
