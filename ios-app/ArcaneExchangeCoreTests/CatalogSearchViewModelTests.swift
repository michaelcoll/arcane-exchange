import APIClient
import Testing

@testable import ArcaneExchangeCore

private func catalogCard(_ number: String, name: String = "Lightning Bolt", owned: Int64 = 0) -> SearchCard {
    SearchCard(
        collector_number: number,
        french_name: "Foudre",
        image_url: "https://cards.scryfall.io/normal/front/\(number).jpg",
        name: name,
        offered_languages: [],
        owned_quantity: owned,
        owner_count: 0,
        rarity_code: "C",
        scryfall_id: "id-\(number)",
        set_code: "LEA",
        set_name: "Limited Edition Alpha"
    )
}

private func page(_ items: [SearchCard], total: Int64? = nil) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(
        items: items,
        page: 0,
        page_size: 20,
        total: total ?? Int64(items.count)
    ))))
}

@MainActor
struct CatalogSearchViewModelTests {
    @Test func searchesTheTrimmedQueryAndListsTheImpressions() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: Operations.search_cards.Input.Query?
        api.onSearchCards = { input in
            asked = input.query
            return page([catalogCard("161"), catalogCard("162")])
        }
        let model = CatalogSearchViewModel(api: api)

        model.query = "  foudre "
        await model.paginator.reload()

        #expect(model.paginator.items.map(\.collector_number) == ["161", "162"])
        #expect(model.paginator.total == 2)
        #expect(asked?.q == "foudre")
        #expect(asked?.page == 0)
        #expect(asked?.sets == nil)
    }

    @Test func asksForTheWholeCatalogueWithNoOfferNorPossessionFilter() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: Operations.search_cards.Input.Query?
        api.onSearchCards = { input in
            asked = input.query
            return page([catalogCard("161")])
        }
        let model = CatalogSearchViewModel(api: api)

        model.query = "foudre"
        await model.paginator.reload()

        #expect(asked?.offered == nil)
        #expect(asked?.offered_by == nil)
        #expect(asked?.owned == nil)
    }

    @Test func listsTheQuantityAlreadyOwnedOfEachImpression() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([catalogCard("161", owned: 3), catalogCard("162")]) }
        let model = CatalogSearchViewModel(api: api)

        model.query = "foudre"
        await model.paginator.reload()

        #expect(model.paginator.items.map(\.owned_quantity) == [3, 0])
    }

    @Test func restrictsTheSearchToTheSetTyped() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: Operations.search_cards.Input.Query?
        api.onSearchCards = { input in
            asked = input.query
            return page([catalogCard("161")])
        }
        let model = CatalogSearchViewModel(api: api)

        model.query = "foudre"
        model.setCode = " FDN "
        await model.paginator.reload()

        #expect(asked?.q == "foudre")
        #expect(asked?.sets == "FDN")
    }

    @Test func doesNotSendASetCodeTooShortToBeOne() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: Operations.search_cards.Input.Query?
        api.onSearchCards = { input in
            asked = input.query
            return page([catalogCard("161")])
        }
        let model = CatalogSearchViewModel(api: api)

        model.query = "foudre"
        // A code under three characters is still being typed.
        model.setCode = "FD"
        await model.paginator.reload()

        #expect(asked?.sets == nil)
    }

    @Test func aSetAloneIsNotASearch() async {
        let api = FakeAPI()
        let model = CatalogSearchViewModel(api: api)

        model.setCode = "FDN"
        await model.search()

        #expect(!model.hasQuery)
        #expect(api.calls.isEmpty)
    }

    @Test func aBlankQueryAsksNothing() async {
        let api = FakeAPI()
        let model = CatalogSearchViewModel(api: api)

        model.query = "   "
        await model.search()

        #expect(!model.hasQuery)
        #expect(model.paginator.items.isEmpty)
        #expect(api.calls.isEmpty)
    }

    @Test func loadsTheNextPageWhenTheListNearsItsEnd() async {
        let api = FakeAPI()
        nonisolated(unsafe) var pages: [Int32] = []
        api.onSearchCards = { input in
            pages.append(input.query.page ?? -1)
            return page([catalogCard("\(pages.count)")], total: 2)
        }
        let model = CatalogSearchViewModel(api: api)
        model.query = "bolt"

        await model.paginator.reload()
        await model.paginator.loadMore()

        #expect(pages == [0, 1])
        #expect(model.paginator.items.count == 2)
        #expect(!model.paginator.hasMore)
    }

    @Test func aFailedSearchShowsItsTranslatedMessage() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "internal", status: 500) }
        let model = CatalogSearchViewModel(api: api)
        model.query = "bolt"

        await model.paginator.reload()

        #expect(model.paginator.phase == .failed("Erreur technique, réessaie plus tard."))
    }
}
