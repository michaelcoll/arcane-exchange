import APIClient
import Testing

@testable import ArcaneExchangeCore

/// The set filter on the search results screens: which facet `GET /search/card/sets` is asked
/// for, and how the selection reaches `GET /search/card`.
struct SearchResultsSetsTests {
    typealias Query = Operations.search_cards.Input.Query
    typealias SetsQuery = Operations.search_card_sets.Input.Query

    @Test func textSearchAsksForTheSetsOfTheOfferedCardsMatchingItsQuery() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        #expect(target.setsQuery == SetsQuery(q: "Sol Ring", offered: true))
    }

    @Test func playerCollectionAsksForTheSetsThePlayerOffers() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        #expect(target.setsQuery == SetsQuery(offered_by: "jace"))
    }

    @Test func decklistHasNoSetFacet() {
        #expect(SearchResultsRoute.Target.decklist.setsQuery == nil)
    }

    @Test func textSearchSendsTheSelectedSetsSortedAndCommaSeparated() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let filters = CollectionFilters(sets: ["TST", "ABC"])
        let query = target.query(filters: filters, page: 0, pageSize: 20)
        #expect(query == Query(
            page: 0,
            page_size: 20,
            sort_dir: .desc,
            q: "Sol Ring",
            offered: true,
            sets: "ABC,TST"
        ))
    }

    @Test func playerCollectionSendsTheSelectedSets() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        let filters = CollectionFilters(sets: ["MH3"])
        let query = target.query(filters: filters, page: 0, pageSize: 20)
        #expect(query?.sets == "MH3")
        #expect(query?.offered_by == "jace")
    }

    @Test func noSetSelectedSendsNoSetsParameter() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let query = target.query(filters: CollectionFilters(rarities: [.M]), page: 0, pageSize: 20)
        #expect(query?.sets == nil)
    }

    @Test func selectedSetsCountInTheFilterChip() {
        let filters = CollectionFilters(rarities: [.M], sets: ["ABC", "TST"])
        #expect(CollectionCopy.filterChip(activeCount: filters.activeCount) == "Filtres · 3")
    }

    @Test func anEmptyListWithOnlySetsSelectedOffersToClearThem() {
        let state = SearchResultsEmptyState(filters: CollectionFilters(sets: ["ABC"]))
        #expect(state == .noMatchWithFilters)
    }

    @Test func clearingTheFiltersAlsoEmptiesTheSetsButKeepsTheSort() {
        var filters = CollectionFilters(rarities: [.M], sets: ["ABC"], sortBy: .added_at, sortDir: .asc)
        filters.clearAll()
        #expect(filters == CollectionFilters(sortBy: .added_at, sortDir: .asc))
    }
}
