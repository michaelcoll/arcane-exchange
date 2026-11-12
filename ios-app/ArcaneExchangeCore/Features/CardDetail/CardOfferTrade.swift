import APIClient
import Foundation

/// Opens the trade with a card's owner and makes sure the card is on their side.
///
/// Two calls, exactly like the web client: `POST /trades` is idempotent per partner (it
/// hands back the active trade) and `POST /trades/{id}/cards` is idempotent too, so every
/// tap asks the server and nothing is remembered here.
///
/// Shared by the card screen's short owners list and the full "Possesseurs" screen, which
/// both offer the same "Échanger" button on an offer row.
@MainActor
@Observable
package final class CardOfferTrade {
    /// The owner whose trade is being opened, so only that row shows a spinner.
    package private(set) var startingWith: String?
    /// Set when opening a trade is refused; the view shows it in an alert and clears it.
    package var startError: String?

    private let card: CollectionCard
    private let api: any APIProtocol

    package init(card: CollectionCard, api: any APIProtocol) {
        self.card = card
        self.api = api
    }

    /// Opens (or reuses) the trade with this offer's owner, this card on their side.
    /// Returns `nil` when another opening is in flight or the API refused (`startError` is set).
    package func start(with offer: CardOffer) async -> TradeDetailRoute? {
        guard startingWith == nil else { return nil }
        startingWith = offer.owner_username
        defer { startingWith = nil }
        let tradeID: String
        do {
            tradeID = try await api.create_trade(
                body: .json(.init(respondent_username: offer.owner_username))
            ).created.body.json.id
            _ = try await api.add_trade_card(
                path: .init(trade_id: tradeID),
                body: .json(.init(
                    collector_number: card.collector_number,
                    foil: card.foil,
                    language_code: card.language_code,
                    owner_username: offer.owner_username,
                    set_code: card.set_code
                ))
            ).noContent
        } catch {
            startError = APIError.read(error).message
            return nil
        }
        return TradeDetailRoute(id: tradeID, partnerUsername: offer.owner_username)
    }
}

/// Resolves a set code to its human-readable name.
///
/// Both card screens show the set line under the card name and both fall back to the raw
/// code when the lookup fails (same as `frontend/`'s `DetailModal.vue`).
package enum SetName {
    /// The set's name, or `nil` when it cannot be resolved — keep showing the code then.
    package static func resolve(_ setCode: String, api: any APIProtocol) async -> String? {
        try? await api.get_set(path: .init(set_code: setCode)).ok.body.json.name
    }
}
