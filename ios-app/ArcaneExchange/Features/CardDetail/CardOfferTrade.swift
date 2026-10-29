import APIClient
import Foundation

/// Opens (or reuses) the trade with a card's owner and puts the card on their side.
///
/// Two calls, exactly like the web client's `startTrade`: `POST /trades` is idempotent per
/// partner — it hands back the existing active trade — so this doubles as "add this card to
/// the trade I already have with them".
///
/// Shared by the card screen's short owners list and the full "Possesseurs" screen, which
/// both offer the same "Échanger" button on an offer row.
@MainActor
enum CardOfferTrade {
    /// Throws the `APIError` the API call module raised: read it with `APIError.read`.
    static func start(
        card: CollectionCard,
        with offer: CardOffer,
        api: any APIProtocol
    ) async throws -> TradeDetailRoute {
        let tradeID = try await api.create_trade(
            body: .json(.init(respondent_username: offer.owner_username))
        ).created.body.json.id
        _ = try await api.add_trade_card(
            path: .init(trade_id: tradeID),
            body: .json(.init(
                collector_number: card.collector_number,
                foil: card.foil,
                language_code: card.language_code,
                owner_username: offer.owner_username,
                quantity: 1,
                set_code: card.set_code
            ))
        ).noContent
        HomeWidgetReload.reload(.trades)
        return TradeDetailRoute(id: tradeID, partnerUsername: offer.owner_username)
    }
}

/// Resolves a set code to its human-readable name.
///
/// Both card screens show the set line under the card name and both fall back to the raw
/// code when the lookup fails (same as `frontend/`'s `DetailModal.vue`).
enum SetName {
    /// The set's name, or `nil` when it cannot be resolved — keep showing the code then.
    static func resolve(_ setCode: String, api: any APIProtocol) async -> String? {
        try? await api.get_set(path: .init(set_code: setCode)).ok.body.json.name
    }
}
