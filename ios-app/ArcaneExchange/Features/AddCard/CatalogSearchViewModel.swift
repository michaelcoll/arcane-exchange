import APIClient
import ArcaneCore
import Foundation

typealias CatalogCard = Components.Schemas.CatalogCardResponse

/// Backs the search of the catalog, the first step of adding a card by hand: the impressions
/// (set and collector number) whose English or French name matches the query, page by page.
@MainActor
@Observable
final class CatalogSearchViewModel {
    /// Delay the search waits out before hitting the network.
    private static let debounce: Duration = .milliseconds(300)

    var query = ""
    /// The code of the set to restrict the search to (`FDN`), optional.
    var setCode = ""

    /// The impressions found. Implicitly unwrapped on purpose: its `fetch` reads `query`, so it
    /// is built in `init` once `self` is usable.
    private(set) var paginator: Paginator<CatalogCard>!

    private let api: any APIProtocol

    init(api: any APIProtocol) {
        self.api = api
        paginator = Paginator { [weak self] page, pageSize in
            guard let self else { throw CancellationError() }
            return try await fetchPage(page, pageSize: pageSize)
        }
    }

    /// Whether there is something to search for: the screen shows a prompt until then.
    var hasQuery: Bool {
        !trimmedQuery.isEmpty
    }

    /// What the search depends on: the screen restarts it when this changes.
    var searchKey: String {
        "\(query)\u{0}\(setCodeFilter ?? "")"
    }

    /// Debounced: called from `.task(id: searchKey)`, so a new keystroke cancels the pending sleep.
    func search() async {
        guard hasQuery else {
            await paginator.reload()
            return
        }
        try? await Task.sleep(for: Self.debounce)
        guard !Task.isCancelled else { return }

        await paginator.reload()
    }

    private var trimmedQuery: String {
        query.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// The set code to send: only once it has the length of one (3 to 6 characters), since the
    /// server refuses a shorter code and the player is still typing it.
    private var setCodeFilter: String? {
        let code = setCode.trimmingCharacters(in: .whitespacesAndNewlines)
        return (3 ... 6).contains(code.count) ? code : nil
    }

    private func fetchPage(_ page: Int32, pageSize: Int32) async throws -> PaginatorPage<CatalogCard> {
        guard hasQuery else { return PaginatorPage(items: [], total: 0) }

        let output = try await api.search_catalog(query: .init(
            q: trimmedQuery,
            set_code: setCodeFilter,
            page: page,
            page_size: pageSize
        ))
        let body = try output.ok.body.json
        return PaginatorPage(items: body.items, total: Int(body.total))
    }
}
