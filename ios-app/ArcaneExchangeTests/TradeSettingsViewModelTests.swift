import APIClient
import Testing

@testable import ArcaneExchange

private func rarity(_ code: String, isOpen: Bool = true, copies: Int64 = 10, proposed: Int64 = 10) -> RarityFilter {
    RarityFilter(copies: copies, is_open: isOpen, kept_copies: 0, proposed: proposed, rarity: code)
}

/// A fake already answering the four requests of `load()`.
private func loadableAPI(rarities: [RarityFilter] = [rarity("C")]) -> FakeAPI {
    let api = FakeAPI()
    api.onGetTradeVisibility = { _ in .ok(.init(body: .json(.init(visibility: .trade)))) }
    api.onGetCollectionStats = { _ in
        .ok(.init(body: .json(.init(
            binders: [.init(card_count: 3, name: "Binder 10"), .init(card_count: 5, name: "Binder 2")],
            sets: [],
            total_cards: 8,
            unique_cards: 8
        ))))
    }
    api.onGetTradeBinders = { _ in .ok(.init(body: .json(.init(binders: ["Binder 2"])))) }
    api.onGetTradeRarityFilters = { _ in .ok(.init(body: .json(.init(rarities: rarities)))) }
    return api
}

@MainActor
struct TradeSettingsViewModelTests {
    @Test func loadsEverySetting() async {
        let api = loadableAPI()
        let model = TradeSettingsViewModel(api: api)

        await model.load()

        #expect(model.visibility == .trade)
        #expect(model.binders.map(\.name) == ["Binder 2", "Binder 10"])
        #expect(model.selectedBinders == ["Binder 2"])
        #expect(model.rarities.map(\.rarity) == ["C"])
        #expect(model.loadError == nil)
        #expect(!model.isLoading)
        #expect(Set(api.calls) == [
            "get_trade_visibility", "get_collection_stats", "get_trade_binders", "get_trade_rarity_filters",
        ])
    }

    @Test func showsTheTranslatedMessageOfAFailedLoad() async {
        let api = loadableAPI()
        api.onGetTradeBinders = { _ in throw APIError(code: "internal", status: 500) }
        let model = TradeSettingsViewModel(api: api)

        await model.load()

        #expect(model.loadError == "Erreur technique, réessaie plus tard.")
        #expect(!model.isLoading)
    }

    @Test func showsTheNetworkMessageWhenTheServerIsUnreachableOnLoad() async {
        let api = loadableAPI()
        api.onGetTradeVisibility = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = TradeSettingsViewModel(api: api)

        await model.load()

        #expect(model.loadError == APIErrorMessages.network)
    }

    @Test func savesAVisibilityChange() async {
        let api = loadableAPI()
        nonisolated(unsafe) var sent: CollectionVisibility?
        api.onSetTradeVisibility = { input in
            if case let .json(body) = input.body { sent = body.visibility }
            return .noContent(.init())
        }
        let model = TradeSettingsViewModel(api: api)

        await model.setVisibility(._public)

        #expect(model.visibility == ._public)
        #expect(sent == ._public)
        #expect(model.writeError == nil)
        #expect(!model.isSavingVisibility)
    }

    @Test func rollsBackARefusedVisibilityWithTheTranslatedMessage() async {
        let api = loadableAPI()
        api.onSetTradeVisibility = { _ in throw APIError(code: "unauthorized", status: 401) }
        let model = TradeSettingsViewModel(api: api)
        await model.load()

        await model.setVisibility(._public)

        #expect(model.visibility == .trade)
        #expect(model.writeError == "Ta session a expiré, reconnecte-toi.")
    }

    @Test func rollsBackAVisibilityChangeLostToTheNetwork() async {
        let api = loadableAPI()
        api.onSetTradeVisibility = { _ in throw APIError(code: nil, status: nil, isNetworkFailure: true) }
        let model = TradeSettingsViewModel(api: api)
        await model.load()

        await model.setVisibility(._private)

        #expect(model.visibility == .trade)
        #expect(model.writeError == APIErrorMessages.network)
    }

    @Test func selectsABinderThenRefreshesTheRarities() async {
        let api = loadableAPI()
        api.onAddTradeBinder = { _ in .noContent(.init()) }
        let model = TradeSettingsViewModel(api: api)
        await model.load()

        await model.toggleBinder("Binder 10")

        #expect(model.selectedBinders == ["Binder 2", "Binder 10"])
        #expect(model.writeError == nil)
        #expect(api.calls.suffix(2) == ["add_trade_binder", "get_trade_rarity_filters"])
    }

    @Test func rollsBackARefusedBinderToggle() async {
        let api = loadableAPI()
        api.onRemoveTradeBinder = { _ in throw APIError(code: "binder_not_found", status: 404) }
        let model = TradeSettingsViewModel(api: api)
        await model.load()

        await model.toggleBinder("Binder 2")

        #expect(model.selectedBinders == ["Binder 2"])
        #expect(model.writeError == "Ce binder n'existe plus dans ta collection.")
        // Only the load fetched the rarities: a refused write has nothing to refresh.
        #expect(api.calls.filter { $0 == "get_trade_rarity_filters" }.count == 1)
    }

    @Test func savesARarityRuleThenRefetchesIt() async {
        let api = loadableAPI()
        nonisolated(unsafe) var sent: Components.Schemas.SetRarityFilterRequest?
        api.onSetTradeRarityFilter = { input in
            if case let .json(body) = input.body { sent = body }
            return .noContent(.init())
        }
        let model = TradeSettingsViewModel(api: api)

        await model.setRarity("C", isOpen: false, keptCopies: 9)

        #expect(sent?.rarity == "C")
        #expect(sent?.is_open == false)
        #expect(sent?.kept_copies == TradeSettings.maxKeptCopies)
        #expect(model.writeError == nil)
        #expect(api.calls == ["set_trade_rarity_filter", "get_trade_rarity_filters"])
    }

    @Test func showsTheMessageOfARefusedRarityRuleAndStillRefetches() async {
        let api = loadableAPI()
        api.onSetTradeRarityFilter = { _ in throw APIError(code: "internal", status: 500) }
        let model = TradeSettingsViewModel(api: api)

        await model.setRarity("C", isOpen: true, keptCopies: 1)

        #expect(model.writeError == "Erreur technique, réessaie plus tard.")
        #expect(api.calls == ["set_trade_rarity_filter", "get_trade_rarity_filters"])
    }
}
