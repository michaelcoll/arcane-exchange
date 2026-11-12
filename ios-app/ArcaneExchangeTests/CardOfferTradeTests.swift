import APIClient
import Testing

@testable import ArcaneCore
@testable import ArcaneExchange

@MainActor
struct CardOfferTradeTests {
    @Test func opensATradeWithTheOwnerAndPutsTheCardOnTheirSide() async {
        let api = FakeAPI()
        var added: Operations.add_trade_card.Input?
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t1")))) }
        api.onAddTradeCard = { input in
            added = input
            return .noContent(.init())
        }
        let trade = CardOfferTrade(card: testCard(), api: api)

        let route = await trade.start(with: offer("mizzix_42"))

        #expect(route == TradeDetailRoute(id: "t1", partnerUsername: "mizzix_42"))
        #expect(trade.startError == nil)
        #expect(trade.startingWith == nil)
        #expect(api.calls == ["create_trade", "add_trade_card"])
        #expect(added?.path.trade_id == "t1")
        #expect(added?.body == .json(.init(
            collector_number: testCard().collector_number,
            foil: testCard().foil,
            language_code: testCard().language_code,
            owner_username: "mizzix_42",
            set_code: testCard().set_code
        )))
    }

    @Test func asksTheServerAgainOnEverySubsequentOpening() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t1")))) }
        api.onAddTradeCard = { _ in .noContent(.init()) }
        let trade = CardOfferTrade(card: testCard(), api: api)
        _ = await trade.start(with: offer("mizzix_42"))

        let route = await trade.start(with: offer("mizzix_42"))

        #expect(route?.id == "t1")
        #expect(api.calls == ["create_trade", "add_trade_card", "create_trade", "add_trade_card"])
    }

    @Test func refusesASecondOpeningWhileOneIsInFlightAndSendsNoExtraRequest() async {
        let api = FakeAPI()
        let gate = CreateTradeGate()
        api.onCreateTrade = { _ in await gate.next() }
        api.onAddTradeCard = { _ in .noContent(.init()) }
        let trade = CardOfferTrade(card: testCard(), api: api)
        let first = Task { await trade.start(with: offer("mizzix_42")) }
        await gate.waitForRequest()

        let second = await trade.start(with: offer("other_owner"))
        gate.respond(with: .created(.init(body: .json(.init(id: "t1")))))
        let firstRoute = await first.value

        #expect(second == nil)
        #expect(firstRoute?.id == "t1")
        #expect(api.calls == ["create_trade", "add_trade_card"])
    }

    @Test func showsTheTranslatedReasonWhenTheTradeCannotBeOpened() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in throw APIError(code: "self_trade", status: 400) }
        let trade = CardOfferTrade(card: testCard(), api: api)

        let route = await trade.start(with: offer("me"))

        #expect(route == nil)
        #expect(trade.startError == "Tu ne peux pas ouvrir un échange avec toi-même.")
        #expect(trade.startingWith == nil)
        #expect(api.calls == ["create_trade"])
    }

    @Test func showsTheTranslatedReasonWhenTheCardCannotBeAdded() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t1")))) }
        api.onAddTradeCard = { _ in throw APIError(code: "card_already_reserved", status: 409) }
        let trade = CardOfferTrade(card: testCard(), api: api)

        let route = await trade.start(with: offer("mizzix_42"))

        #expect(route == nil)
        #expect(trade.startError == "Cette carte est déjà réservée dans un autre échange.")
        #expect(trade.startingWith == nil)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let trade = CardOfferTrade(card: testCard(), api: api)

        _ = await trade.start(with: offer("mizzix_42"))

        #expect(trade.startError == APIErrorMessages.network)
    }
}

/// Holds the `create_trade` response until the test releases it, to keep an opening in flight.
@MainActor
private final class CreateTradeGate {
    private var waiting: CheckedContinuation<Operations.create_trade.Output, Never>?

    func next() async -> Operations.create_trade.Output {
        await withCheckedContinuation { waiting = $0 }
    }

    func respond(with output: Operations.create_trade.Output) {
        waiting?.resume(returning: output)
    }

    func waitForRequest() async {
        while waiting == nil {
            await Task.yield()
        }
    }
}
