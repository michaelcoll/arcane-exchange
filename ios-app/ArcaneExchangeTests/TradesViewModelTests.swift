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

        #expect(model.paginator.items.map(\.id) == ["1", "2"])
        #expect(model.ongoing.map(\.id) == ["1"])
        #expect(model.past.map(\.id) == ["2"])
        #expect(model.paginator.state == .idle)
        #expect(api.calls == ["list_trades"])
    }

    @Test func loadsAnEmptyFeed() async {
        let api = FakeAPI()
        api.onListTrades = { _ in page([], total: 0) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.paginator.items.isEmpty)
        #expect(model.paginator.state == .idle)
    }

    @Test func loadsTheNextPageWhenTheLastRowOfTheDisplayedSegmentAppears() async {
        let api = FakeAPI()
        // The last four loaded trades belong to the other segment.
        let first = (1 ... 16).map { summary("\($0)") } + (17 ... 20).map { summary("\($0)", status: .closed) }
        let second = [summary("21")]
        api.onListTrades = { input in
            input.query.page == 0 ? page(first, total: 21) : page(second, total: 21)
        }
        let model = TradesViewModel(api: api)
        await model.reload()

        await model.loadMore(after: first[15])

        #expect(model.paginator.items.count == 21)
        #expect(api.calls == ["list_trades", "list_trades"])
    }

    @Test func doesNotLoadMoreForARowFarFromTheEndOfTheSegment() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { summary("\($0)") }
        api.onListTrades = { _ in page(first, total: 40) }
        let model = TradesViewModel(api: api)
        await model.reload()

        await model.loadMore(after: first[0])

        #expect(api.calls == ["list_trades"])
    }

    @Test func fillsAnEmptySegmentUntilItHasARow() async {
        let api = FakeAPI()
        let ongoing = { (range: ClosedRange<Int>) in range.map { summary("\($0)") } }
        api.onListTrades = { input in
            switch input.query.page {
            case 0: page(ongoing(1 ... 20), total: 61)
            case 1: page(ongoing(21 ... 40), total: 61)
            default: page(ongoing(41 ... 59) + [summary("60", status: .closed)], total: 61)
            }
        }
        let model = TradesViewModel(api: api)
        model.segment = .past

        await model.reload()

        #expect(model.visibleTrades.map(\.id) == ["60"])
        #expect(api.calls == ["list_trades", "list_trades", "list_trades"])
    }

    @Test func stopsFillingAnEmptySegmentAtTheTotal() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { summary("\($0)") }
        let second = (21 ... 25).map { summary("\($0)") }
        api.onListTrades = { input in
            input.query.page == 0 ? page(first, total: 25) : page(second, total: 25)
        }
        let model = TradesViewModel(api: api)
        model.segment = .past

        await model.reload()

        #expect(model.visibleTrades.isEmpty)
        #expect(api.calls == ["list_trades", "list_trades"])
    }

    @Test func fillsTheSegmentItSwitchesToWithoutReloadingFromPageZero() async {
        let api = FakeAPI()
        let first = (1 ... 20).map { summary("\($0)") }
        let second = [summary("21", status: .closed)]
        api.onListTrades = { input in
            input.query.page == 0 ? page(first, total: 21) : page(second, total: 21)
        }
        let model = TradesViewModel(api: api)
        await model.reload()

        model.segment = .past
        // The fill starts on its own, in a task of the model.
        while model.visibleTrades.isEmpty {
            await Task.yield()
        }

        #expect(model.visibleTrades.map(\.id) == ["21"])
        #expect(model.paginator.items.count == 21)
        #expect(api.calls == ["list_trades", "list_trades"])
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = FakeAPI()
        api.onListTrades = { _ in throw APIError(code: "internal", status: 500) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.paginator.state == .failed("Erreur technique, réessaie plus tard."))
        #expect(model.paginator.items.isEmpty)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachable() async {
        let api = FakeAPI()
        api.onListTrades = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.paginator.state == .failed(APIErrorMessages.network))
    }

    @Test func showsTheGenericMessageForAnUnknownCode() async {
        let api = FakeAPI()
        api.onListTrades = { _ in throw APIError(code: "new_backend_code", status: 409) }
        let model = TradesViewModel(api: api)

        await model.reload()

        #expect(model.paginator.state == .failed("Une erreur est survenue."))
    }
}
