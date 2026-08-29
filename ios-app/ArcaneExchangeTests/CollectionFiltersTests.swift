import Testing

@testable import ArcaneExchange

struct CollectionFiltersTests {
    @Test func countsEveryActiveFacet() {
        var filters = CollectionFilters()
        #expect(filters.activeCount == 0)

        filters.rarities = [.R, .M]
        filters.sets = ["MH3"]
        #expect(filters.activeCount == 3)
    }

    @Test func clearAllKeepsTheSortUntouched() {
        var filters = CollectionFilters(rarities: [.C], sets: ["EOE"], sortBy: .added_at, sortDir: .asc)
        filters.clearAll()

        #expect(filters.activeCount == 0)
        #expect(filters.sortBy == .added_at)
        #expect(filters.sortDir == .asc)
    }

    @Test func labelsEveryRarityAndExposedSortField() {
        #expect(RarityCode.allCases.allSatisfy { !$0.label.isEmpty })
        #expect(SortField.collectionOptions.allSatisfy { !$0.label.isEmpty })
    }

    @Test func pluralizesTheCardCount() {
        #expect(CollectionCopy.cardCount(0) == "0 carte")
        #expect(CollectionCopy.cardCount(1) == "1 carte")
        #expect(CollectionCopy.cardCount(8) == "8 cartes")
    }
}
