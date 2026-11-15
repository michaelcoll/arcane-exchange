import APIClient
import Foundation
import Testing

@testable import ArcaneExchangeCore

struct SearchCardTests {
    private func impression(
        french: String? = nil,
        normal: Int32? = nil,
        foil: Int32? = nil,
        reserved: Bool? = nil,
        offeredLanguages: [String] = []
    ) -> SearchCard {
        SearchCard(
            collector_number: "246",
            foil_price_guide: foil.map { .init(trend: $0) },
            french_name: french,
            name: "Sol Ring",
            offered_languages: offeredLanguages,
            owner_count: 2,
            price_guide: normal.map { .init(trend: $0) },
            rarity_code: "R",
            reserved: reserved,
            scryfall_id: "abc",
            set_code: "LTR",
            set_name: "The Lord of the Rings"
        )
    }

    /// The detail screens read a `CollectionCard`: a search result opens them with its public
    /// figures and no collection entry.
    @Test func opensTheDetailWithoutACollectionEntry() {
        let result = searchCard("abc").asCollectionCard

        #expect(result.scryfall_id == "abc")
        #expect(result.name == "Card abc")
        #expect(result.collection_entry == nil)
    }

    @Test func showsTheFrenchNameWhenThereIsOne() {
        #expect(impression(french: "Anneau solaire").displayName == "Anneau solaire")
        #expect(impression().displayName == "Sol Ring")
    }

    @Test func theReferencePriceIsTheNormalTrend() {
        let card = impression(normal: 500, foil: 900)

        #expect(card.referencePriceCents == 500)
        #expect(!card.isFoilOnly)
    }

    @Test func theReferencePriceIsTheFoilTrendWithoutANormalPrice() {
        let card = impression(foil: 900)

        #expect(card.referencePriceCents == 900)
        #expect(card.isFoilOnly)
    }

    @Test func anUnpricedImpressionHasNoReferencePrice() {
        let card = impression()

        #expect(card.referencePriceCents == nil)
        #expect(!card.isFoilOnly)
    }

    @Test func theDetailOpensOnTheEnglishCardWithTheNormalPrice() {
        let detail = impression(french: "Anneau solaire", normal: 500, foil: 900).asCollectionCard

        #expect(detail.language_code == "EN")
        #expect(!detail.foil)
        #expect(detail.name == "Anneau solaire")
        #expect(detail.price_guide?.trend == 500)
    }

    @Test func theDetailOpensOnTheFrenchCardWhenItIsOffered() {
        #expect(impression(offeredLanguages: ["EN", "FR"]).asCollectionCard.language_code == "FR")
        #expect(impression(offeredLanguages: ["FR"]).asCollectionCard.language_code == "FR")
    }

    @Test func theDetailOpensOnTheEnglishCardWhenFrenchIsNotOffered() {
        #expect(impression(offeredLanguages: ["EN", "JA"]).asCollectionCard.language_code == "EN")
        #expect(impression(offeredLanguages: ["JA"]).asCollectionCard.language_code == "EN")
        #expect(impression(offeredLanguages: []).asCollectionCard.language_code == "EN")
    }

    @Test func theDetailOfAFoilOnlyImpressionOpensInFoil() {
        let detail = impression(foil: 900).asCollectionCard

        #expect(detail.foil)
        #expect(detail.price_guide?.trend == 900)
    }

    @Test func theDetailKeepsTheReservationOfTheSearchedPlayer() {
        #expect(impression(reserved: true).asCollectionCard.reserved)
        #expect(!impression().asCollectionCard.reserved)
    }
}
