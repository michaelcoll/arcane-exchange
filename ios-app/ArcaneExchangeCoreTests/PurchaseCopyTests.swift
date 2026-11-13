import Testing

@testable import ArcaneExchangeCore

struct PurchaseCopyTests {
    @Test(arguments: [
        (SealedProductType.BOOSTER, "Booster"),
        (.DISPLAY, "Display"),
        (.BUNDLE, "Bundle"),
        (.PRERELEASE_PACK, "Pack d'avant-première"),
        (.PRECONSTRUCTED_DECK, "Deck préconstruit")
    ])
    func labelsEachType(type: SealedProductType, label: String) {
        #expect(PurchaseCopy.label(for: type) == label)
    }

    @Test func anUnnamedProductIsCalledByItsTypeAndSet() {
        let product = purchase("1", type: .DISPLAY, set: ("ECL", "Lorwyn Eclipsed"), on: "2026-11-13")

        #expect(PurchaseCopy.title(of: product) == "Display · Lorwyn Eclipsed")
        #expect(PurchaseCopy.subtitle(of: product) == "13 nov. 2026")
    }

    @Test func aNamedProductShowsItsNameAndKeepsTheTypeAndSetBelow() {
        let product = purchase("1", type: .PRECONSTRUCTED_DECK, name: "Deck Bleu", on: "2026-11-13")

        #expect(PurchaseCopy.title(of: product) == "Deck Bleu")
        #expect(PurchaseCopy.subtitle(of: product) == "Deck préconstruit · Foundations · 13 nov. 2026")
    }

    @Test func aChoiceShowsTheNameOfAProductThatHasOneOrItsTypeAndSetWithItsDate() {
        let unnamed = purchase("1", type: .DISPLAY, set: ("ECL", "Lorwyn Eclipsed"), on: "2026-11-13")
        let named = purchase("2", type: .PRECONSTRUCTED_DECK, name: "Deck Bleu", on: "2026-11-01")

        #expect(PurchaseCopy.choice(of: unnamed) == "Display · Lorwyn Eclipsed · 13 nov. 2026")
        #expect(PurchaseCopy.choice(of: named) == "Deck Bleu · 1 nov. 2026")
    }

    @Test(arguments: [
        (855, 12.4, "+8,55\u{00A0}€ (+12 %)"),
        (-300, -75.0, "−3\u{00A0}€ (−75 %)"),
        (0, 0.0, "0\u{00A0}€ (0 %)"),
        (-1, -0.04, "−0,01\u{00A0}€ (0 %)")
    ])
    func writesTheProfitabilityWithItsSign(cents: Int, percent: Double, label: String) {
        #expect(PurchaseCopy.profitability(cents: cents, percent: percent) == label)
    }

    @Test func countsTheLinesLeftOutOfTheValue() {
        #expect(PurchaseCopy.withoutTrend(0) == nil)
        #expect(PurchaseCopy.withoutTrend(1) == "1 carte sans prix de tendance, exclue de la valeur")
        #expect(PurchaseCopy.withoutTrend(3) == "3 cartes sans prix de tendance, exclues de la valeur")
    }

    @Test func aSingleCardPurchaseIsCalledByItsCardAndDescribedByItsCopies() {
        let normal = singleCardPurchase("1", card: "Gobelin", foil: false, quantity: 4)
        let foil = singleCardPurchase("2", card: "Gobelin", foil: true, quantity: 1)

        #expect(PurchaseCopy.title(of: normal) == "Gobelin")
        #expect(PurchaseCopy.subtitle(of: normal) == "Foundations · 4 × FR · 13 nov. 2026")
        #expect(PurchaseCopy.subtitle(of: foil) == "Foundations · 1 × FR · Foil · 13 nov. 2026")
    }

    @Test(arguments: [
        ("49,99", 4999),
        ("49.99", 4999),
        ("50", 5000),
        (" 0,5 ", 50),
        ("1234", 123_400)
    ])
    func readsAnAmountInEurosAsCents(text: String, cents: Int) {
        #expect(EuroInput.cents(from: text) == cents)
    }

    @Test(arguments: ["", "0", "0,00", "-5", "abc", "4,999", "4,9x", "1 000"])
    func refusesWhatIsNotAPositiveAmount(text: String) {
        #expect(EuroInput.cents(from: text) == nil)
    }

    @Test func writesCentsBackAsTheFieldShowsThem() {
        #expect(EuroInput.text(cents: 4999) == "49,99")
        #expect(EuroInput.text(cents: 5000) == "50")
        #expect(EuroInput.text(cents: 120_000) == "1200")
    }

    @Test func aDayRoundTripsThroughItsWireForm() {
        let date = PurchaseDate.date(from: "2026-11-13")

        #expect(date.map(PurchaseDate.string(from:)) == "2026-11-13")
        #expect(PurchaseDate.display("not a date") == "not a date")
    }
}
