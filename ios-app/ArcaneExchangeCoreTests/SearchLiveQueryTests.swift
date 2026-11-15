import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

private typealias Query = Operations.search_cards.Input.Query

private func page(_ items: [SearchCard], total: Int64) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total))))
}

/// The live grid of the Rechercher tab: `search(_:)` follows the field as it is typed.
@MainActor
struct SearchLiveQueryTests {
    private func makeModel(_ api: FakeAPI) -> SearchResultsViewModel {
        SearchResultsViewModel(target: .card(query: ""), api: api, defaults: isolatedDefaults())
    }

    @Test func loadsTheCardsAndTheSetsOfTheTypedText() async {
        let api = FakeAPI()
        nonisolated(unsafe) var queries: [Query] = []
        nonisolated(unsafe) var setsTexts: [String?] = []
        api.onSearchCards = { input in
            queries.append(input.query)
            return page([searchCard("a")], total: 1)
        }
        api.onSearchCardSets = { input in
            setsTexts.append(input.query.q)
            return .ok(.init(body: .json([SetInfo(code: "SOA", name: "Secrets")])))
        }
        let model = makeModel(api)

        await model.search("  Sol Ring ")

        #expect(model.target == .card(query: "Sol Ring"))
        #expect(queries.map(\.q) == ["Sol Ring"])
        #expect(model.paginator.items.map(\.scryfall_id) == ["a"])
        #expect(setsTexts == ["Sol Ring"])
        #expect(!model.isAwaitingQuery)
    }

    @Test func keepsTheFiltersAcrossTexts() async {
        let api = FakeAPI()
        nonisolated(unsafe) var queries: [Query] = []
        api.onSearchCards = { input in
            queries.append(input.query)
            return page([], total: 0)
        }
        let model = makeModel(api)
        model.filters.rarities = [.M]

        await model.search("Sol")
        await model.search("Sol Ring")

        #expect(queries.map(\.q) == ["Sol", "Sol Ring"])
        #expect(queries.map(\.rarity) == [[.M], [.M]])
    }

    @Test func fetchesNothingForABlankTextNorTheSameTextAgain() async {
        let api = FakeAPI()
        nonisolated(unsafe) var calls = 0
        api.onSearchCards = { _ in
            calls += 1
            return page([], total: 0)
        }
        let model = makeModel(api)

        await model.search("   ")
        await model.search("Sol")
        await model.search("Sol ")

        #expect(calls == 1)
    }

    /// The tab shows the recent searches while the field is blank: a filter picked then must
    /// still apply when the same text comes back.
    @Test func reloadsTheSameTextAfterAFilterChange() async {
        let api = FakeAPI()
        nonisolated(unsafe) var queries: [Query] = []
        api.onSearchCards = { input in
            queries.append(input.query)
            return page([], total: 0)
        }
        let model = makeModel(api)

        await model.search("Sol")
        await model.search("")
        model.filters.rarities = [.M]
        await model.search("Sol")

        #expect(queries.map(\.rarity) == [nil, [.M]])
    }
}
