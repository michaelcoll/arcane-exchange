import APIClient
import Testing

@testable import ArcaneExchange

private func detail(status: TradeStatus = .pending) -> TradeDetail {
    TradeDetail(
        id: "t1",
        me: .init(accepted: false, confirmed: false),
        my_cards: [],
        partner: .init(accepted: false, confirmed: false),
        partner_cards: [],
        partner_username: "mizzix_42",
        status: status.rawValue
    )
}

private func loaded(_ status: TradeStatus = .pending) -> Operations.get_trade.Output {
    .ok(.init(body: .json(detail(status: status))))
}

private let partnerCard = TradeCard(
    collector_number: "243",
    foil: false,
    language_code: "fr",
    name: "The Soul Stone",
    price_guide: nil,
    quantity: 1,
    scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
    set_code: "FIN"
)

@MainActor
struct TradeDetailViewModelTests {
    @Test func loadsTheTrade() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded(.oneAccepted) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.load()

        #expect(model.status == .oneAccepted)
        #expect(model.partnerUsername == "mizzix_42")
        #expect(model.loadError == nil)
    }

    @Test func showsTheTranslatedMessageWhenTheTradeIsGone() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in throw APIError(code: "trade_not_found", status: 404) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.load()

        #expect(model.loadError == "Cet échange n'existe pas.")
        #expect(model.trade == nil)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.load()

        #expect(model.loadError == APIErrorMessages.network)
    }

    @Test func anActionIsFollowedByAReload() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded() }
        api.onAcceptTrade = { _ in .noContent(.init()) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.accept()

        #expect(api.calls == ["accept_trade", "get_trade"])
        #expect(model.actionError == nil)
        #expect(!model.isBusy)
    }

    @Test func anAbandonIsFollowedByAReload() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded() }
        api.onAbandonTrade = { _ in .noContent(.init()) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.abandon()

        #expect(api.calls == ["abandon_trade", "get_trade"])
        #expect(model.actionError == nil)
    }

    @Test func aConfirmationIsFollowedByAReload() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded(.fullyAccepted) }
        api.onConfirmTrade = { _ in .noContent(.init()) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.confirmExchange()

        #expect(api.calls == ["confirm_trade", "get_trade"])
        #expect(model.actionError == nil)
    }

    @Test func aRatingIsFollowedByAReload() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded(.completed) }
        api.onRateTrade = { _ in .noContent(.init()) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.rate(5)

        #expect(api.calls == ["rate_trade", "get_trade"])
        #expect(model.actionError == nil)
    }

    @Test func removingTheCardOfThePartnerIsFollowedByAReload() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded() }
        nonisolated(unsafe) var removed: Operations.remove_trade_card.Input?
        api.onRemoveTradeCard = { input in
            removed = input
            return .noContent(.init())
        }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)
        await model.load()

        await model.removePartnerCard(partnerCard)

        #expect(api.calls == ["get_trade", "remove_trade_card", "get_trade"])
        #expect(model.actionError == nil)
        if case let .json(body)? = removed?.body {
            #expect(body.owner_username == "mizzix_42")
            #expect(body.set_code == "FIN")
        } else {
            Issue.record("expected a JSON body")
        }
    }

    @Test func aRefusedAcceptShowsTheReasonOfTheCode() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded() }
        api.onAcceptTrade = { _ in throw APIError(code: "card_already_reserved", status: 409) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.accept()

        #expect(model.actionError == "Cette carte est déjà réservée dans un autre échange.")
        #expect(api.calls == ["accept_trade", "get_trade"])
    }

    @Test func aRefusedActionOnAnUnreachableServerShowsTheNetworkMessage() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded() }
        api.onAbandonTrade = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.abandon()

        #expect(model.actionError == APIErrorMessages.network)
    }

    @Test func aRefusedConfirmationWithAnUnknownCodeShowsTheGenericMessage() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded() }
        api.onConfirmTrade = { _ in throw APIError(code: "new_backend_code", status: 409) }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.confirmExchange()

        #expect(model.actionError == "Une erreur est survenue.")
    }

    @Test func ratesWithTheGivenNumberOfStars() async {
        let api = FakeAPI()
        api.onGetTrade = { _ in loaded(.completed) }
        nonisolated(unsafe) var sent: Int32?
        api.onRateTrade = { input in
            if case let .json(body) = input.body {
                sent = body.rating
            }
            return .noContent(.init())
        }
        let model = TradeDetailViewModel(tradeID: "t1", api: api)

        await model.rate(4)

        #expect(sent == 4)
        #expect(model.actionError == nil)
    }
}
