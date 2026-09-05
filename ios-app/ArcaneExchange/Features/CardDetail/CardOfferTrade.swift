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
    /// `.failure` carries the message to show the player, already worded for them.
    static func start(card: CollectionCard, with offer: CardOffer) async -> Result<TradeDetailRoute, Refusal> {
        do {
            let tradeID = try await createTrade(with: offer.owner_username)
            try await addCard(card, to: tradeID, ownedBy: offer.owner_username)
            return .success(TradeDetailRoute(id: tradeID, partnerUsername: offer.owner_username))
        } catch let refusal as Refusal {
            return .failure(refusal)
        } catch {
            return .failure(Refusal(message: "Serveur injoignable. Réessaie une fois la connexion revenue."))
        }
    }

    /// Why the trade could not be started, worded for the player.
    struct Refusal: Error {
        let message: String
    }

    private static func createTrade(with owner: String) async throws -> String {
        let output = try await APIClientProvider.shared.create_trade(
            body: .json(.init(respondent_username: owner))
        )
        switch output {
        case let .created(response):
            return try response.body.json.id
        case .badRequest:
            throw Refusal(message: "Tu ne peux pas ouvrir un échange avec toi-même.")
        case .unauthorized:
            throw Refusal(message: "Session expirée. Reconnecte-toi.")
        case .notFound:
            throw Refusal(message: "Ce joueur n'existe plus.")
        case let .undocumented(statusCode, _):
            throw Refusal(message: "Le serveur a répondu \(statusCode).")
        }
    }

    private static func addCard(_ card: CollectionCard, to tradeID: String, ownedBy owner: String) async throws {
        let output = try await APIClientProvider.shared.add_trade_card(
            path: .init(trade_id: tradeID),
            body: .json(.init(
                collector_number: card.collector_number,
                foil: card.foil,
                language_code: card.language_code,
                owner_username: owner,
                quantity: 1,
                set_code: card.set_code
            ))
        )
        switch output {
        case .noContent:
            return
        case .badRequest:
            throw Refusal(message: "Requête invalide.")
        case .unauthorized:
            throw Refusal(message: "Session expirée. Reconnecte-toi.")
        case .forbidden:
            throw Refusal(message: "Tu n'es pas partie à cet échange.")
        case .notFound:
            throw Refusal(message: "Ce joueur ne propose plus assez de copies de cette carte.")
        case .conflict:
            throw Refusal(message: "Cette copie est déjà réservée par un autre échange.")
        case let .undocumented(statusCode, _):
            throw Refusal(message: "Le serveur a répondu \(statusCode).")
        }
    }
}

/// Resolves a set code to its human-readable name.
///
/// Both card screens show the set line under the card name and both fall back to the raw
/// code when the lookup fails (same as `frontend-vue`'s `DetailModal.vue`).
enum SetName {
    /// The set's name, or `nil` when it cannot be resolved — keep showing the code then.
    static func resolve(_ setCode: String) async -> String? {
        do {
            let output = try await APIClientProvider.shared.get_set(path: .init(set_code: setCode))
            if case let .ok(response) = output {
                return try response.body.json.name
            }
        } catch {
            // Keep the raw code.
        }
        return nil
    }
}
