import Testing

@testable import ArcaneExchangeCore

struct OffersCopyTests {
    @Test func saysNobodyOffersACardNobodyOwns() {
        #expect(OffersCopy.text(ownerCount: 0) == "Personne ne la propose.")
    }

    @Test func saysOnePlayerOffersIt() {
        #expect(OffersCopy.text(ownerCount: 1) == "1 joueur la propose.")
    }

    @Test func countsThePlayersWhoOfferIt() {
        #expect(OffersCopy.text(ownerCount: 3) == "3 joueurs la proposent.")
    }

    @Test func asksToOpenTheOffersAfterTheAnswer() {
        #expect(OffersCopy.prompt(ownerCount: 3) == "3 joueurs la proposent. Ouvrir les offres ?")
    }
}
