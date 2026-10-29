import APIClient
import Foundation

/// Backs the Collection tab: one page-by-page `GET /collection` feed, plus the set list from
/// `GET /collection/stats` that the filter sheet needs.
@MainActor
@Observable
final class CollectionViewModel {
    /// Restored from the previous launch, and saved again on every change.
    var filters: CollectionFilters {
        didSet { store.save(filters) }
    }

    /// The loaded cards and their loading state. Reload it whenever `filters` changes. Implicitly
    /// unwrapped on purpose: its `fetch` reads `filters`, so it is built in `init` once `self` is
    /// usable, and `@Observable` rules out `lazy` and a `let` assigned last.
    private(set) var paginator: Paginator<CollectionCard>!

    private(set) var sets: [SetInfo] = []

    /// True while the first load waits for the set list, so the grid shows its spinner rather
    /// than the "no match" screen.
    private(set) var isWaitingForSets = false

    /// The set-list request in flight. The first load and the view's own `.task` both ask for
    /// the sets: the second caller joins the first request instead of racing it.
    private var setsLoad: Task<Void, Never>?

    private let api: any APIProtocol
    private let store: CollectionFiltersStore

    init(api: any APIProtocol, defaults: UserDefaults = .standard) {
        self.api = api
        store = CollectionFiltersStore(defaults: defaults)
        filters = store.load()
        paginator = .cardGrid(owner: self) { model, page, pageSize in
            try await model.fetchPage(page, pageSize: pageSize)
        }
    }

    /// First load only. The view's `.task` re-runs every time the screen re-appears (e.g. after
    /// popping a card detail); without this guard that would reload page 0 and throw away both
    /// the already-loaded pages and the user's scroll position.
    ///
    /// A restored set filter waits for the set list first, so a set no longer owned is dropped
    /// before the grid is asked for — not after an empty first page.
    func loadInitiallyIfNeeded() async {
        guard paginator.items.isEmpty, paginator.state == .idle, !isWaitingForSets else { return }
        if !filters.sets.isEmpty {
            isWaitingForSets = true
            await loadSetsIfNeeded()
            isWaitingForSets = false
        }
        await paginator.reload()
    }

    /// The import may have replaced the whole collection: its sets included.
    func reloadAfterImport() async {
        // A list requested before the import finished is stale: wait it out, then ask again.
        await setsLoad?.value
        await loadSets()
        await paginator.reload()
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

    private func fetchPage(_ page: Int32, pageSize: Int32) async throws -> PaginatorPage<CollectionCard> {
        let query = Operations.get_collection.Input.Query(
            page: page,
            page_size: pageSize,
            sort_by: filters.sortBy,
            sort_dir: filters.sortDir,
            rarity: filters.rarityQuery,
            sets: filters.setsQuery
        )
        let response = try await api.get_collection(query: query).ok.body.json
        return PaginatorPage(items: response.items, total: Int(response.total))
    }
}
