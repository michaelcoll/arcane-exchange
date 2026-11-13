import Testing

@testable import ArcaneExchangeCore

struct ScanBatchCopyTests {
    @Test(arguments: [(0, "0 carte"), (1, "1 carte"), (2, "2 cartes"), (40, "40 cartes")])
    func countsCardsWithTheFrenchPlural(count: Int, text: String) {
        #expect(ScanBatchCopy.cards(count) == text)
    }

    @Test func namesTheActionThatAddsTheBatchToTheCollection() {
        #expect(ScanBatchCopy.addToCollection(cards: 1) == "Ajouter 1 carte à la collection")
        #expect(ScanBatchCopy.addToCollection(cards: 7) == "Ajouter 7 cartes à la collection")
        #expect(ScanBatchCopy.add(cards: 2) == "Ajouter 2 cartes")
    }

    @Test func tellsHowManyLinesHaveNoPrice() {
        #expect(ScanBatchCopy.valueText(cents: 1250, unpricedLines: 0) == "Valeur \(Price.euros(cents: 1250))")
        #expect(ScanBatchCopy.valueText(cents: 1250, unpricedLines: 1)
            == "Valeur \(Price.euros(cents: 1250)) · 1 ligne sans prix")
        #expect(ScanBatchCopy.valueText(cents: 1250, unpricedLines: 3)
            == "Valeur \(Price.euros(cents: 1250)) · 3 lignes sans prix")
    }
}
