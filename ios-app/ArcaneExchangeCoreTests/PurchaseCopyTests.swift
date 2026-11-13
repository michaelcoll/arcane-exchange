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
