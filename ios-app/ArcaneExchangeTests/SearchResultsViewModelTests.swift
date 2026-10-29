import APIClient
import Foundation
import Testing

@testable import ArcaneExchange

private func page(_ items: [CollectionCard], total: Int64) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total))))
}

@MainActor
struct SearchResultsViewModelTests {
    private func makeModel(_ api: FakeAPI) -> SearchResultsViewModel {
        SearchResultsViewModel(target: .card(query: "Sol Ring"), api: api, defaults: isolatedDefaults())
    }

    @Test func loadsTheFirstPage() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([card("a"), card("b")], total: 2) }
        let model = makeModel(api)

        await model.loadInitiallyIfNeeded()

        #expect(model.paginator.items.map(\.scryfall_id) == ["a", "b"])
        #expect(model.paginator.total == 2)
        #expect(model.paginator.state == .idle)
        #expect(api.calls == ["search_cards"])
    }

    @Test func carriesTheSearchAndTheFiltersToTheNextPage() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { card("\($0)") }
        nonisolated(unsafe) var queries: [Operations.search_cards.Input.Query] = []
        api.onSearchCards = { input in
            queries.append(input.query)
            return page(first, total: 21)
        }
        let model = makeModel(api)
        model.filters = CollectionFilters(rarities: [.M], sets: ["MH3"])
        await model.loadInitiallyIfNeeded()

        await model.paginator.loadMore(after: first[19])

        #expect(queries.map(\.page) == [0, 1])
        #expect(queries[1].q == "Sol Ring")
        #expect(queries[1].rarity == queries[0].rarity)
        #expect(queries[1].sets == "MH3")
    }

    @Test func doesNotReloadOnceLoaded() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([card("a")], total: 1) }
        let model = makeModel(api)

        await model.loadInitiallyIfNeeded()
        await model.loadInitiallyIfNeeded()

        #expect(api.calls == ["search_cards"])
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "internal", status: 500) }
        let model = makeModel(api)

        await model.loadInitiallyIfNeeded()

        #expect(model.paginator.state == .failed("Erreur technique, réessaie plus tard."))
        #expect(model.paginator.items.isEmpty)
    }

    @Test func loadsTheSetsOnce() async {
        let api = FakeAPI()
        api.onSearchCardSets = { _ in .ok(.init(body: .json([SetInfo(code: "SOA", name: "Secrets")]))) }
        let model = makeModel(api)

        await model.loadSetsIfNeeded()
        await model.loadSetsIfNeeded()

        #expect(model.sets.map(\.code) == ["SOA"])
        #expect(api.calls == ["search_card_sets"])
    }

    @Test func keepsTheSetsEmptyWhenTheirLoadFails() async {
        let api = FakeAPI()
        api.onSearchCardSets = { _ in throw APIError(code: "internal", status: 500) }
        let model = makeModel(api)

        await model.loadSetsIfNeeded()

        #expect(model.sets.isEmpty)
    }
}
