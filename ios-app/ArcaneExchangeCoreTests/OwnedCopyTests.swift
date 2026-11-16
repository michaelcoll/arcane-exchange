import Testing

@testable import ArcaneExchangeCore

struct OwnedCopyTests {
    private func impression(owned: Int64) -> SearchCard {
        var card = searchCard("a")
        card.name = "Sol Ring"
        card.french_name = nil
        card.owned_quantity = owned
        return card
    }

    @Test func saysThePlayerDoesNotHaveACardTheyDoNotOwn() {
        #expect(OwnedCopy.text(of: impression(owned: 0)) == "Tu n'as pas Sol Ring.")
    }

    @Test func saysThePlayerHasOneCopy() {
        #expect(OwnedCopy.text(of: impression(owned: 1)) == "Tu as 1 exemplaire de Sol Ring.")
    }

    @Test func countsTheCopiesTheyOwn() {
        #expect(OwnedCopy.text(of: impression(owned: 3)) == "Tu as 3 exemplaires de Sol Ring.")
    }
}
