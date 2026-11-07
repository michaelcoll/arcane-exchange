import Testing
@testable import OCRPrototype

struct ScanLabelsTests {
    private let delney = ScryfallCard(
        name: "Delney", printedName: nil, set: "mkm", number: "12", language: "en", layout: "normal", faceNames: [])

    @Test func latencyHasAFrenchLabel() {
        #expect(ScanLabels.latency(0.5) == "Lu en 0.50 s")
        #expect(ScanLabels.latency(nil) == "Pas encore de lecture")
    }

    @Test func onlyTheContourModeSaysWhetherACardIsDetected() {
        let contour = CardContour(topLeft: .zero, topRight: .zero, bottomRight: .zero, bottomLeft: .zero)
        #expect(ScanLabels.contour(contour, in: .cardContour) == "Carte détectée")
        #expect(ScanLabels.contour(nil, in: .cardContour) == "Aucune carte détectée")
        #expect(ScanLabels.contour(contour, in: .fixedStrip) == nil)
    }

    @Test func readingShowsSetNumberAndLanguage() {
        #expect(ScanLabels.reading(CollectorReading(set: "mkm", number: "12", language: "fr")) == "MKM n° 12 fr")
        #expect(ScanLabels.reading(nil) == "-")
    }

    @Test func theEnglishPrintingShownForAnotherLanguageIsSaid() {
        let text = ScanLabels.lookup(.found(LookupMatch(card: delney, isEnglishPrinting: true)))
        #expect(text.hasPrefix("Delney"))
        #expect(text.contains("version anglaise affichée"))
        #expect(!ScanLabels.lookup(.found(LookupMatch(card: delney, isEnglishPrinting: false)))
            .contains("version anglaise affichée"))
    }
}
