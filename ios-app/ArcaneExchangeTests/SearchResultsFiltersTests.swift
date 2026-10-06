import APIClient
import Testing

@testable import ArcaneExchange

/// The rarity filter on the search results screens: what `GET /search/card` is asked for, and
/// which empty state shows when nothing comes back.
struct SearchResultsFiltersTests {
    typealias Query = Operations.search_cards.Input.Query

    @Test func textSearchSendsTheSelectedRaritiesInSchemaOrder() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let filters = CollectionFilters(rarities: [.M, .C, .R])
        let query = target.query(filters: filters, page: 0, pageSize: 20)
        #expect(query == Query(
            page: 0,
            page_size: 20,
            sort_by: .trend,
            sort_dir: .desc,
            q: "Sol Ring",
            rarity: [.C, .R, .M]
        ))
    }

    @Test func playerCollectionSendsTheSelectedRarities() {
        let target = SearchResultsRoute.Target.player(username: "jace")
        let filters = CollectionFilters(rarities: [.S], sortBy: .added_at)
        let query = target.query(filters: filters, page: 1, pageSize: 20)
        #expect(query == Query(
            page: 1,
            page_size: 20,
            sort_by: .added_at,
            sort_dir: .desc,
            rarity: [.S],
            player_username: "jace"
        ))
    }

    @Test func noRaritySelectedSendsNoRarityParameter() {
        let target = SearchResultsRoute.Target.card(query: "Sol Ring")
        let query = target.query(filters: CollectionFilters(), page: 0, pageSize: 20)
        #expect(query?.rarity == nil)
    }

    @MainActor @Test func aNewSearchStartsWithoutFilters() {
        #expect(SearchResultsViewModel(target: .card(query: "Sol Ring")).filters.activeCount == 0)
        #expect(SearchResultsViewModel(target: .player(username: "jace")).filters.activeCount == 0)
    }

    @Test func withoutFiltersAnEmptyListKeepsTheNoOfferMessage() {
        let state = SearchResultsEmptyState(filters: CollectionFilters(sortDir: .asc))
        #expect(state == .noMatch)
        #expect(state.title == "Aucun résultat")
        #expect(state.message == "Personne ne propose de carte correspondant à cette recherche.")
        #expect(!state.offersClearFilters)
    }

    @Test func withActiveFiltersAnEmptyListOffersToClearThem() {
        let state = SearchResultsEmptyState(filters: CollectionFilters(rarities: [.M]))
        #expect(state == .noMatchWithFilters)
        #expect(state.title == "Aucun résultat avec ces filtres")
        #expect(state.offersClearFilters)
    }
}
