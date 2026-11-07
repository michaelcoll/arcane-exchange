import Foundation
import Testing
@testable import OCRPrototype

struct ScryfallLookupPlanTests {
    private func urls(_ reading: CollectorReading) -> [String] {
        ScryfallLookup.plan(for: reading).map(\.url.absoluteString)
    }

    @Test func aCardTriesTheReadLanguageThenEnglish() {
        let plan = ScryfallLookup.plan(for: CollectorReading(set: "mkm", number: "12", language: "fr"))
        #expect(plan.map(\.url.absoluteString) == [
            "https://api.scryfall.com/cards/mkm/12/fr",
            "https://api.scryfall.com/cards/mkm/12",
        ])
        #expect(plan.map(\.isEnglishPrinting) == [false, true])
    }

    @Test func anEnglishCardNeedsOneRequest() {
        #expect(urls(CollectorReading(set: "mom", number: "88", language: "en"))
            == ["https://api.scryfall.com/cards/mom/88/en"])
    }

    @Test func theNumberIsPercentEncoded() {
        #expect(urls(CollectorReading(set: "sld", number: "5★", language: "en"))
            == ["https://api.scryfall.com/cards/sld/5%E2%98%85/en"])
    }
}

struct ScryfallDecodingTests {
    @Test func decodesATransformInFrenchWithPrintedNames() throws {
        let card = try #require(ScryfallLookup.decode(ScryfallFixtures.transformFrench))
        #expect(card.displayName == "Agent étherlame // Piqueur d'esprit gitaxien")
        #expect(card.faceNames == ["Aetherblade Agent", "Gitaxian Mindstinger"])
        #expect(card.language == "fr")
    }

    @Test func theErrorBodyIsNotACard() {
        #expect(ScryfallLookup.decode(ScryfallFixtures.notFound) == nil)
    }
}
