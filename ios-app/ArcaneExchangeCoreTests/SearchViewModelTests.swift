import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

@MainActor
struct SearchViewModelTests {
    private func makeModel(_ api: FakeAPI) -> SearchViewModel {
        SearchViewModel(api: api, defaults: isolatedDefaults())
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
