import Foundation
import Testing

@testable import ArcaneExchangeCore

struct ScanBatchCorrectionTests {
    private func card(_ number: String, set: String = "MOM", language: CardLanguage = .french) -> RecognisedCard {
        RecognisedCard(set: set, number: number, language: language, rarity: "C", name: "Card \(number)",
                       frenchName: nil)
    }

    private func batch(_ cards: (RecognisedCard, foil: Bool)...) -> ScanBatch {
        var batch = ScanBatch()
        for (card, foil) in cards {
            _ = batch.addFromCamera(card, foil: foil)
        }
        return batch
    }

    @Test func changesTheLanguageOfALine() {
        var batch = batch((card("1"), false))

        batch.setLanguage(.english, ofLine: batch.lines[0].id)

        #expect(batch.lines.count == 1)
        #expect(batch.lines[0].language == .english)
        #expect(batch.lines[0].number == "1")
    }

    @Test func changesTheFinishOfALine() {
        var batch = batch((card("1"), false))

        batch.setFoil(true, ofLine: batch.lines[0].id)

        #expect(batch.lines[0].foil)
        #expect(batch.lines[0].quantity == 1)
    }

    @Test func keepsTheLinePlaceWhenItIsCorrected() {
        var batch = batch((card("1"), false), (card("2"), false), (card("3"), false))
        let middle = batch.lines[1].id

        batch.setLanguage(.german, ofLine: middle)

        #expect(batch.lines.map(\.number) == ["3", "2", "1"])
    }

    @Test func mergesTwoLinesMadeIdenticalByACorrection() {
        var batch = batch((card("1", language: .english), false), (card("2"), false), (card("1"), false))
        batch.setQuantity(3, ofLine: batch.lines[2].id)
        let french = batch.lines[0].id

        batch.setLanguage(.english, ofLine: french)

        #expect(batch.lines.count == 2)
        let merged = batch.lines.first { $0.number == "1" }
        #expect(merged?.language == .english)
        #expect(merged?.quantity == 4)
        #expect(batch.cardCount == 5)
    }

    @Test func capsAMergedLineAtTheMaximumQuantity() {
        var batch = batch((card("1", language: .english), false), (card("2"), false), (card("1"), false))
        batch.setQuantity(200, ofLine: batch.lines[2].id)
        batch.setQuantity(200, ofLine: batch.lines[0].id)

        batch.setLanguage(.english, ofLine: batch.lines[0].id)

        #expect(batch.lines.first { $0.number == "1" }?.quantity == 255)
    }

    @Test func ignoresACorrectionOfAnUnknownLine() {
        var batch = batch((card("1"), false))
        let before = batch

        let unknown = ScanBatchLine.Key(set: "ZZZ", number: "9", language: .english, foil: false)
        batch.setLanguage(.english, ofLine: unknown)
        batch.setFoil(true, ofLine: unknown)

        #expect(batch == before)
    }

    @Test func replacesTheCardOfALineKeepingQuantityLanguageAndFinish() {
        var batch = batch((card("1", language: .german), true))
        batch.setQuantity(3, ofLine: batch.lines[0].id)

        batch.replaceCard(ofLine: batch.lines[0].id, with: card("99", set: "ONE", language: .french))

        #expect(batch.lines.count == 1)
        let line = batch.lines[0]
        #expect(line.set == "ONE")
        #expect(line.number == "99")
        #expect(line.displayName == "Card 99")
        #expect(line.quantity == 3)
        #expect(line.language == .german)
        #expect(line.foil)
    }

    @Test func mergesAReplacedLineWithTheLineOfTheChosenCard() {
        var batch = batch((card("99"), false), (card("1"), false))
        batch.setQuantity(2, ofLine: batch.lines[1].id)

        batch.replaceCard(ofLine: batch.lines[0].id, with: card("99"))

        #expect(batch.lines.count == 1)
        #expect(batch.lines[0].quantity == 3)
    }

    @Test func addsACatalogCardWithQuantityOneAtTheTop() {
        var batch = batch((card("1"), false))

        batch.addFromCatalog(card("2"), language: .japanese, foil: true)

        #expect(batch.lines.map(\.number) == ["2", "1"])
        #expect(batch.lines[0].quantity == 1)
        #expect(batch.lines[0].language == .japanese)
        #expect(batch.lines[0].foil)
    }

    @Test func addingACatalogCardAlreadyInTheBatchRaisesItsQuantity() {
        var batch = batch((card("1"), false), (card("2"), false))

        batch.addFromCatalog(card("1"), language: .french, foil: false)

        #expect(batch.lines.map(\.number) == ["1", "2"])
        #expect(batch.lines[0].quantity == 2)
    }

    @Test func aCorrectionWithdrawsTheUndoOfTheLastCameraAdd() {
        var batch = batch((card("1"), false))

        batch.setFoil(true, ofLine: batch.lines[0].id)
        let undone = batch.undoLastCameraAdd()

        #expect(!undone)
        #expect(batch.cardCount == 1)
    }

    @Test func aCatalogAdditionWithdrawsTheUndoOfTheLastCameraAdd() {
        var batch = batch((card("1"), false))

        batch.addFromCatalog(card("2"), language: .french, foil: false)
        let undone = batch.undoLastCameraAdd()

        #expect(!undone)
        #expect(batch.cardCount == 2)
    }

    @Test func correctionsSurviveAnEncodingRoundTrip() throws {
        var batch = batch((card("1"), false))
        batch.setLanguage(.english, ofLine: batch.lines[0].id)
        batch.addFromCatalog(card("2"), language: .french, foil: true)

        let decoded = try JSONDecoder().decode(ScanBatch.self, from: JSONEncoder().encode(batch))

        #expect(decoded == batch)
    }
}
