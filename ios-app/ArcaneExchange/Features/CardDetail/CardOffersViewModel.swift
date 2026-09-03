import APIClient
import Foundation

/// Backs `CardOffersView`: one page of `GET /card/offers` for the card at hand, plus the
/// "propose an exchange" action each row offers.
@MainActor
@Observable
final class CardOffersViewModel {
    enum State: Equatable {
        case loading
        case loaded([CardOffer])
        case failed
    }

    let card: CollectionCard
    private(set) var state: State = .loading
    /// The owner whose trade is being created, so only that row shows a spinner.
    private(set) var startingWith: String?
    /// Set when starting a trade is refused; the view shows it in an alert and clears it.
    var startError: String?

    init(card: CollectionCard) {
        self.card = card
    }

    func load() async {
        state = .loading
        do {
            let output = try await APIClientProvider.shared.get_card_offers(
                query: .init(
                    set_code: card.set_code,
                    collector_number: card.collector_number,
                    language_code: card.language_code,
                    foil: card.foil,
                    sort_by: .selling_price,
                    page: 0,
                    page_size: 20
                )
            )
            switch output {
            case let .ok(response):
                state = try .loaded(response.body.json.items)
            case .badRequest, .notFound:
                // Nothing to show rather than an error screen.
                state = .loaded([])
            case .unauthorized, .undocumented:
                state = .failed
            }
        } catch {
            state = .failed
        }
    }

    /// Opens (or reuses) the trade with this card's owner and puts the card on their side.
    ///
    /// Two calls, exactly like the web client's `startTrade`: `POST /trades` is idempotent per
    /// partner — it hands back the existing active trade — so this doubles as "add this card
    /// to the trade I already have with them".
    func startTrade(with offer: CardOffer) async -> TradeDetailRoute? {
        guard startingWith == nil else { return nil }
        startingWith = offer.owner_username
        defer { startingWith = nil }
        do {
            guard let tradeID = try await createTrade(with: offer.owner_username) else { return nil }
            guard try await addCard(to: tradeID, ownedBy: offer.owner_username) else { return nil }
            return TradeDetailRoute(id: tradeID, partnerUsername: offer.owner_username)
        } catch {
            startError = "Serveur injoignable. Réessaie une fois la connexion revenue."
            return nil
        }
    }

    private func createTrade(with owner: String) async throws -> String? {
        let output = try await APIClientProvider.shared.create_trade(
            body: .json(.init(respondent_username: owner))
        )
        switch output {
        case let .created(response):
            return try response.body.json.id
        case .badRequest:
            startError = "Tu ne peux pas ouvrir un échange avec toi-même."
        case .unauthorized:
            startError = "Session expirée. Reconnecte-toi."
        case .notFound:
            startError = "Ce joueur n'existe plus."
        case let .undocumented(statusCode, _):
            startError = "Le serveur a répondu \(statusCode)."
        }
        return nil
    }

    private func addCard(to tradeID: String, ownedBy owner: String) async throws -> Bool {
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
            return true
        case .badRequest:
            startError = "Requête invalide."
        case .unauthorized:
            startError = "Session expirée. Reconnecte-toi."
        case .forbidden:
            startError = "Tu n'es pas partie à cet échange."
        case .notFound:
            startError = "Ce joueur ne propose plus assez de copies de cette carte."
        case .conflict:
            startError = "Cette copie est déjà réservée par un autre échange."
        case let .undocumented(statusCode, _):
            startError = "Le serveur a répondu \(statusCode)."
        }
        return false
    }
}
