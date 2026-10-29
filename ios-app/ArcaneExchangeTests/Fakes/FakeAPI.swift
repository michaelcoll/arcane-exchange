import APIClient
import Foundation

/// An in-memory `APIProtocol` for view model tests (ADR 0020): no network, no middleware.
///
/// Configure an operation by assigning its `on...` closure, which returns the generated
/// `Output` or throws an `APIError` as the live client would:
///
///     let api = FakeAPI()
///     api.onListTrades = { _ in .ok(.init(body: .json(page))) }
///     api.onAcceptTrade = { _ in throw APIError(code: "card_already_reserved", status: 409) }
///
/// An operation left unconfigured throws `FakeAPI.NotStubbed`, so a test fails loudly on a call
/// it did not expect. `calls` lists the operations received, in order, by generated name
/// (`"list_trades"`); to check an input, capture it in the closure.
///
/// Generated from the operations of `APIProtocol`: add a line per new operation.
final class FakeAPI: APIProtocol, @unchecked Sendable {
    struct NotStubbed: Error, CustomStringConvertible {
        let operation: String
        var description: String { "FakeAPI: \(operation) was called but never configured" }
    }

    private let lock = NSLock()
    private var recorded: [String] = []

    /// The operations received so far, by generated name, in call order.
    var calls: [String] {
        lock.withLock { recorded }
    }

    private func record(_ operation: String) {
        lock.withLock { recorded.append(operation) }
    }

    var onAutocompleteUser: (Operations.autocomplete_user.Input) async throws -> Operations.autocomplete_user.Output = { _ in
        throw NotStubbed(operation: "autocomplete_user")
    }

    func autocomplete_user(_ input: Operations.autocomplete_user.Input) async throws -> Operations.autocomplete_user.Output {
        record("autocomplete_user")
        return try await onAutocompleteUser(input)
    }

    var onGetCardInfo: (Operations.get_card_info.Input) async throws -> Operations.get_card_info.Output = { _ in
        throw NotStubbed(operation: "get_card_info")
    }

    func get_card_info(_ input: Operations.get_card_info.Input) async throws -> Operations.get_card_info.Output {
        record("get_card_info")
        return try await onGetCardInfo(input)
    }

    var onGetCardOffers: (Operations.get_card_offers.Input) async throws -> Operations.get_card_offers.Output = { _ in
        throw NotStubbed(operation: "get_card_offers")
    }

    func get_card_offers(_ input: Operations.get_card_offers.Input) async throws -> Operations.get_card_offers.Output {
        record("get_card_offers")
        return try await onGetCardOffers(input)
    }

    var onGetCardPriceHistory: (Operations.get_card_price_history.Input) async throws -> Operations.get_card_price_history.Output = { _ in
        throw NotStubbed(operation: "get_card_price_history")
    }

    func get_card_price_history(_ input: Operations.get_card_price_history.Input) async throws -> Operations.get_card_price_history.Output {
        record("get_card_price_history")
        return try await onGetCardPriceHistory(input)
    }

    var onGetCollection: (Operations.get_collection.Input) async throws -> Operations.get_collection.Output = { _ in
        throw NotStubbed(operation: "get_collection")
    }

    func get_collection(_ input: Operations.get_collection.Input) async throws -> Operations.get_collection.Output {
        record("get_collection")
        return try await onGetCollection(input)
    }

    var onListCardImports: (Operations.list_card_imports.Input) async throws -> Operations.list_card_imports.Output = { _ in
        throw NotStubbed(operation: "list_card_imports")
    }

    func list_card_imports(_ input: Operations.list_card_imports.Input) async throws -> Operations.list_card_imports.Output {
        record("list_card_imports")
        return try await onListCardImports(input)
    }

    var onImportCards: (Operations.import_cards.Input) async throws -> Operations.import_cards.Output = { _ in
        throw NotStubbed(operation: "import_cards")
    }

    func import_cards(_ input: Operations.import_cards.Input) async throws -> Operations.import_cards.Output {
        record("import_cards")
        return try await onImportCards(input)
    }

    var onGetCardImport: (Operations.get_card_import.Input) async throws -> Operations.get_card_import.Output = { _ in
        throw NotStubbed(operation: "get_card_import")
    }

    func get_card_import(_ input: Operations.get_card_import.Input) async throws -> Operations.get_card_import.Output {
        record("get_card_import")
        return try await onGetCardImport(input)
    }

    var onGetCollectionPriceHistory: (Operations.get_collection_price_history.Input) async throws -> Operations.get_collection_price_history.Output = { _ in
        throw NotStubbed(operation: "get_collection_price_history")
    }

    func get_collection_price_history(_ input: Operations.get_collection_price_history.Input) async throws -> Operations.get_collection_price_history.Output {
        record("get_collection_price_history")
        return try await onGetCollectionPriceHistory(input)
    }

    var onGetCollectionStats: (Operations.get_collection_stats.Input) async throws -> Operations.get_collection_stats.Output = { _ in
        throw NotStubbed(operation: "get_collection_stats")
    }

    func get_collection_stats(_ input: Operations.get_collection_stats.Input) async throws -> Operations.get_collection_stats.Output {
        record("get_collection_stats")
        return try await onGetCollectionStats(input)
    }

    var onGetTradeBinders: (Operations.get_trade_binders.Input) async throws -> Operations.get_trade_binders.Output = { _ in
        throw NotStubbed(operation: "get_trade_binders")
    }

    func get_trade_binders(_ input: Operations.get_trade_binders.Input) async throws -> Operations.get_trade_binders.Output {
        record("get_trade_binders")
        return try await onGetTradeBinders(input)
    }

    var onAddTradeBinder: (Operations.add_trade_binder.Input) async throws -> Operations.add_trade_binder.Output = { _ in
        throw NotStubbed(operation: "add_trade_binder")
    }

    func add_trade_binder(_ input: Operations.add_trade_binder.Input) async throws -> Operations.add_trade_binder.Output {
        record("add_trade_binder")
        return try await onAddTradeBinder(input)
    }

    var onRemoveTradeBinder: (Operations.remove_trade_binder.Input) async throws -> Operations.remove_trade_binder.Output = { _ in
        throw NotStubbed(operation: "remove_trade_binder")
    }

    func remove_trade_binder(_ input: Operations.remove_trade_binder.Input) async throws -> Operations.remove_trade_binder.Output {
        record("remove_trade_binder")
        return try await onRemoveTradeBinder(input)
    }

    var onGetTradeRarityFilters: (Operations.get_trade_rarity_filters.Input) async throws -> Operations.get_trade_rarity_filters.Output = { _ in
        throw NotStubbed(operation: "get_trade_rarity_filters")
    }

    func get_trade_rarity_filters(_ input: Operations.get_trade_rarity_filters.Input) async throws -> Operations.get_trade_rarity_filters.Output {
        record("get_trade_rarity_filters")
        return try await onGetTradeRarityFilters(input)
    }

    var onSetTradeRarityFilter: (Operations.set_trade_rarity_filter.Input) async throws -> Operations.set_trade_rarity_filter.Output = { _ in
        throw NotStubbed(operation: "set_trade_rarity_filter")
    }

    func set_trade_rarity_filter(_ input: Operations.set_trade_rarity_filter.Input) async throws -> Operations.set_trade_rarity_filter.Output {
        record("set_trade_rarity_filter")
        return try await onSetTradeRarityFilter(input)
    }

    var onGetTradeVisibility: (Operations.get_trade_visibility.Input) async throws -> Operations.get_trade_visibility.Output = { _ in
        throw NotStubbed(operation: "get_trade_visibility")
    }

    func get_trade_visibility(_ input: Operations.get_trade_visibility.Input) async throws -> Operations.get_trade_visibility.Output {
        record("get_trade_visibility")
        return try await onGetTradeVisibility(input)
    }

    var onSetTradeVisibility: (Operations.set_trade_visibility.Input) async throws -> Operations.set_trade_visibility.Output = { _ in
        throw NotStubbed(operation: "set_trade_visibility")
    }

    func set_trade_visibility(_ input: Operations.set_trade_visibility.Input) async throws -> Operations.set_trade_visibility.Output {
        record("set_trade_visibility")
        return try await onSetTradeVisibility(input)
    }

    var onTriggerPriceUpdate: (Operations.trigger_price_update.Input) async throws -> Operations.trigger_price_update.Output = { _ in
        throw NotStubbed(operation: "trigger_price_update")
    }

    func trigger_price_update(_ input: Operations.trigger_price_update.Input) async throws -> Operations.trigger_price_update.Output {
        record("trigger_price_update")
        return try await onTriggerPriceUpdate(input)
    }

    var onUpdateCardmarketIds: (Operations.update_cardmarket_ids.Input) async throws -> Operations.update_cardmarket_ids.Output = { _ in
        throw NotStubbed(operation: "update_cardmarket_ids")
    }

    func update_cardmarket_ids(_ input: Operations.update_cardmarket_ids.Input) async throws -> Operations.update_cardmarket_ids.Output {
        record("update_cardmarket_ids")
        return try await onUpdateCardmarketIds(input)
    }

    var onSearchCards: (Operations.search_cards.Input) async throws -> Operations.search_cards.Output = { _ in
        throw NotStubbed(operation: "search_cards")
    }

    func search_cards(_ input: Operations.search_cards.Input) async throws -> Operations.search_cards.Output {
        record("search_cards")
        return try await onSearchCards(input)
    }

    var onSearchCardSets: (Operations.search_card_sets.Input) async throws -> Operations.search_card_sets.Output = { _ in
        throw NotStubbed(operation: "search_card_sets")
    }

    func search_card_sets(_ input: Operations.search_card_sets.Input) async throws -> Operations.search_card_sets.Output {
        record("search_card_sets")
        return try await onSearchCardSets(input)
    }

    var onListSets: (Operations.list_sets.Input) async throws -> Operations.list_sets.Output = { _ in
        throw NotStubbed(operation: "list_sets")
    }

    func list_sets(_ input: Operations.list_sets.Input) async throws -> Operations.list_sets.Output {
        record("list_sets")
        return try await onListSets(input)
    }

    var onGetSet: (Operations.get_set.Input) async throws -> Operations.get_set.Output = { _ in
        throw NotStubbed(operation: "get_set")
    }

    func get_set(_ input: Operations.get_set.Input) async throws -> Operations.get_set.Output {
        record("get_set")
        return try await onGetSet(input)
    }

    var onGetShowcase: (Operations.get_showcase.Input) async throws -> Operations.get_showcase.Output = { _ in
        throw NotStubbed(operation: "get_showcase")
    }

    func get_showcase(_ input: Operations.get_showcase.Input) async throws -> Operations.get_showcase.Output {
        record("get_showcase")
        return try await onGetShowcase(input)
    }

    var onGetStats: (Operations.get_stats.Input) async throws -> Operations.get_stats.Output = { _ in
        throw NotStubbed(operation: "get_stats")
    }

    func get_stats(_ input: Operations.get_stats.Input) async throws -> Operations.get_stats.Output {
        record("get_stats")
        return try await onGetStats(input)
    }

    var onListTrades: (Operations.list_trades.Input) async throws -> Operations.list_trades.Output = { _ in
        throw NotStubbed(operation: "list_trades")
    }

    func list_trades(_ input: Operations.list_trades.Input) async throws -> Operations.list_trades.Output {
        record("list_trades")
        return try await onListTrades(input)
    }

    var onCreateTrade: (Operations.create_trade.Input) async throws -> Operations.create_trade.Output = { _ in
        throw NotStubbed(operation: "create_trade")
    }

    func create_trade(_ input: Operations.create_trade.Input) async throws -> Operations.create_trade.Output {
        record("create_trade")
        return try await onCreateTrade(input)
    }

    var onGetTrade: (Operations.get_trade.Input) async throws -> Operations.get_trade.Output = { _ in
        throw NotStubbed(operation: "get_trade")
    }

    func get_trade(_ input: Operations.get_trade.Input) async throws -> Operations.get_trade.Output {
        record("get_trade")
        return try await onGetTrade(input)
    }

    var onAbandonTrade: (Operations.abandon_trade.Input) async throws -> Operations.abandon_trade.Output = { _ in
        throw NotStubbed(operation: "abandon_trade")
    }

    func abandon_trade(_ input: Operations.abandon_trade.Input) async throws -> Operations.abandon_trade.Output {
        record("abandon_trade")
        return try await onAbandonTrade(input)
    }

    var onAcceptTrade: (Operations.accept_trade.Input) async throws -> Operations.accept_trade.Output = { _ in
        throw NotStubbed(operation: "accept_trade")
    }

    func accept_trade(_ input: Operations.accept_trade.Input) async throws -> Operations.accept_trade.Output {
        record("accept_trade")
        return try await onAcceptTrade(input)
    }

    var onAddTradeCard: (Operations.add_trade_card.Input) async throws -> Operations.add_trade_card.Output = { _ in
        throw NotStubbed(operation: "add_trade_card")
    }

    func add_trade_card(_ input: Operations.add_trade_card.Input) async throws -> Operations.add_trade_card.Output {
        record("add_trade_card")
        return try await onAddTradeCard(input)
    }

    var onRemoveTradeCard: (Operations.remove_trade_card.Input) async throws -> Operations.remove_trade_card.Output = { _ in
        throw NotStubbed(operation: "remove_trade_card")
    }

    func remove_trade_card(_ input: Operations.remove_trade_card.Input) async throws -> Operations.remove_trade_card.Output {
        record("remove_trade_card")
        return try await onRemoveTradeCard(input)
    }

    var onConfirmTrade: (Operations.confirm_trade.Input) async throws -> Operations.confirm_trade.Output = { _ in
        throw NotStubbed(operation: "confirm_trade")
    }

    func confirm_trade(_ input: Operations.confirm_trade.Input) async throws -> Operations.confirm_trade.Output {
        record("confirm_trade")
        return try await onConfirmTrade(input)
    }

    var onRateTrade: (Operations.rate_trade.Input) async throws -> Operations.rate_trade.Output = { _ in
        throw NotStubbed(operation: "rate_trade")
    }

    func rate_trade(_ input: Operations.rate_trade.Input) async throws -> Operations.rate_trade.Output {
        record("rate_trade")
        return try await onRateTrade(input)
    }

    var onRegister: (Operations.register.Input) async throws -> Operations.register.Output = { _ in
        throw NotStubbed(operation: "register")
    }

    func register(_ input: Operations.register.Input) async throws -> Operations.register.Output {
        record("register")
        return try await onRegister(input)
    }

    var onGetUserProfile: (Operations.get_user_profile.Input) async throws -> Operations.get_user_profile.Output = { _ in
        throw NotStubbed(operation: "get_user_profile")
    }

    func get_user_profile(_ input: Operations.get_user_profile.Input) async throws -> Operations.get_user_profile.Output {
        record("get_user_profile")
        return try await onGetUserProfile(input)
    }
}
