import APIClient
import Foundation

/// One page-by-page `GET /search/card` feed, scoped either to a free-text query or to a
/// single player's tradable collection. Modelled on `CollectionViewModel`.
@MainActor
@Observable
package final class SearchResultsViewModel {
    /// Changes only through `search(_:)`, as the player types in the Rechercher tab.
    package private(set) var target: SearchResultsRoute.Target

    /// Between a keystroke and the reload it triggers, debounce included: the grid shows a
    /// spinner rather than "no result" while the first page of a new text is on its way.
    package private(set) var isAwaitingQuery = false

    /// The sort picked on this screen. It lives as long as the model, i.e. while the screen
    /// stays on the navigation stack, so a round trip to a card detail keeps it. The sort is
    /// also remembered on the device for the next search of the same kind.
    package var filters: CollectionFilters {
        didSet { sortStore.save(filters, for: target) }
    }

    /// The Possession and Offre axes picked on this screen. Reload the paginator whenever it
    /// changes, like `filters`. Not remembered: a search starts on the default axes.
    package var axes = SearchAxes()

    private let api: any APIProtocol
    private let sortStore: SearchResultsSortStore

    /// The loaded cards and their loading state. Reload it whenever `filters` changes. Implicitly
    /// unwrapped on purpose: its `fetch` reads `filters`, so it is built in `init` once `self` is
    /// usable, and `@Observable` rules out `lazy` and a `let` assigned last.
    package private(set) var paginator: Paginator<SearchCard>!

    /// What the set drawer lists: the sets of every card this search can return, whatever the
    /// filters. Empty until loaded, which disables the drawer's Sets row.
    package private(set) var sets: [SetInfo] = []
    private var isLoadingSets = false
    /// The axes `sets` was loaded for: the facet follows them.
    private var loadedSetsAxes: SearchAxes?

    package init(target: SearchResultsRoute.Target, api: any APIProtocol, defaults: UserDefaults = .standard) {
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
    package func loadInitiallyIfNeeded() async {
        guard paginator.items.isEmpty, paginator.state == .idle else { return }
        await paginator.reload()
    }

    /// The live search of the Rechercher tab: from `.task(id:)`, so a new keystroke cancels the
    /// pending debounce. Keeps the filters and axes, reloads the cards and the set facet for the
    /// new text. A blank text fetches nothing: the tab shows the recent searches instead.
    package func search(_ raw: String) async {
        let query = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Criteria(target: .card(query: query), filters: filters, axes: axes) != loaded else { return }
        guard !query.isEmpty else {
            isAwaitingQuery = false
            return
        }
        isAwaitingQuery = true
        try? await Task.sleep(for: Self.debounce)
        guard !Task.isCancelled else { return }

        target = .card(query: query)
        sets = []
        loadedSetsAxes = nil
        await paginator.reload()
        isAwaitingQuery = false
        await loadSetsIfNeeded()
    }

    /// How long `search(_:)` waits for the next keystroke before hitting the network.
    private static let debounce: Duration = .milliseconds(300)

    /// What the loaded pages were fetched for. Filters and axes can change while the tab shows
    /// the recent searches, so the same text typed again must still reload.
    private struct Criteria: Equatable {
        let target: SearchResultsRoute.Target
        let filters: CollectionFilters
        let axes: SearchAxes
    }

    private var loaded: Criteria?

    /// Loads the set list once per combination of axes, since the filters never change it. A
    /// failure is silent: the grid stays usable, and the next appearance tries again.
    package func loadSetsIfNeeded() async {
        let axes = axes
        guard sets.isEmpty || loadedSetsAxes != axes, !isLoadingSets else { return }
        let query = target.setsQuery(axes: axes)
        isLoadingSets = true
        defer { isLoadingSets = false }
        guard let sets = try? await api.search_card_sets(query: query).ok.body.json else { return }
        self.sets = sets
        loadedSetsAxes = axes
    }

    private func fetchPage(_ page: Int32, pageSize: Int32) async throws -> PaginatorPage<SearchCard> {
        let criteria = Criteria(target: target, filters: filters, axes: axes)
        let query = target.query(filters: filters, axes: axes, page: page, pageSize: pageSize)
        let response = try await api.search_cards(query: query).ok.body.json
        if page == 0 {
            loaded = criteria
        }
        return PaginatorPage(items: response.items, total: Int(response.total))
    }
}
