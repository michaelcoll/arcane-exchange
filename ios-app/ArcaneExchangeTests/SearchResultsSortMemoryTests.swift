import Foundation
import Testing

@testable import ArcaneExchange

/// The sort picked on the search results screens, remembered on the device across launches.
@MainActor
struct SearchResultsSortMemoryTests {
    @Test func aTextSearchRestoresTheSortPickedOnThePreviousOne() {
        let defaults = isolatedDefaults()
        SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters.sortDir = .asc

        let next = SearchResultsViewModel(target: .card(query: "Lightning Bolt"), api: FakeAPI(), defaults: defaults)
        #expect(next.filters.sortBy == .trend)
        #expect(next.filters.sortDir == .asc)
    }

    /// One setting for every player's collection, not one per player.
    @Test func aPlayerCollectionSortAppliesToAnyOtherPlayer() {
        let defaults = isolatedDefaults()
        SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters
            = CollectionFilters(sortBy: .trend, sortDir: .asc)

        let other = SearchResultsViewModel(target: .player(username: "chandra"), api: FakeAPI(), defaults: defaults)
        #expect(other.filters == CollectionFilters(sortBy: .trend, sortDir: .asc))
    }

    @Test func theTwoSettingsDoNotAffectEachOther() {
        let defaults = isolatedDefaults()
        SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters
            = CollectionFilters(sortBy: .trend, sortDir: .asc)
        SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters.sortDir = .desc

        #expect(SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters
            == CollectionFilters(sortBy: .trend, sortDir: .asc))

        SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters.sortDir = .desc
        SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters.sortDir = .asc

        #expect(SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters.sortDir == .desc)
        #expect(SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters.sortDir == .asc)
    }

    /// Nothing remembered: each screen opens on its own default.
    @Test func eachScreenOpensOnItsDefaultSortWhenNothingIsRemembered() {
        let defaults = isolatedDefaults()
        #expect(SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters
            == CollectionFilters(sortBy: .trend, sortDir: .desc))
        #expect(SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters
            == CollectionFilters(sortBy: .added_at, sortDir: .desc))
    }

    /// Only the sort is remembered: the filters start empty on each new search.
    @Test func filtersAreNotRemembered() {
        let defaults = isolatedDefaults()
        SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters
            = CollectionFilters(rarities: [.R], sets: ["MH3"], sortBy: .trend, sortDir: .asc)

        let next = SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults)
        #expect(next.filters == CollectionFilters(sortBy: .trend, sortDir: .asc))
    }

    @Test func signingOutForgetsBothSettings() {
        let defaults = isolatedDefaults()
        SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters.sortBy = .trend
        SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters.sortDir = .asc

        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: defaults, widgetDefaults: defaults)

        #expect(SearchResultsViewModel(target: .player(username: "jace"), api: FakeAPI(), defaults: defaults).filters
            == CollectionFilters(sortBy: .added_at, sortDir: .desc))
        #expect(SearchResultsViewModel(target: .card(query: "Sol Ring"), api: FakeAPI(), defaults: defaults).filters
            == CollectionFilters(sortBy: .trend, sortDir: .desc))
    }
}
