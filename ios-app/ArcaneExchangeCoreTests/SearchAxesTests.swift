import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

private func page(_ items: [SearchCard], total: Int64) -> Operations.search_cards.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total))))
}

/// The Possession and Offre axes of the search results screens: what `GET /search/card` and its
/// set facet are asked for.
struct SearchAxesTests {
    typealias Query = Operations.search_cards.Input.Query
    typealias SetsQuery = Operations.search_card_sets.Input.Query

    private let text = SearchResultsRoute.Target.card(query: "Sol Ring")
    private let player = SearchResultsRoute.Target.player(username: "jace")

    @Test func startsOnTheWholeCatalogue() {
        #expect(SearchAxes() == SearchAxes(possession: .any, offer: .all))
    }

    @Test func theOwnedImpressionsSendOwnedTrue() {
        let axes = SearchAxes(possession: .owned)
        let query = text.query(filters: CollectionFilters(), axes: axes, page: 0, pageSize: 20)
        #expect(query.owned == true)
        #expect(query.offered == nil)
    }

    @Test func theImpressionsNotOwnedSendOwnedFalseAndStayOnTheCatalogue() {
        let axes = SearchAxes(possession: .notOwned)
        let query = text.query(filters: CollectionFilters(), axes: axes, page: 0, pageSize: 20)
        #expect(query.owned == false)
        #expect(query.offered == nil)
    }

    @Test func everyPossessionSendsNoOwnedParameter() {
        let query = text.query(filters: CollectionFilters(), axes: SearchAxes(), page: 0, pageSize: 20)
        #expect(query.owned == nil)
    }

    @Test func theWholeCatalogueSendsNoOfferedParameter() {
        let axes = SearchAxes(offer: .all)
        let query = text.query(filters: CollectionFilters(), axes: axes, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_dir: .desc, q: "Sol Ring"))
    }

    @Test func theAxesCombine() {
        let axes = SearchAxes(possession: .notOwned, offer: .offered)
        let query = text.query(filters: CollectionFilters(), axes: axes, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_dir: .desc, q: "Sol Ring", owned: false, offered: true))
    }

    @Test func aPlayersCollectionKeepsThePlayerAsItsOfferAxis() {
        let axes = SearchAxes(possession: .notOwned, offer: .all)
        let query = player.query(filters: CollectionFilters(), axes: axes, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_dir: .desc, owned: false, offered_by: "jace"))
    }

    @Test func theSetFacetFollowsTheTextAndBothAxes() {
        let notOwned = SearchAxes(possession: .notOwned, offer: .offered)
        #expect(text.setsQuery(axes: SearchAxes(offer: .all)) == SetsQuery(q: "Sol Ring"))
        #expect(text.setsQuery(axes: SearchAxes()) == SetsQuery(q: "Sol Ring"))
        #expect(text.setsQuery(axes: SearchAxes(offer: .offered)) == SetsQuery(q: "Sol Ring", offered: true))
        #expect(text.setsQuery(axes: notOwned) == SetsQuery(q: "Sol Ring", owned: false, offered: true))
        #expect(player.setsQuery(axes: SearchAxes(possession: .owned, offer: .all))
            == SetsQuery(owned: true, offered_by: "jace"))
    }

    @Test func onlyAPlayerFreeSearchOffersTheOfferAxis() {
        #expect(text.offersOfferAxis)
        #expect(!player.offersOfferAxis)
    }

    @MainActor
    @Test func changingAnAxisReloadsWithItAndTheSetsFollowBothAxes() async {
        let api = FakeAPI()
        nonisolated(unsafe) var queries: [Query] = []
        nonisolated(unsafe) var setsQueries: [SetsQuery] = []
        api.onSearchCards = { input in
            queries.append(input.query)
            return page([searchCard("a")], total: 1)
        }
        api.onSearchCardSets = { input in
            setsQueries.append(input.query)
            return .ok(.init(body: .json([SetInfo(code: "SOA", name: "Secrets")])))
        }
        let model = SearchResultsViewModel(target: text, api: api, defaults: isolatedDefaults())
        await model.loadInitiallyIfNeeded()
        await model.loadSetsIfNeeded()

        model.axes = SearchAxes(possession: .notOwned, offer: .offered)
        await model.paginator.reload()
        await model.loadSetsIfNeeded()
        await model.loadSetsIfNeeded()

        #expect(queries.map(\.owned) == [nil, false])
        #expect(queries.map(\.offered) == [nil, true])
        #expect(setsQueries == [
            SetsQuery(q: "Sol Ring"),
            SetsQuery(q: "Sol Ring", owned: false, offered: true),
        ])
    }
}
