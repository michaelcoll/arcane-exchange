import APIClient
import Testing

@testable import ArcaneExchange

@MainActor
struct CardDetailViewModelTests {
    @Test func loadsThePreviewAndTheTotalOfOffers() async {
        let api = FakeAPI()
        var query: Operations.get_card_offers.Input.Query?
        api.onGetCardOffers = { input in
            query = input.query
            return offersPage([offer("a"), offer("b"), offer("c")], total: 12, pageSize: 3)
        }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadOffers()

        #expect(model.offers == .loaded(preview: [offer("a"), offer("b"), offer("c")], total: 12))
        #expect(query?.page == 0)
        #expect(query?.page_size == 3)
    }

    @Test func failsTheOffersWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onGetCardOffers = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadOffers()

        #expect(model.offers == .failed)
    }

    @Test func showsNoOffersWhenTheCardIsUnknown() async {
        let api = FakeAPI()
        api.onGetCardOffers = { _ in throw APIError(code: "card_not_found", status: 404) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadOffers()

        #expect(model.offers == .loaded(preview: [], total: 0))
    }

    @Test func saysThereIsNotEnoughDataWhenTheCardIsUnknown() async {
        let api = FakeAPI()
        api.onGetCardPriceHistory = { _ in throw APIError(code: "card_not_found", status: 404) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadHistory()

        #expect(model.history == .notEnoughData)
    }

    @Test func loadsAChartWhenThereIsEnoughHistory() async {
        let api = FakeAPI()
        api.onGetCardPriceHistory = { _ in
            .ok(.init(body: .json([
                PriceHistoryEntry(avg: 850, date: "2026-08-01", low: 800, trend: 880),
                PriceHistoryEntry(avg: 870, date: "2026-08-02", low: 830, trend: 900),
            ])))
        }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadHistory()

        guard case let .ready(points) = model.history else {
            Issue.record("expected a chart, got \(model.history)")
            return
        }
        #expect(points.count == 2)
    }

    @Test func saysThereIsNotEnoughDataForASinglePoint() async {
        let api = FakeAPI()
        api.onGetCardPriceHistory = { _ in
            .ok(.init(body: .json([PriceHistoryEntry(avg: 850, date: "2026-08-01", low: 800, trend: 880)])))
        }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadHistory()

        #expect(model.history == .notEnoughData)
    }

    @Test func failsTheHistoryWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onGetCardPriceHistory = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        await model.loadHistory()

        #expect(model.history == .failed)
    }

    @Test func startsATradeFromAnOffer() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t9")))) }
        api.onAddTradeCard = { _ in .noContent(.init()) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        let route = await model.startTrade(with: offer("mizzix_42"))

        #expect(route == TradeDetailRoute(id: "t9", partnerUsername: "mizzix_42"))
        #expect(api.calls == ["create_trade", "add_trade_card"])
    }

    @Test func showsTheTranslatedReasonWhenTheCardIsGone() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in .created(.init(body: .json(.init(id: "t9")))) }
        api.onAddTradeCard = { _ in throw APIError(code: "card_not_found", status: 404) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        let route = await model.startTrade(with: offer("mizzix_42"))

        #expect(route == nil)
        #expect(model.startError == "Cette carte est introuvable ou n'est plus disponible.")
    }

    @Test func showsTheTranslatedReasonForATradeWithOneself() async {
        let api = FakeAPI()
        api.onCreateTrade = { _ in throw APIError(code: "self_trade", status: 400) }
        let model = CardDetailViewModel(card: testCard(), api: api)

        _ = await model.startTrade(with: offer("me"))

        #expect(model.startError == "Tu ne peux pas ouvrir un échange avec toi-même.")
    }
}
