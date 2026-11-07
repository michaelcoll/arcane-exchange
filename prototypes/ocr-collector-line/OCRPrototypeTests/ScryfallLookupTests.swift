import Foundation
import Testing
@testable import OCRPrototype

struct ScryfallLookupPlanTests {
    private func reading(_ set: String, _ number: String, _ language: String, token: Bool) -> CollectorReading {
        CollectorReading(set: set, number: number, language: language, isToken: token)
    }

    private func urls(_ target: LookupTarget) -> [String] { target.steps.map(\.url.absoluteString) }

    @Test func aCardTriesTheReadLanguageThenEnglishAndAlsoTheTokenSet() {
        let plan = ScryfallLookup.plan(for: reading("mkm", "12", "fr", token: false))
        #expect(plan.map(\.isToken) == [false, true])
        #expect(urls(plan[0]) == [
            "https://api.scryfall.com/cards/mkm/12/fr",
            "https://api.scryfall.com/cards/mkm/12",
        ])
        #expect(plan[0].steps.map(\.isEnglishPrinting) == [false, true])
        #expect(urls(plan[1]) == ["https://api.scryfall.com/cards/tmkm/12"])
        #expect(plan[1].steps.map(\.isEnglishPrinting) == [true])
    }

    @Test func aTokenLooksOnlyInTheTokenSetAndSkipsTheLanguageRequest() {
        let plan = ScryfallLookup.plan(for: reading("mkm", "12", "fr", token: true))
        #expect(plan.map(\.isToken) == [true])
        #expect(urls(plan[0]) == ["https://api.scryfall.com/cards/tmkm/12"])
        #expect(plan[0].steps.map(\.isEnglishPrinting) == [true])
    }

    @Test func anEnglishCardNeedsOneRequestPerSet() {
        let plan = ScryfallLookup.plan(for: reading("mom", "88", "en", token: false))
        #expect(urls(plan[0]) == ["https://api.scryfall.com/cards/mom/88/en"])
        #expect(urls(plan[1]) == ["https://api.scryfall.com/cards/tmom/88"])
        #expect(plan[1].steps.map(\.isEnglishPrinting) == [false])
    }

    @Test func theNumberIsPercentEncoded() {
        let plan = ScryfallLookup.plan(for: reading("ta25", "5★", "en", token: true))
        #expect(urls(plan[0]) == ["https://api.scryfall.com/cards/tta25/5%E2%98%85"])
    }
}

struct ScryfallDecodingTests {
    @Test func decodesADoubleFacedToken() throws {
        let card = try #require(ScryfallLookup.decode(ScryfallFixtures.doubleFacedToken))
        #expect(card == ScryfallCard(
            name: "Incubator // Phyrexian", printedName: nil, set: "tmom", number: "16", language: "en",
            layout: "double_faced_token", faceNames: ["Incubator", "Phyrexian"]))
    }

    @Test func decodesATransformInFrenchWithPrintedNames() throws {
        let card = try #require(ScryfallLookup.decode(ScryfallFixtures.transformFrench))
        #expect(card.displayName == "Agent étherlame // Piqueur d'esprit gitaxien")
        #expect(card.faceNames == ["Aetherblade Agent", "Gitaxian Mindstinger"])
        #expect(card.language == "fr")
    }

    @Test func aRegularTokenHasNoFaces() throws {
        let card = try #require(ScryfallLookup.decode(ScryfallFixtures.regularToken))
        #expect(card.faceNames.isEmpty)
        #expect(card.displayName == "Spirit")
        #expect(card.layout == "token")
    }

    @Test func theErrorBodyIsNotACard() {
        #expect(ScryfallLookup.decode(ScryfallFixtures.notFound) == nil)
    }
}
