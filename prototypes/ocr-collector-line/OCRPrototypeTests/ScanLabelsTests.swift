import Testing
@testable import OCRPrototype

struct ScanLabelsTests {
    private let spirit = ScryfallCard(
        name: "Spirit", printedName: nil, set: "tmkm", number: "12", language: "en", layout: "token", faceNames: [])
    private let delney = ScryfallCard(
        name: "Delney", printedName: nil, set: "mkm", number: "12", language: "fr", layout: "normal", faceNames: [])

    @Test func latencyHasAFrenchLabel() {
        #expect(ScanLabels.latency(0.5) == "Lu en 0.50 s")
        #expect(ScanLabels.latency(nil) == "Pas encore de lecture")
    }

    @Test func readingSaysWhetherTheRarityMakesItAToken() {
        let token = CollectorReading(set: "mkm", number: "12", language: "en", isToken: true)
        #expect(ScanLabels.reading(token).contains("rareté T : token"))
        #expect(ScanLabels.reading(CollectorReading(set: "mkm", number: "12", language: "en", isToken: false))
            .contains("carte ou token"))
        #expect(ScanLabels.reading(nil) == "-")
    }

    @Test func bothCandidatesAreShownAndToldApart() {
        let text = ScanLabels.lookup(.found([
            LookupMatch(card: delney, isToken: false, isEnglishPrinting: false),
            LookupMatch(card: spirit, isToken: true, isEnglishPrinting: true),
        ]))
        #expect(text.contains("CARTE : Delney"))
        #expect(text.contains("TOKEN : Spirit"))
        #expect(text.components(separatedBy: "version anglaise affichée").count == 2)
    }
}
