import APIClient
import Testing

@testable import ArcaneExchange

private func summary(_ id: String, status: TradeStatus = .pending) -> TradeSummary {
    TradeSummary(
        id: id,
        my_card_count: 1,
        partner_card_count: 1,
        partner_username: "mizzix_42",
        status: status.rawValue,
        updated_at: "2026-10-01T10:00:00Z"
    )
}

private func page(_ items: [TradeSummary], total: Int64) -> Operations.list_trades.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: total))))
}

@MainActor
struct TradesViewModelTests {
    @Test func loadsTheFirstPageAndSplitsTheSegments() async {
        let api = FakeAPI()
        api.onListTrades = { _ in page([summary("1"), summary("2", status: .closed)], total: 2) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.trades.map(\.id) == ["1", "2"])
        #expect(model.ongoing.map(\.id) == ["1"])
        #expect(model.past.map(\.id) == ["2"])
        #expect(model.loadError == nil)
        #expect(!model.isLoading)
        #expect(api.calls == ["list_trades"])
    }

    @Test func loadsAnEmptyFeed() async {
        let api = FakeAPI()
        api.onListTrades = { _ in page([], total: 0) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.trades.isEmpty)
        #expect(model.loadError == nil)
    }

    @Test func loadsTheNextPageWhenTheLastRowsAppear() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { summary("\($0)") }
        let second = [summary("21")]
        api.onListTrades = { input in
            input.query.page == 0 ? page(first, total: 21) : page(second, total: 21)
        }
        let model = TradesViewModel(api: api)
        await model.reload()
        #expect(model.hasMore)

        await model.loadMoreIfNeeded(displaying: first[19])

        #expect(model.trades.count == 21)
        #expect(!model.hasMore)
        #expect(api.calls == ["list_trades", "list_trades"])
    }

    @Test func doesNotLoadMoreForARowFarFromTheEnd() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { summary("\($0)") }
        api.onListTrades = { _ in page(first, total: 40) }
        let model = TradesViewModel(api: api)
        await model.reload()

        await model.loadMoreIfNeeded(displaying: first[0])

        #expect(api.calls == ["list_trades"])
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = FakeAPI()
        api.onListTrades = { _ in throw APIError(code: "internal", status: 500) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.loadError == "Erreur technique, réessaie plus tard.")
        #expect(model.trades.isEmpty)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onListTrades = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.loadError == APIErrorMessages.network)
    }

    @Test func showsTheGenericMessageForAnUnknownCode() async {
        let api = FakeAPI()
        api.onListTrades = { _ in throw APIError(code: "new_backend_code", status: 409) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.loadError == "Une erreur est survenue.")
    }
}
