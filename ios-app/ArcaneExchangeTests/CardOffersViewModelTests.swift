import APIClient
import Testing

@testable import ArcaneExchange

@MainActor
struct CardOffersViewModelTests {
    @Test func loadsTheCheapestOffersFirstPage() async {
        let api = FakeAPI()
        var query: Operations.get_card_offers.Input.Query?
        api.onGetCardOffers = { input in
            query = input.query
            return offersPage([offer("mizzix_42"), offer("jace", reserved: true)], total: 2)
        }
        let model = CardOffersViewModel(card: testCard(), api: api)

        await model.load()

        #expect(model.state == .loaded([offer("mizzix_42"), offer("jace", reserved: true)]))
        #expect(query?.page == 0)
        #expect(query?.page_size == 20)
        #expect(query?.set_code == "FIN")
        #expect(api.calls == ["get_card_offers"])
    }

    @Test func failsWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onGetCardOffers = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = CardOffersViewModel(card: testCard(), api: api)

        await model.load()

        #expect(model.state == .failed)
    }

    @Test func showsNoOffersWhenTheCardIsUnknown() async {
        let api = FakeAPI()
        api.onGetCardOffers = { _ in throw APIError(code: "card_not_found", status: 404) }
        let model = CardOffersViewModel(card: testCard(), api: api)

        await model.load()

        #expect(model.state == .loaded([]))
    }

    @Test func resolvesTheSetName() async {
        let api = FakeAPI()
        api.onGetSet = { _ in .ok(.init(body: .json(.init(code: "FIN", name: "Final Fantasy")))) }
        let model = CardOffersViewModel(card: testCard(), api: api)
        #expect(model.setName == "FIN")

        await model.loadSetName()

        #expect(model.setName == "Final Fantasy")
        #expect(model.isSetKnown)
    }

    @Test func keepsTheSetCodeWhenTheLookupFails() async {
        let api = FakeAPI()
        api.onGetSet = { _ in throw APIError(code: "set_not_found", status: 404) }
        let model = CardOffersViewModel(card: testCard(), api: api)

        await model.loadSetName()

        #expect(model.setName == "FIN")
        #expect(!model.isSetKnown)
    }

    @Test func startsATradeWithTheOwnerAndPutsTheCardOnTheirSide() async {
        let api = FakeAPI()
        var added: Operations.add_trade_card.Input?
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t1")))) }
        api.onAddTradeCard = { input in
            added = input
            return .noContent(.init())
        }
        let model = CardOffersViewModel(card: testCard(), api: api)

        let route = await model.startTrade(with: offer("mizzix_42"))

        #expect(route == TradeDetailRoute(id: "t1", partnerUsername: "mizzix_42"))
        #expect(model.startError == nil)
        #expect(api.calls == ["create_trade", "add_trade_card"])
        #expect(added?.path.trade_id == "t1")
        #expect(model.startingWith == nil)
    }

    @Test func reopensTheTradeAlreadyOpenedWithoutAddingTheCardAgain() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t1")))) }
        api.onAddTradeCard = { _ in .noContent(.init()) }
        let model = CardOffersViewModel(card: testCard(), api: api)
        _ = await model.startTrade(with: offer("mizzix_42"))

        let route = await model.startTrade(with: offer("mizzix_42"))

        #expect(route?.id == "t1")
        #expect(api.calls == ["create_trade", "add_trade_card"])
    }

    @Test func showsTheTranslatedReasonWhenTheTradeCannotBeOpened() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in throw APIError(code: "self_trade", status: 400) }
        let model = CardOffersViewModel(card: testCard(), api: api)

        let route = await model.startTrade(with: offer("me"))

        #expect(route == nil)
        #expect(model.startError == "Tu ne peux pas ouvrir un échange avec toi-même.")
        #expect(api.calls == ["create_trade"])
    }

    @Test func showsTheTranslatedReasonWhenTheCardIsAlreadyReserved() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t1")))) }
        api.onAddTradeCard = { _ in throw APIError(code: "card_already_reserved", status: 409) }
        let model = CardOffersViewModel(card: testCard(), api: api)

        let route = await model.startTrade(with: offer("mizzix_42"))

        #expect(route == nil)
        #expect(model.startError == "Cette carte est déjà réservée dans un autre échange.")
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachableWhileStartingATrade() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = CardOffersViewModel(card: testCard(), api: api)

        _ = await model.startTrade(with: offer("mizzix_42"))

        #expect(model.startError == APIErrorMessages.network)
    }
}
