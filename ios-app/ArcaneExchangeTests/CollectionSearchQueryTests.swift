import APIClient
import Foundation
import Testing

@testable import ArcaneCore
@testable import ArcaneExchange

/// Split out of `CollectionViewModelTests` (hostless): `SearchResultsRoute` lives in the app.
@MainActor
struct CollectionSearchQueryTests {
    @Test func sendsTheSameRaritiesAndSetsAsTheSearch() async {
        let api = FakeAPI()
        nonisolated(unsafe) var collectionQuery: Operations.get_collection.Input.Query?
        api.onGetCollection = { input in
            collectionQuery = input.query
            return .ok(.init(body: .json(.init(items: [], page: 0, page_size: 20, total: 0))))
        }
        let filters = CollectionFilters(rarities: [.M, .C, .R], sets: ["MH3", "FDN"], sortBy: .added_at, sortDir: .asc)
        let model = CollectionViewModel(api: api, defaults: isolatedDefaults())
        model.filters = filters

        await model.paginator.reload()
        let searchQuery = SearchResultsRoute.Target.card(query: "Sol Ring")
            .query(filters: filters, page: 0, pageSize: 20)

        #expect(collectionQuery?.rarity == [.C, .R, .M])
        #expect(collectionQuery?.sets == "FDN,MH3")
        #expect(collectionQuery?.sort_by == .added_at)
        #expect(collectionQuery?.sort_dir == .asc)
        #expect(searchQuery?.rarity == collectionQuery?.rarity)
        #expect(searchQuery?.sets == collectionQuery?.sets)
    }
}
