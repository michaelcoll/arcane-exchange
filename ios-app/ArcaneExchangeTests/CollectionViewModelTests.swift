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

        #expect(model.cards.map(\.scryfall_id) == ["a", "b"])
        #expect(model.total == 2)
        #expect(model.loadError == nil)
        #expect(!model.isLoading)
        #expect(api.calls == ["get_collection"])
    }

    @Test func loadsTheNextPageWhenTheLastCardsAppear() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { card("\($0)") }
        api.onGetCollection = { input in
            input.query.page == 0 ? page(first, total: 21) : page([card("21")], total: 21)
        }
        let model = makeModel(api)
        await model.reload()
        #expect(model.hasMore)

        await model.loadMoreIfNeeded(displaying: first[19])

        #expect(model.cards.count == 21)
        #expect(!model.hasMore)
        #expect(api.calls == ["get_collection", "get_collection"])
    }

    @Test func doesNotLoadMoreForACardFarFromTheEnd() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { card("\($0)") }
        api.onGetCollection = { _ in page(first, total: 40) }
        let model = makeModel(api)
        await model.reload()

        await model.loadMoreIfNeeded(displaying: first[0])

        #expect(api.calls == ["get_collection"])
    }

    @Test func asksForTheRaritiesSetsAndSortOfTheFilters() async {
        let api = FakeAPI()
        nonisolated(unsafe) var query: Operations.get_collection.Input.Query?
        api.onGetCollection = { input in
            query = input.query
            return page([], total: 0)
        }
        let model = makeModel(api)
        model.filters = CollectionFilters(rarities: [.R, .C], sets: ["MH3", "FDN"], sortBy: .added_at, sortDir: .asc)

        await model.reload()

        #expect(query?.rarity == [.C, .R])
        #expect(query?.sets == "FDN,MH3")
        #expect(query?.sort_by == .added_at)
        #expect(query?.sort_dir == .asc)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onGetCollection = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = makeModel(api)

        await model.reload()

        #expect(model.loadError == APIErrorMessages.network)
        #expect(model.cards.isEmpty)
        #expect(!model.isLoading)
    }

    @Test func showsTheGenericMessageForAnUnknownCode() async {
        let api = FakeAPI()
        api.onGetCollection = { _ in throw APIError(code: "brand_new_code", status: 418) }
        let model = makeModel(api)

        await model.reload()

        #expect(model.loadError == APIErrorMessages.generic)
    }

    @Test func aFailedNextPageKeepsTheCardsAlreadyLoaded() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { card("\($0)") }
        api.onGetCollection = { input in
            if input.query.page == 0 { return page(first, total: 40) }
            throw APIError(code: "internal", status: 500)
        }
        let model = makeModel(api)
        await model.reload()

        await model.loadMoreIfNeeded(displaying: first[19])

        #expect(model.cards.count == 20)
        #expect(model.loadError == APIErrorMessages.message(for: "internal"))
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

    @Test func aFailedSetListDoesNotBlankTheGrid() async {
        let api = FakeAPI()
        api.onGetCollectionStats = { _ in throw APIError(code: "internal", status: 500) }
        let model = makeModel(api)

        await model.loadSetsIfNeeded()

        #expect(model.sets.isEmpty)
        #expect(model.loadError == nil)
    }
}
