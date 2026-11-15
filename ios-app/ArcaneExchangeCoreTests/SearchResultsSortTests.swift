import APIClient
import Testing

@testable import ArcaneExchangeCore

/// What `GET /search/card` is asked for, depending on the results screen and the sort picked.
struct SearchResultsSortTests {
    typealias Query = Operations.search_cards.Input.Query

    @Test func textSearchDefaultsToValueDescending() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let query = target.query(filters: target.defaultFilters, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_dir: .desc, q: "Sol Ring"))
    }

    @Test func playerCollectionDefaultsToValueDescendingToo() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        let query = target.query(filters: target.defaultFilters, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_dir: .desc, offered_by: "jace"))
    }

    @Test func playerCollectionSendsTheChosenDirection() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        let filters = CollectionFilters(sortBy: .trend, sortDir: .asc)
        let query = target.query(filters: filters, page: 2, pageSize: 20)
        #expect(query == Query(page: 2, page_size: 20, sort_dir: .asc, offered_by: "jace"))
    }

    @Test func textSearchSendsTheChosenDirection() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let filters = CollectionFilters(sortBy: .trend, sortDir: .asc)
        let query = target.query(filters: filters, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_dir: .asc, q: "Sol Ring"))
    }

    @Test func aSearchNeverOffersTheDateAdded() {
        // The date added belongs to the Collection: a remembered one falls back to the value.
        let target = SearchResultsRoute.Target.player(username: "jace")
        #expect(target.offeredSort(.added_at) == .trend)
    }

    @Test func everySearchOffersValueOnly() {
        #expect(SearchResultsRoute.Target.card(query: "Sol Ring").sortOptions == [.trend])
        #expect(SearchResultsRoute.Target.player(username: "jace").sortOptions == [.trend])
    }

    @Test func summaryStatesCountAndSort() {
        #expect(CollectionCopy.sortedSummary(total: 42, sortBy: .trend) == "42 cartes · triées par valeur")
        #expect(CollectionCopy.sortedSummary(total: 1, sortBy: .added_at) == "1 carte · triées par ajout")
    }
}
