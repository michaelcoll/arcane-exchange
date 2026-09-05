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

    /// The grid offers what a card is worth and when it arrived — nothing that duplicates a
    /// filter facet.
    @Test func exposesOnlyValueAndAddedAtSorts() {
        #expect(SortField.collectionOptions == [.trend, .added_at])
    }

    @Test func flipsTheArrowWithTheSortDirection() {
        #expect(SortDirection.desc.icon == "arrow.down")
        #expect(SortDirection.asc.icon == "arrow.up")
        #expect(SortDirection.desc.icon != SortDirection.asc.icon)
        #expect(SortDirection.desc.label == "Décroissant")
        #expect(SortDirection.asc.label == "Croissant")
    }

    @Test func readsAnEmptyFacetAsNoRestriction() {
        #expect(CollectionCopy.facetSelection(selected: 0, total: 14, noneSelected: "Tous") == "Tous")
        #expect(CollectionCopy.facetSelection(selected: 2, total: 14, noneSelected: "Tous") == "2 sur 14")
    }

    /// The sets come from a request of their own, so the drawer can be opened before the total
    /// is known — "2 sur 0" would be a lie, "2" is not.
    @Test func dropsTheTotalUntilTheFacetListHasLoaded() {
        #expect(CollectionCopy.facetSelection(selected: 2, total: 0, noneSelected: "Tous") == "2")
        #expect(CollectionCopy.facetSelection(selected: 0, total: 0, noneSelected: "Tous") == "Tous")
    }

    @Test func countsTheActiveFiltersOnTheChip() {
        #expect(CollectionCopy.filterChip(activeCount: 0) == "Filtres")
        #expect(CollectionCopy.filterChip(activeCount: 3) == "Filtres · 3")
    }

    // MARK: Sets search

    private static let sets = [
        SetInfo(code: "MH3", name: "Modern Horizons 3"),
        SetInfo(code: "LTR", name: "The Lord of the Rings"),
        SetInfo(code: "FDN", name: "Foundations"),
        SetInfo(code: "EOE", name: "Édition Éternelle")
    ]

    @Test func keepsEverySetWhenTheQueryIsBlank() {
        #expect(SetSearch.filter(Self.sets, matching: "").count == 4)
        #expect(SetSearch.filter(Self.sets, matching: "   ").count == 4)
    }

    @Test func matchesTheSetNameOrItsCode() {
        #expect(SetSearch.filter(Self.sets, matching: "horizons").map(\.code) == ["MH3"])
        #expect(SetSearch.filter(Self.sets, matching: "ltr").map(\.code) == ["LTR"])
        #expect(SetSearch.filter(Self.sets, matching: "fou").map(\.code) == ["FDN"])
    }

    @Test func ignoresCaseAndAccents() {
        #expect(SetSearch.filter(Self.sets, matching: "MODERN").map(\.code) == ["MH3"])
        #expect(SetSearch.filter(Self.sets, matching: "edition eternelle").map(\.code) == ["EOE"])
    }

    @Test func returnsNothingWhenNoSetMatches() {
        #expect(SetSearch.filter(Self.sets, matching: "zzz").isEmpty)
    }

    @Test func pluralizesTheCardCount() {
        #expect(CollectionCopy.cardCount(0) == "0 carte")
        #expect(CollectionCopy.cardCount(1) == "1 carte")
        #expect(CollectionCopy.cardCount(8) == "8 cartes")
    }

    @Test func pluralizesTheOwnersHeaders() {
        #expect(CollectionCopy.offerCount(1) == "1 joueur la propose")
        #expect(CollectionCopy.offerCount(3) == "3 joueurs la proposent")
        #expect(CollectionCopy.offerAvailability(available: 1, reserved: 0) == "1 disponible")
        #expect(CollectionCopy.offerAvailability(available: 2, reserved: 1) == "2 disponibles · 1 réservée")
        #expect(CollectionCopy.offerAvailability(available: 0, reserved: 2) == "0 disponible · 2 réservées")
    }
}
