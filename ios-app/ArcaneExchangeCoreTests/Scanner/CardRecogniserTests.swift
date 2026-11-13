import APIClient
import Testing

@testable import ArcaneExchangeCore

private func entry(_ set: String, _ number: String, rarity: String, name: String, french: String? = nil)
    -> Components.Schemas.ScanIndexEntryResponse {
    .init(collector_number: number, french_name: french, name: name, rarity: rarity, set_code: set)
}

struct CardRecogniserTests {
    let index = ScanIndex(responses: [
        entry("MOM", "123", rarity: "R", name: "Elesh Norn", french: "Elesh Norn, Mère des Machines"),
        entry("MKM", "12", rarity: "C", name: "Hard Evidence"),
        entry("DMU", "5", rarity: "U", name: "Lone Knight"),
    ])

    @Test func recognisesACardFromItsPrintedLine() {
        let card = CardRecogniser.recognise(lines: ["Elesh Norn", "R 0123", "MOM • FR", "Jean Dupont"], in: index)

        #expect(card == RecognisedCard(set: "MOM", number: "123", language: .french, rarity: "R",
                                       name: "Elesh Norn", frenchName: "Elesh Norn, Mère des Machines"))
    }

    @Test func showsTheFrenchNameWhenThereIsOneAndTheEnglishNameOtherwise() {
        let french = CardRecogniser.recognise(lines: ["R 0123", "MOM • EN"], in: index)
        let english = CardRecogniser.recognise(lines: ["C 0012", "MKM • EN"], in: index)

        #expect(french?.displayName == "Elesh Norn, Mère des Machines")
        #expect(english?.displayName == "Hard Evidence")
    }

    @Test func keepsTheLanguageReadOnTheCard() {
        let card = CardRecogniser.recognise(lines: ["R 0123", "MOM • DE"], in: index)

        #expect(card?.language == .german)
    }

    @Test func dropsLeadingZerosOfTheNumber() {
        let card = CardRecogniser.recognise(lines: ["0005/0280 U", "DMU • EN"], in: index)

        #expect(card?.number == "5")
    }

    @Test func dropsASetAndNumberAbsentFromTheIndex() {
        #expect(CardRecogniser.recognise(lines: ["R 0999", "MOM • EN"], in: index) == nil)
        #expect(CardRecogniser.recognise(lines: ["R 0123", "ZZZ • EN"], in: index) == nil)
    }

    @Test func dropsANonNumericNumber() {
        #expect(CardRecogniser.recognise(lines: ["R H013", "MOM • EN"], in: index) == nil)
    }

    @Test func dropsATokenWhoseLetterTWasMisreadAsItsCardsSet() {
        // `tmkm/12` read as `mkm/12`: the rarity read is still T, the index says the card is common.
        #expect(CardRecogniser.recognise(lines: ["T 0012", "MKM • EN"], in: index) == nil)
    }

    @Test func dropsAReadingWhoseRarityDisagreesWithTheIndex() {
        #expect(CardRecogniser.recognise(lines: ["0123/0280 M", "MOM • EN"], in: index) == nil)
    }

    @Test func dropsAReadingWithoutARarityLetter() {
        // `tmkm/12` read as `mkm/12`, with the letter lost: no reading beats a wrong one.
        #expect(CardRecogniser.recognise(lines: ["0012/0280", "MKM • EN"], in: index) == nil)
    }

    @Test func acceptsTheLandLetterForACommonOfTheIndex() {
        // A basic land prints `L`, the index keeps it as a common.
        let card = CardRecogniser.recognise(lines: ["L 0012", "MKM • EN"], in: index)

        #expect(card?.number == "12")
        #expect(card?.rarity == "C")
    }

    @Test func dropsTheLandLetterForACardOfAnotherRarity() {
        #expect(CardRecogniser.recognise(lines: ["L 0123", "MOM • EN"], in: index) == nil)
    }

    @Test func bothFacesOfATransformGiveTheSameCard() {
        let front = CardRecogniser.recognise(lines: ["R 0123", "MOM • EN", "Jean Dupont"], in: index)
        let back = CardRecogniser.recognise(lines: ["R 0123", "MOM • EN", "Marie Martin"], in: index)

        #expect(front != nil)
        #expect(front == back)
    }
}
