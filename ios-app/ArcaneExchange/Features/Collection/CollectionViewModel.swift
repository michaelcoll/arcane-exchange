import APIClient
import Foundation

/// Backs the Collection tab: one page-by-page `GET /collection` feed, plus the set list from
/// `GET /collection/stats` that the filter sheet needs.
@MainActor
@Observable
final class CollectionViewModel {
    /// Matches the mockup's "pagination API de 20 cartes, chargées au défilement".
    private static let pageSize: Int32 = 20

    /// Restored from the previous launch, and saved again on every change.
    var filters: CollectionFilters {
        didSet { store.save(filters) }
    }

    private(set) var cards: [CollectionCard] = []
    private(set) var total = 0
    private(set) var sets: [SetInfo] = []
    private(set) var isLoading = false
    private(set) var isLoadingMore = false
    /// French message of the last failed load, ready to show.
    private(set) var loadError: String?

    /// Index of the page the next `loadMore` should ask for. Doubles as a generation counter:
    /// `reload` resets it to 0, which lets an in-flight `loadMore` notice its page is stale.
    private var nextPage: Int32 = 0

    /// The set-list request in flight. The first load and the view's own `.task` both ask for
    /// the sets: the second caller joins the first request instead of racing it.
    private var setsLoad: Task<Void, Never>?

    private let api: any APIProtocol
    private let store: CollectionFiltersStore

    var hasMore: Bool {
        cards.count < total
    }

    init(api: any APIProtocol, defaults: UserDefaults = .standard) {
        self.api = api
        store = CollectionFiltersStore(defaults: defaults)
        filters = store.load()
    }

    /// First load only. The view's `.task` re-runs every time the screen re-appears (e.g. after
    /// popping a card detail); without this guard that would reload page 0 and throw away both
    /// the already-loaded pages and the user's scroll position.
    ///
    /// A restored set filter waits for the set list first, so a set no longer owned is dropped
    /// before the grid is asked for — not after an empty first page.
    func loadInitiallyIfNeeded() async {
        guard cards.isEmpty, !isLoading, loadError == nil else { return }
        if !filters.sets.isEmpty {
            // The spinner, not the "no match" screen, while the set list comes back.
            isLoading = true
            await loadSetsIfNeeded()
        }
        await reload()
    }

    /// The import may have replaced the whole collection: its sets included.
    func reloadAfterImport() async {
        // A list requested before the import finished is stale: wait it out, then ask again.
        await setsLoad?.value
        await loadSets()
        await reload()
    }

    /// Reloads from page 0. Called for the first load and whenever `filters` changes.
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
            loadError = APIError.read(error).message
        }
        isLoading = false
    }

    /// Appends the next page once the user scrolls near the end of what is loaded.
    ///
    /// The page doubles as the prefetch window: this fires four cards from the end, so the
    /// twenty that come back are exactly the ones just below the fold, and warming them here
    /// needs no separate viewport tracking.
    func loadMoreIfNeeded(displaying card: CollectionCard) async {
        guard !isLoading, !isLoadingMore, hasMore, loadError == nil, cards.suffix(4).contains(card) else { return }

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
            loadError = APIError.read(error).message
        }
    }

    /// Sets owned by the user, for the filter sheet. Loaded once, and silently: the sheet can
    /// live without the list (rarities still work), so a failure here must not blank the grid.
    func loadSetsIfNeeded() async {
        guard sets.isEmpty else { return }
        await loadSets()
    }

    private func loadSets() async {
        if let setsLoad {
            return await setsLoad.value
        }
        let load = Task { await fetchSets() }
        setsLoad = load
        await load.value
        setsLoad = nil
    }

    /// Only a list that actually came back prunes the set filter: a failed request says nothing
    /// about what the collection holds.
    private func fetchSets() async {
        guard let stats = try? await api.get_collection_stats().ok.body.json else {
            return
        }
        sets = stats.sets.sorted { $0.name.localizedStandardCompare($1.name) == .orderedAscending }
        filters.dropSets(notIn: sets)
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
        return try await api.get_collection(query: query).ok.body.json
    }
}
