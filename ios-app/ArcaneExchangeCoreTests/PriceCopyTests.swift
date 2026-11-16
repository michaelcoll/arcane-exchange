import APIClient
import Testing

@testable import ArcaneExchangeCore

private func card(normal: Int?, foil: Int?, frenchName: String? = "Anneau solaire") -> SearchCard {
    var card = searchCard("a")
    card.name = "Sol Ring"
    card.french_name = frenchName
    card.price_guide = normal.map { .init(avg: nil, low: nil, trend: Int32($0)) }
    card.foil_price_guide = foil.map { .init(avg: nil, low: nil, trend: Int32($0)) }
    return card
}

/// The prices are written as the app writes them: a no-break space before the euro sign.
struct PriceCopyTests {
    @Test func givesTheNormalPriceAlone() {
        #expect(PriceCopy.text(of: card(normal: 900, foil: nil)) == "Anneau solaire vaut 9\u{00A0}€.")
    }

    @Test func givesTheNormalAndTheFoilPrices() {
        #expect(
            PriceCopy.text(of: card(normal: 950, foil: 2800))
                == "Anneau solaire vaut 9,5\u{00A0}€, et 28\u{00A0}€ en foil."
        )
    }

    @Test func givesTheFoilPriceAlone() {
        #expect(
            PriceCopy.text(of: card(normal: nil, foil: 2800)) == "Anneau solaire vaut 28\u{00A0}€ en foil."
        )
    }

    @Test func saysThereIsNoPriceRatherThanZero() {
        #expect(PriceCopy.text(of: card(normal: nil, foil: nil)) == "Anneau solaire n'a pas de prix.")
    }

    @Test func namesTheCardInEnglishWithoutAFrenchName() {
        #expect(
            PriceCopy.text(of: card(normal: 900, foil: nil, frenchName: nil)) == "Sol Ring vaut 9\u{00A0}€."
        )
    }
}
