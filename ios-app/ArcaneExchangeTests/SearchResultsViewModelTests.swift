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

        await model.load()

        #expect(model.cards.map(\.scryfall_id) == ["a", "b"])
        #expect(model.total == 2)
        #expect(model.loadError == nil)
        #expect(api.calls == ["search_cards"])
    }

    @Test func loadsTheNextPageWhenTheLastCardsAppear() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { card("\($0)") }
        api.onSearchCards = { input in
            input.query.page == 0 ? page(first, total: 21) : page([card("21")], total: 21)
        }
        let model = makeModel(api)
        await model.load()
        #expect(model.hasMore)

        await model.loadMoreIfNeeded(displaying: first[19])

        #expect(model.cards.count == 21)
        #expect(!model.hasMore)
        #expect(api.calls == ["search_cards", "search_cards"])
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "internal", status: 500) }
        let model = makeModel(api)

        await model.load()

        #expect(model.loadError == "Erreur technique, réessaie plus tard.")
        #expect(model.cards.isEmpty)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = makeModel(api)

        await model.load()

        #expect(model.loadError == APIErrorMessages.network)
    }

    @Test func showsTheGenericMessageForAnUnknownCode() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "new_backend_code", status: 409) }
        let model = makeModel(api)

        await model.load()

        #expect(model.loadError == "Une erreur est survenue.")
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
