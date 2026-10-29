import APIClient
import Foundation

/// One page-by-page `GET /search/card` feed, scoped either to a free-text query or to a
/// single player's tradable collection. Modelled on `CollectionViewModel`.
@MainActor
@Observable
final class SearchResultsViewModel {
    let target: SearchResultsRoute.Target

    /// The sort picked on this screen. It lives as long as the model, i.e. while the screen
    /// stays on the navigation stack, so a round trip to a card detail keeps it. The sort is
    /// also remembered on the device for the next search of the same kind.
    var filters: CollectionFilters {
        didSet { sortStore.save(filters, for: target) }
    }

    private let api: any APIProtocol
    private let sortStore: SearchResultsSortStore

    /// The loaded cards and their loading state. Reload it whenever `filters` changes. Implicitly
    /// unwrapped on purpose: its `fetch` reads `filters`, so it is built in `init` once `self` is
    /// usable, and `@Observable` rules out `lazy` and a `let` assigned last.
    private(set) var paginator: Paginator<CollectionCard>!

    /// What the set drawer lists: the sets of every card this search can return, whatever the
    /// filters. Empty until loaded, which disables the drawer's Sets row.
    private(set) var sets: [SetInfo] = []
    private var isLoadingSets = false

    init(target: SearchResultsRoute.Target, api: any APIProtocol, defaults: UserDefaults = .standard) {
        self.target = target
        self.api = api
        sortStore = SearchResultsSortStore(defaults: defaults)
        filters = sortStore.load(for: target)
        paginator = .cardGrid(owner: self) { model, page, pageSize in
            try await model.fetchPage(page, pageSize: pageSize)
        }
    }

    /// First load only. The view's `.task` re-runs each time the screen re-appears (e.g. after
    /// popping a card detail); reloading then would throw away the loaded pages and the scroll
    /// position.
    func loadInitiallyIfNeeded() async {
        guard paginator.items.isEmpty, paginator.state == .idle else { return }
        await paginator.reload()
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

    private func fetchPage(_ page: Int32, pageSize: Int32) async throws -> PaginatorPage<CollectionCard> {
        guard let query = target.query(filters: filters, page: page, pageSize: pageSize) else {
            // The view never instantiates this model for a decklist target.
            return PaginatorPage(items: [], total: 0)
        }

        let response = try await api.search_cards(query: query).ok.body.json
        return PaginatorPage(items: response.items, total: Int(response.total))
    }
}
