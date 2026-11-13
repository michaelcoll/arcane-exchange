import Foundation
import Testing

@testable import ArcaneExchangeCore

struct ScanBatchTests {
    private func card(_ number: String, set: String = "MOM", language: CardLanguage = .french) -> RecognisedCard {
        RecognisedCard(set: set, number: number, language: language, rarity: "C", name: "Card \(number)",
                       frenchName: nil)
    }

    @Test func addsARecognisedCardAsALineOfQuantityOne() {
        var batch = ScanBatch()

        let added = batch.addFromCamera(card("1"), foil: false)

        #expect(added)
        #expect(batch.lines.count == 1)
        #expect(batch.lines[0].quantity == 1)
        #expect(batch.lines[0].set == "MOM")
        #expect(batch.lines[0].number == "1")
        #expect(batch.lines[0].language == .french)
        #expect(batch.lines[0].foil == false)
        #expect(batch.lines[0].displayName == "Card 1")
        #expect(batch.cardCount == 1)
    }

    @Test func putsTheMostRecentLineFirst() {
        var batch = ScanBatch()

        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.addFromCamera(card("2"), foil: false)

        #expect(batch.lines.map(\.number) == ["2", "1"])
    }

    @Test func ignoresTheSameCardReadTwiceInARow() {
        var batch = ScanBatch()

        _ = batch.addFromCamera(card("1"), foil: false)
        let added = batch.addFromCamera(card("1"), foil: false)

        #expect(!added)
        #expect(batch.cardCount == 1)
    }

    @Test func ignoresTheSameCardInARowWhateverTheFoilSwitchSays() {
        var batch = ScanBatch()

        _ = batch.addFromCamera(card("1"), foil: false)
        let added = batch.addFromCamera(card("1"), foil: true)

        #expect(!added)
        #expect(batch.cardCount == 1)
    }

    @Test func countsTheSameCardAgainOnceAnotherCardWasAddedBetween() {
        var batch = ScanBatch()

        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.addFromCamera(card("2"), foil: false)
        let added = batch.addFromCamera(card("1"), foil: false)

        #expect(added)
        #expect(batch.lines.map(\.number) == ["1", "2"])
        #expect(batch.lines[0].quantity == 2)
        #expect(batch.cardCount == 3)
    }

    @Test func countsTheSameCardInAnotherLanguageAsAnotherCard() {
        var batch = ScanBatch()

        _ = batch.addFromCamera(card("1", language: .french), foil: false)
        let added = batch.addFromCamera(card("1", language: .english), foil: false)

        #expect(added)
        #expect(batch.lines.count == 2)
    }

    @Test func keepsOneLinePerFinish() {
        var batch = ScanBatch()

        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.addFromCamera(card("2"), foil: false)
        _ = batch.addFromCamera(card("1"), foil: true)

        #expect(batch.lines.count == 3)
        #expect(batch.lines[0].foil)
    }

    @Test func undoesTheCardJustAdded() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.addFromCamera(card("2"), foil: false)

        let undone = batch.undoLastCameraAdd()

        #expect(undone)
        #expect(batch.lines.map(\.number) == ["1"])
    }

    @Test func undoOnlyTakesOffTheCopyJustAdded() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.addFromCamera(card("2"), foil: false)
        _ = batch.addFromCamera(card("1"), foil: false)

        _ = batch.undoLastCameraAdd()

        #expect(batch.lines.first { $0.number == "1" }?.quantity == 1)
        #expect(batch.cardCount == 2)
    }

    @Test func countsTheSameCardAgainAfterAnUndo() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.undoLastCameraAdd()

        let added = batch.addFromCamera(card("1"), foil: false)

        #expect(added)
        #expect(batch.cardCount == 1)
    }

    @Test func cannotUndoTwice() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        _ = batch.addFromCamera(card("2"), foil: false)
        _ = batch.undoLastCameraAdd()

        let undone = batch.undoLastCameraAdd()

        #expect(!undone)
        #expect(batch.cardCount == 1)
    }

    @Test func cannotUndoOnceALineWasCorrectedByHand() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        batch.setQuantity(4, ofLine: batch.lines[0].id)

        let undone = batch.undoLastCameraAdd()

        #expect(!undone)
        #expect(batch.cardCount == 4)
    }

    @Test func setsTheQuantityOfALine() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)

        batch.setQuantity(3, ofLine: batch.lines[0].id)

        #expect(batch.lines[0].quantity == 3)
    }

    @Test func removesALineWhoseQuantityGoesDownToZero() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)

        batch.setQuantity(0, ofLine: batch.lines[0].id)

        #expect(batch.lines.isEmpty)
    }

    @Test func keepsTheQuantityBetweenZeroAnd255() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        let id = batch.lines[0].id

        batch.setQuantity(300, ofLine: id)
        #expect(batch.lines[0].quantity == 255)

        batch.setQuantity(-2, ofLine: id)
        #expect(batch.lines.isEmpty)
    }

    @Test func ignoresACameraAddOnceTheLineIsFull() {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: false)
        batch.setQuantity(255, ofLine: batch.lines[0].id)
        _ = batch.addFromCamera(card("2"), foil: false)

        let added = batch.addFromCamera(card("1"), foil: false)

        #expect(!added)
        #expect(batch.lines.first { $0.number == "1" }?.quantity == 255)
    }

    @Test func clearsTheLinesAndStartsANewBatchIdentity() {
        var batch = ScanBatch()
        let before = batch.id
        _ = batch.addFromCamera(card("1"), foil: false)

        batch.clear()

        #expect(batch.lines.isEmpty)
        #expect(batch.id != before)
        // The same card counts again in the emptied batch.
        let added = batch.addFromCamera(card("1"), foil: false)
        #expect(added)
    }

    @Test func eachBatchGetsItsOwnIdentifier() {
        #expect(ScanBatch().id != ScanBatch().id)
    }

    @Test func survivesAnEncodingRoundTripWithItsIdentifier() throws {
        var batch = ScanBatch()
        _ = batch.addFromCamera(card("1"), foil: true)
        _ = batch.addFromCamera(card("2"), foil: false)

        let decoded = try JSONDecoder().decode(ScanBatch.self, from: JSONEncoder().encode(batch))

        #expect(decoded == batch)
        // The re-read comparison survives too.
        var again = decoded
        let added = again.addFromCamera(card("2"), foil: false)
        #expect(!added)
    }
}
