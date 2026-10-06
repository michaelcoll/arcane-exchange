import APIClient
import Testing

@testable import ArcaneExchange

/// What `GET /search/card` is asked for, depending on the results screen and the sort picked.
struct SearchResultsSortTests {
    typealias Query = Operations.search_cards.Input.Query

    @Test func textSearchDefaultsToValueDescending() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let query = target.query(filters: target.defaultFilters, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_by: .trend, sort_dir: .desc, q: "Sol Ring"))
    }

    @Test func playerCollectionDefaultsToAddedDescending() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        let query = target.query(filters: target.defaultFilters, page: 0, pageSize: 20)
        #expect(query == Query(
            page: 0,
            page_size: 20,
            sort_by: .added_at,
            sort_dir: .desc,
            player_username: "jace"
        ))
    }

    @Test func playerCollectionSendsTheChosenSort() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        let filters = CollectionFilters(sortBy: .trend, sortDir: .asc)
        let query = target.query(filters: filters, page: 2, pageSize: 20)
        #expect(query == Query(page: 2, page_size: 20, sort_by: .trend, sort_dir: .asc, player_username: "jace"))
    }

    @Test func textSearchSendsTheChosenDirection() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let filters = CollectionFilters(sortBy: .trend, sortDir: .asc)
        let query = target.query(filters: filters, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_by: .trend, sort_dir: .asc, q: "Sol Ring"))
    }

    @Test func textSearchNeverSendsAddedAt() {
        // The API rejects `sort_by=added_at` without `player_username` (400).
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let filters = CollectionFilters(sortBy: .added_at, sortDir: .asc)
        let query = target.query(filters: filters, page: 0, pageSize: 20)
        #expect(query == Query(page: 0, page_size: 20, sort_by: .trend, sort_dir: .asc, q: "Sol Ring"))
    }

    @Test func textSearchOffersValueOnly() {
        #expect(SearchResultsRoute.Target.card(query: "Sol Ring").sortOptions == [.trend])
    }

    @Test func playerCollectionOffersValueAndAdded() {
        #expect(SearchResultsRoute.Target.player(username: "jace").sortOptions == [.trend, .added_at])
    }

    @Test func decklistIsNotSentToTheEndpoint() {
        let query = SearchResultsRoute.Target.decklist.query(filters: CollectionFilters(), page: 0, pageSize: 20)
        #expect(query == nil)
    }

    @MainActor @Test func resultsScreenOpensOnItsDefaultSort() {
        #expect(SearchResultsViewModel(target: .card(query: "Sol Ring")).filters.sortBy == .trend)
        #expect(SearchResultsViewModel(target: .player(username: "jace")).filters.sortBy == .added_at)
    }

    @Test func summaryStatesCountAndSort() {
        #expect(CollectionCopy.sortedSummary(total: 42, sortBy: .trend) == "42 cartes · triées par valeur")
        #expect(CollectionCopy.sortedSummary(total: 1, sortBy: .added_at) == "1 carte · triées par ajout")
    }
}
