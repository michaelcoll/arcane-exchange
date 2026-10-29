import APIClient
import Foundation
import Testing

@testable import ArcaneExchange

private func page(_ items: [CollectionCard], total: Int64) -> Operations.get_collection.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total))))
}

private func stats(sets: [SetInfo]) -> Operations.get_collection_stats.Output {
    .ok(.init(body: .json(.init(
        binders: [],
        sets: sets,
        total_cards: 0,
        unique_cards: 0
    ))))
}

@MainActor
struct CollectionViewModelTests {
    private func makeModel(_ api: FakeAPI) -> CollectionViewModel {
        CollectionViewModel(api: api, defaults: isolatedDefaults())
    }

    @Test func loadsTheFirstPage() async {
        let api = FakeAPI()
        api.onGetCollection = { _ in page([card("a"), card("b")], total: 2) }
        let model = makeModel(api)

        await model.loadInitiallyIfNeeded()

        #expect(model.paginator.items.map(\.scryfall_id) == ["a", "b"])
        #expect(model.paginator.total == 2)
        #expect(model.paginator.state == .idle)
        #expect(api.calls == ["get_collection"])
    }

    @Test func theLatestFilterWinsWhateverOrderTheResponsesComeBackIn() async {
        let api = FakeAPI()
        let gate = ResponseGate()
        api.onGetCollection = { _ in await gate.next() }
        let model = makeModel(api)
        model.filters = CollectionFilters(rarities: [.C])
        let first = Task { await model.paginator.reload() }
        await gate.waitForRequests(1)
        model.filters = CollectionFilters(rarities: [.M])
        let second = Task { await model.paginator.reload() }
        await gate.waitForRequests(2)

        gate.respond(1, with: page([card("mythic")], total: 1))
        await second.value
        gate.respond(0, with: page([card("common")], total: 1))
        await first.value

        #expect(model.paginator.items.map(\.scryfall_id) == ["mythic"])
    }

    @Test func aRefreshDuringTheNextPageDropsTheOldPage() async {
        let api = FakeAPI()
        let gate = ResponseGate()
        api.onGetCollection = { _ in await gate.next() }
        let model = makeModel(api)
        let initial = Task { await model.paginator.reload() }
        await gate.waitForRequests(1)
        let first = (1 ... 20).map { card("\($0)") }
        gate.respond(0, with: page(first, total: 40))
        await initial.value
        let more = Task { await model.paginator.loadMore(after: first[19]) }
        await gate.waitForRequests(2)
        let refresh = Task { await model.paginator.reload() }
        await gate.waitForRequests(3)

        gate.respond(2, with: page([card("fresh")], total: 1))
        await refresh.value
        gate.respond(1, with: page([card("stale")], total: 40))
        await more.value

        #expect(model.paginator.items.map(\.scryfall_id) == ["fresh"])
    }

    @Test func sendsTheSameRaritiesAndSetsAsTheSearch() async {
        let api = FakeAPI()
        nonisolated(unsafe) var collectionQuery: Operations.get_collection.Input.Query?
        api.onGetCollection = { input in
            collectionQuery = input.query
            return page([], total: 0)
        }
        let filters = CollectionFilters(rarities: [.M, .C, .R], sets: ["MH3", "FDN"], sortBy: .added_at, sortDir: .asc)
        let model = makeModel(api)
        model.filters = filters

        await model.paginator.reload()
        let searchQuery = SearchResultsRoute.Target.card(query: "Sol Ring")
            .query(filters: filters, page: 0, pageSize: 20)

        #expect(collectionQuery?.rarity == [.C, .R, .M])
        #expect(collectionQuery?.sets == "FDN,MH3")
        #expect(collectionQuery?.sort_by == .added_at)
        #expect(collectionQuery?.sort_dir == .asc)
        #expect(searchQuery?.rarity == collectionQuery?.rarity)
        #expect(searchQuery?.sets == collectionQuery?.sets)
    }

    @Test func loadsTheSetsSortedByName() async {
        let api = FakeAPI()
        api.onGetCollectionStats = { _ in
            stats(sets: [.init(code: "MH3", name: "Modern Horizons 3"), .init(code: "FDN", name: "Foundations")])
        }
        let model = makeModel(api)

        await model.loadSetsIfNeeded()

        #expect(model.sets.map(\.code) == ["FDN", "MH3"])
    }

    @Test func aRestoredSetFilterWaitsForTheSetListAndDropsASetNoLongerOwned() async {
        let defaults = isolatedDefaults()
        CollectionFiltersStore(defaults: defaults).save(CollectionFilters(sets: ["FDN", "OLD"]))
        let api = FakeAPI()
        var model: CollectionViewModel?
        var waitingDuringSets: Bool?
        var callsDuringSets: [String] = []
        nonisolated(unsafe) var collectionQuery: Operations.get_collection.Input.Query?
        api.onGetCollectionStats = { _ in
            waitingDuringSets = model?.isWaitingForSets
            callsDuringSets = api.calls
            return stats(sets: [.init(code: "FDN", name: "Foundations")])
        }
        api.onGetCollection = { input in
            collectionQuery = input.query
            return page([card("a")], total: 1)
        }
        model = CollectionViewModel(api: api, defaults: defaults)

        await model?.loadInitiallyIfNeeded()

        #expect(waitingDuringSets == true)
        #expect(callsDuringSets == ["get_collection_stats"])
        #expect(model?.isWaitingForSets == false)
        #expect(collectionQuery?.sets == "FDN")
        #expect(model?.filters.sets == ["FDN"])
        #expect(model?.paginator.items.map(\.scryfall_id) == ["a"])
    }

    @Test func reloadsTheSetsThenTheGridAfterAnImport() async {
        let api = FakeAPI()
        api.onGetCollectionStats = { _ in stats(sets: [.init(code: "FDN", name: "Foundations")]) }
        api.onGetCollection = { _ in page([card("a")], total: 1) }
        let model = makeModel(api)
        await model.loadSetsIfNeeded()
        api.onGetCollectionStats = { _ in
            stats(sets: [.init(code: "FDN", name: "Foundations"), .init(code: "MH3", name: "Modern Horizons 3")])
        }

        await model.reloadAfterImport()

        #expect(model.sets.map(\.code) == ["FDN", "MH3"])
        #expect(model.paginator.items.map(\.scryfall_id) == ["a"])
        #expect(api.calls == ["get_collection_stats", "get_collection_stats", "get_collection"])
    }

    @Test func aFailedSetListDoesNotBlankTheGrid() async {
        let api = FakeAPI()
        api.onGetCollectionStats = { _ in throw APIError(code: "internal", status: 500) }
        let model = makeModel(api)

        await model.loadSetsIfNeeded()

        #expect(model.sets.isEmpty)
        #expect(model.paginator.state == .idle)
    }
}

/// Holds each `get_collection` response until the test releases it, to fix the order they
/// come back in.
@MainActor
private final class ResponseGate {
    private var waiting: [CheckedContinuation<Operations.get_collection.Output, Never>] = []

    func next() async -> Operations.get_collection.Output {
        await await withCheckedContinuation { waiting.append($0) }
    }

    func respond(_ index: Int, with output: Operations.get_collection.Output) {
        waiting[index].resume(returning: output)
    }

    func waitForRequests(_ count: Int) async {
        while waiting.count < count {
            await Task.yield()
        }
    }
}
