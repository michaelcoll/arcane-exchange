import APIClient
import Foundation
import Testing

@testable import ArcaneExchange

private func page(_ items: [CollectionCard], total: Int64) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total))))
}

@MainActor
struct SearchViewModelTests {
    private func makeModel(_ api: FakeAPI) -> SearchViewModel {
        SearchViewModel(api: api, defaults: isolatedDefaults())
    }

    @Test func previewsTheCardsMatchingTheQuery() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([card("a")], total: 1) }
        let model = makeModel(api)

        await model.previewCards(matching: "sol")

        #expect(model.cardPreview.map(\.scryfall_id) == ["a"])
        #expect(!model.cardPreviewFailed)
    }

    @Test func flagsAFailedCardPreview() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = makeModel(api)

        await model.previewCards(matching: "sol")

        #expect(model.cardPreview.isEmpty)
        #expect(model.cardPreviewFailed)
    }

    @Test func listsThePlayersMatchingTheQuery() async {
        let api = FakeAPI()
        api.onAutocompleteUser = { _ in
            .ok(.init(body: .json([UserSuggestion(card_count: 3, note: 5, username: "jace")])))
        }
        let model = makeModel(api)

        await model.lookUpPlayers(matching: "ja")

        #expect(model.playerSuggestions.map(\.username) == ["jace"])
        #expect(!model.playerLookupFailed)
    }

    @Test func flagsAFailedPlayerLookup() async {
        let api = FakeAPI()
        api.onAutocompleteUser = { _ in throw APIError(code: "new_backend_code", status: 409) }
        let model = makeModel(api)

        await model.lookUpPlayers(matching: "ja")

        #expect(model.playerSuggestions.isEmpty)
        #expect(model.playerLookupFailed)
    }
}
