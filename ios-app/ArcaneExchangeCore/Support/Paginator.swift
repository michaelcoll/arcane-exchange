import Foundation

/// One page of a paginated list, as the `fetch` of a `Paginator` returns it.
package struct PaginatorPage<Item> {
    package let items: [Item]
    package let total: Int

    package init(items: [Item], total: Int) {
        self.items = items
        self.total = total
    }
}

/// The page-by-page loading shared by the lists that fill as the player scrolls.
///
/// It only knows a function that fetches a page, and owns the next page, the total, the state
/// and the guard against late responses: the latest `reload()` always wins, whatever order the
/// responses come back in. It knows neither the API client nor the image pipeline: a list
/// supplies its `fetch`, and optionally two callbacks.
///
/// A fetch that throws `CancellationError` (or whose request was cancelled) leaves the list as it
/// was and the state `.idle`: it is not a failure to show.
///
/// A screen reads this state through `model.paginator`, and calls its actions on the model when
/// the model adds behaviour around them (`TradesViewModel.reload`), otherwise on the paginator.
@MainActor
@Observable
package final class Paginator<Item: Equatable> {
    package enum State: Equatable {
        case idle
        case loading
        case loadingMore
        /// French message of the failed load, ready to show.
        case failed(String)
    }

    /// What a list screen shows, in the order it checks: the spinner of the first load, the
    /// error of a load that left nothing to show, the empty screen, or the items.
    package enum Phase: Equatable {
        case firstLoad
        /// French message of the failed load, ready to show.
        case failed(String)
        case empty
        case content
    }

    /// Number of items asked for per page.
    package static var pageSize: Int32 {
        20
    }

    /// How close to the end of the displayed list an item must be to ask for the next page.
    private static var triggerDistance: Int {
        4
    }

    package private(set) var items: [Item] = []
    package private(set) var total = 0
    package private(set) var state = State.idle

    package var hasMore: Bool {
        items.count < total
    }

    package var phase: Phase {
        guard items.isEmpty else { return .content }

        switch state {
        case .loading: return .firstLoad
        case let .failed(message): return .failed(message)
        case .idle, .loadingMore: return .empty
        }
    }

    private let fetch: @MainActor (_ page: Int32, _ pageSize: Int32) async throws -> PaginatorPage<Item>
    private let onPageApplied: (@MainActor ([Item]) -> Void)?
    private let onReload: (@MainActor () -> Void)?

    private var nextPage: Int32 = 0

    /// Bumped by every `reload()`. A response is applied only while its generation is still the
    /// current one.
    private var generation = 0

    /// - Parameters:
    ///   - fetch: Fetches one page: its index (from 0) and the number of items per page.
    ///   - onPageApplied: Called with the items of each page that gets applied, never with those
    ///     of a late response.
    ///   - onReload: Called at the start of every `reload()`.
    package init(
        fetch: @escaping @MainActor (_ page: Int32, _ pageSize: Int32) async throws -> PaginatorPage<Item>,
        onPageApplied: (@MainActor ([Item]) -> Void)? = nil,
        onReload: (@MainActor () -> Void)? = nil
    ) {
        self.fetch = fetch
        self.onPageApplied = onPageApplied
        self.onReload = onReload
    }

    /// Restarts from page 0. On failure the list is emptied and the state becomes `.failed`.
    package func reload() async {
        generation += 1
        let current = generation
        let previousNextPage = nextPage
        state = .loading
        nextPage = 0
        onReload?()
        do {
            let page = try await fetch(0, Self.pageSize)
            guard generation == current else { return }
            items = page.items
            total = page.total
            nextPage = 1
            state = .idle
            onPageApplied?(page.items)
        } catch is CancellationError {
            guard generation == current else { return }
            nextPage = previousNextPage
            state = .idle
        } catch {
            guard generation == current else { return }
            items = []
            total = 0
            state = .failed(APIError.read(error).message)
        }
    }

    /// Asks for the next page when `item` just appeared among the last four of `displayed`
    /// (`items` by default; a list showing only part of them passes that part).
    package func loadMore(after item: Item, among displayed: [Item]? = nil) async {
        guard (displayed ?? items).suffix(Self.triggerDistance).contains(item) else { return }

        await loadMore()
    }

    /// Asks for the next page if there is one and nothing is loading or failed.
    package func loadMore() async {
        guard state == .idle, hasMore else { return }

        let current = generation
        let page = nextPage
        state = .loadingMore
        do {
            let response = try await fetch(page, Self.pageSize)
            guard generation == current else { return }
            items.append(contentsOf: response.items)
            total = response.total
            nextPage = page + 1
            state = .idle
            onPageApplied?(response.items)
        } catch is CancellationError {
            guard generation == current else { return }
            state = .idle
        } catch {
            guard generation == current else { return }
            state = .failed(APIError.read(error).message)
        }
    }
}
