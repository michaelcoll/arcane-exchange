import Foundation
import Testing

@testable import ArcaneExchange

/// A `fetch` whose responses the test releases by hand, to fix the order they arrive in.
@MainActor
private final class SuspendedFetch {
    private(set) var requestedPages: [Int32] = []
    private var waiting: [CheckedContinuation<PaginatorPage<Int>, any Error>] = []

    func fetch(page: Int32, pageSize _: Int32) async throws -> PaginatorPage<Int> {
        requestedPages.append(page)
        return try await withCheckedThrowingContinuation { waiting.append($0) }
    }

    /// Releases the `index`-th request (in the order they were made).
    func respond(_ index: Int, with result: Result<Response, any Error>) {
        switch result {
        case let .success(response):
            waiting[index].resume(returning: PaginatorPage(items: response.items, total: response.total))
        case let .failure(error):
            waiting[index].resume(throwing: error)
        }
    }

    struct Response {
        let items: [Int]
        let total: Int
    }

    /// Lets the tasks started by the test reach their `fetch`.
    func waitForRequests(_ count: Int) async {
        while requestedPages.count < count {
            await Task.yield()
        }
    }
}

private func numbers(_ range: ClosedRange<Int>) -> [Int] {
    Array(range)
}

/// A paginator over 21 numbers: 1...20 on page 0, 21 on page 1.
@MainActor
private func makeBasicPaginator(
    onPageApplied: (@MainActor ([Int]) -> Void)? = nil,
    onReload: (@MainActor () -> Void)? = nil
) -> (Paginator<Int>, requests: () -> [Int32]) {
    var requested: [Int32] = []
    let paginator = Paginator<Int>(
        fetch: { page, _ in
            requested.append(page)
            return page == 0
                ? PaginatorPage(items: numbers(1 ... 20), total: 21)
                : PaginatorPage(items: [21], total: 21)
        },
        onPageApplied: onPageApplied,
        onReload: onReload
    )
    return (paginator, { requested })
}

@MainActor
struct PaginatorTests {
    // MARK: Loading

    @Test func loadsTheFirstPage() async {
        let (paginator, requests) = makeBasicPaginator()

        await paginator.reload()

        #expect(paginator.items == numbers(1 ... 20))
        #expect(paginator.total == 21)
        #expect(paginator.state == .idle)
        #expect(requests() == [0])
    }

    @Test func loadsTheNextPageWhenOneOfTheLastFourItemsAppears() async {
        let (paginator, requests) = makeBasicPaginator()
        await paginator.reload()

        await paginator.loadMore(after: 17)

        #expect(paginator.items == numbers(1 ... 21))
        #expect(!paginator.hasMore)
        #expect(requests() == [0, 1])
    }

    @Test func ignoresAnItemFarFromTheEnd() async {
        let (paginator, requests) = makeBasicPaginator()
        await paginator.reload()

        await paginator.loadMore(after: 16)

        #expect(paginator.items.count == 20)
        #expect(requests() == [0])
    }

    @Test func asksForNothingOnceTheTotalIsReached() async {
        let (paginator, requests) = makeBasicPaginator()
        await paginator.reload()
        await paginator.loadMore(after: 20)

        await paginator.loadMore(after: 21)

        #expect(requests() == [0, 1])
    }

    @Test func triggersOnTheDisplayedSubList() async {
        let (paginator, requests) = makeBasicPaginator()
        await paginator.reload()
        let displayed = [2, 4, 6, 8, 10, 12]

        await paginator.loadMore(after: 2, among: displayed)
        #expect(requests() == [0])

        await paginator.loadMore(after: 12, among: displayed)
        #expect(requests() == [0, 1])
    }

    @Test func loadsTheNextPageWithoutAnItemWhenOneRemains() async {
        let (paginator, requests) = makeBasicPaginator()
        await paginator.reload()

        await paginator.loadMore()
        await paginator.loadMore()

        #expect(paginator.items.count == 21)
        #expect(requests() == [0, 1])
    }

    @Test func sendsOneRequestForSimultaneousCallsToLoadMore() async {
        let source = SuspendedFetch()
        let paginator = Paginator<Int>(fetch: source.fetch)
        let reload = Task { await paginator.reload() }
        await source.waitForRequests(1)
        source.respond(0, with: .success(.init(items: numbers(1 ... 20), total: 40)))
        await reload.value

        let first = Task { await paginator.loadMore(after: 20) }
        await source.waitForRequests(2)
        #expect(paginator.state == .loadingMore)
        let second = Task { await paginator.loadMore(after: 19) }
        await Task.yield()
        source.respond(1, with: .success(.init(items: numbers(21 ... 40), total: 40)))
        await first.value
        await second.value

        #expect(source.requestedPages == [0, 1])
        #expect(paginator.items == numbers(1 ... 40))
    }

    // MARK: Races

    @Test func theLatestReloadWinsWhenTheResponsesAreInverted() async {
        let source = SuspendedFetch()
        let paginator = Paginator<Int>(fetch: source.fetch)
        let first = Task { await paginator.reload() }
        await source.waitForRequests(1)
        let second = Task { await paginator.reload() }
        await source.waitForRequests(2)

        source.respond(1, with: .success(.init(items: [2], total: 1)))
        await second.value
        source.respond(0, with: .success(.init(items: [1], total: 1)))
        await first.value

        #expect(paginator.items == [2])
        #expect(paginator.state == .idle)
    }

    @Test func aReloadDuringLoadMoreDropsTheLatePage() async {
        let source = SuspendedFetch()
        let paginator = Paginator<Int>(fetch: source.fetch)
        let initial = Task { await paginator.reload() }
        await source.waitForRequests(1)
        source.respond(0, with: .success(.init(items: numbers(1 ... 20), total: 40)))
        await initial.value
        let more = Task { await paginator.loadMore(after: 20) }
        await source.waitForRequests(2)
        let reload = Task { await paginator.reload() }
        await source.waitForRequests(3)

        source.respond(2, with: .success(.init(items: [101], total: 1)))
        await reload.value
        source.respond(1, with: .success(.init(items: numbers(21 ... 40), total: 40)))
        await more.value

        #expect(paginator.items == [101])
        #expect(paginator.state == .idle)
    }

    @Test func theFailureOfAStaleReloadDoesNotOverwriteTheCurrentResult() async {
        let source = SuspendedFetch()
        let paginator = Paginator<Int>(fetch: source.fetch)
        let first = Task { await paginator.reload() }
        await source.waitForRequests(1)
        let second = Task { await paginator.reload() }
        await source.waitForRequests(2)

        source.respond(1, with: .success(.init(items: [2], total: 1)))
        await second.value
        source.respond(0, with: .failure(APIError(code: "internal", status: 500)))
        await first.value

        #expect(paginator.items == [2])
        #expect(paginator.state == .idle)
    }

    // MARK: Errors

    @Test func aFailedFirstLoadEmptiesTheListAndAReloadClearsTheError() async {
        var shouldFail = true
        let paginator = Paginator<Int>(fetch: { _, _ in
            if shouldFail { throw APIError(code: "internal", status: 500) }
            return PaginatorPage(items: [1], total: 1)
        })

        await paginator.reload()

        #expect(paginator.items.isEmpty)
        #expect(paginator.state == .failed("Erreur technique, réessaie plus tard."))

        shouldFail = false
        await paginator.reload()

        #expect(paginator.items == [1])
        #expect(paginator.state == .idle)
    }

    @Test func aFailedLoadMoreKeepsTheItemsAndBlocksFurtherPagesUntilAReload() async {
        var requested: [Int32] = []
        let paginator = Paginator<Int>(fetch: { page, _ in
            requested.append(page)
            if page == 1 { throw APIError(code: nil, status: nil, isNetworkFailure: true) }
            return PaginatorPage(items: numbers(1 ... 20), total: 40)
        })
        await paginator.reload()

        await paginator.loadMore(after: 20)
        await paginator.loadMore(after: 20)

        #expect(paginator.items == numbers(1 ... 20))
        #expect(paginator.state == .failed(APIErrorMessages.network))
        #expect(requested == [0, 1])
    }

    @Test func aCancelledLoadIsNotAnErrorAndKeepsTheList() async {
        var shouldCancel = false
        let paginator = Paginator<Int>(fetch: { page, _ in
            if shouldCancel { throw CancellationError() }
            return PaginatorPage(items: page == 0 ? numbers(1 ... 20) : [21], total: 21)
        })
        await paginator.reload()
        shouldCancel = true

        await paginator.loadMore(after: 20)
        #expect(paginator.state == .idle)

        await paginator.reload()
        #expect(paginator.state == .idle)
        #expect(paginator.items == numbers(1 ... 20))

        shouldCancel = false
        await paginator.loadMore(after: 20)
        #expect(paginator.items == numbers(1 ... 21))
    }

    // MARK: Phase

    @Test func isInTheFirstLoadPhaseWhileTheFirstPageLoads() async {
        let source = SuspendedFetch()
        let paginator = Paginator<Int>(fetch: source.fetch)
        let reload = Task { await paginator.reload() }
        await source.waitForRequests(1)

        #expect(paginator.phase == .firstLoad)

        source.respond(0, with: .success(.init(items: [1], total: 1)))
        await reload.value
    }

    @Test func isInTheFailedPhaseWhenALoadLeftNothingToShow() async {
        let paginator = Paginator<Int>(fetch: { _, _ in throw APIError(code: "internal", status: 500) })

        await paginator.reload()

        #expect(paginator.phase == .failed("Erreur technique, réessaie plus tard."))
    }

    @Test func isInTheEmptyPhaseWhenTheListIsEmpty() async {
        let paginator = Paginator<Int>(fetch: { _, _ in PaginatorPage(items: [], total: 0) })

        await paginator.reload()

        #expect(paginator.phase == .empty)
    }

    @Test func staysInTheContentPhaseWhenALoadMoreFails() async {
        let paginator = Paginator<Int>(fetch: { page, _ in
            if page == 1 { throw APIError(code: "internal", status: 500) }
            return PaginatorPage(items: numbers(1 ... 20), total: 40)
        })
        await paginator.reload()

        await paginator.loadMore(after: 20)

        #expect(paginator.state == .failed("Erreur technique, réessaie plus tard."))
        #expect(paginator.phase == .content)
    }

    // MARK: Callbacks

    @Test func callsBackForEveryAppliedPageAndEveryReload() async {
        var applied: [[Int]] = []
        var reloads = 0
        let (paginator, _) = makeBasicPaginator(
            onPageApplied: { applied.append($0) },
            onReload: { reloads += 1 }
        )

        await paginator.reload()
        await paginator.loadMore(after: 20)
        await paginator.reload()

        #expect(applied == [numbers(1 ... 20), [21], numbers(1 ... 20)])
        #expect(reloads == 2)
    }

    @Test func neverCallsBackForAStaleResponse() async {
        let source = SuspendedFetch()
        var applied: [[Int]] = []
        let paginator = Paginator<Int>(fetch: source.fetch, onPageApplied: { applied.append($0) })
        let first = Task { await paginator.reload() }
        await source.waitForRequests(1)
        let second = Task { await paginator.reload() }
        await source.waitForRequests(2)

        source.respond(1, with: .success(.init(items: [2], total: 1)))
        await second.value
        source.respond(0, with: .success(.init(items: [1], total: 1)))
        await first.value

        #expect(applied == [[2]])
    }
}
