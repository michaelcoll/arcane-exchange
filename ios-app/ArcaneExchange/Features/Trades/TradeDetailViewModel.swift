import APIClient
import Foundation
import OpenAPIRuntime

/// Backs `TradeDetailView`: `GET /trades/{id}` plus the five lifecycle actions.
///
/// Every action ends with a reload rather than a local mutation. The backend owns the state
/// machine — accepting can flip the trade to `FULLY_ACCEPTED`, removing a card can knock it
/// back to `PENDING` and release the reservations — so guessing the next state here would
/// only be a second, less reliable implementation of `trade-workflow.instructions.md`.
@MainActor
@Observable
final class TradeDetailViewModel {
    enum LoadError: Equatable {
        case forbidden
        case notFound
        case unauthorized
        case network
        case other

        var title: String {
            switch self {
            case .notFound: "Cet échange n'existe pas"
            case .forbidden: "Tu n'as pas accès à cet échange"
            default: "Impossible de charger cet échange"
            }
        }

        var message: String {
            switch self {
            case .forbidden: "Seules les deux parties d'un échange peuvent l'ouvrir."
            case .notFound: "Il a peut-être été supprimé."
            case .unauthorized: "Session expirée. Reconnecte-toi."
            case .network:
                "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange."
            case .other: "Réessaie dans un instant."
            }
        }
    }

    let tradeID: String

    private(set) var trade: TradeDetail?
    private(set) var isLoading = false
    private(set) var loadError: LoadError?
    /// An action is in flight — the action bar disables itself rather than queueing taps.
    private(set) var isBusy = false
    /// Set when an action is refused; the view shows it in an alert and clears it.
    var actionError: String?

    init(tradeID: String) {
        self.tradeID = tradeID
    }

    var status: TradeStatus {
        TradeStatus(apiValue: trade?.status ?? "PENDING")
    }

    var partnerUsername: String {
        trade?.partner_username ?? ""
    }

    /// Cards leaving the user's collection — asked for by the partner, so not removable here.
    var myCards: [TradeCard] {
        trade?.my_cards ?? []
    }

    /// Cards the user asked the partner for.
    var partnerCards: [TradeCard] {
        trade?.partner_cards ?? []
    }

    var balance: TradeBalance {
        TradeBalance(give: myCards, get: partnerCards)
    }

    var meAccepted: Bool {
        trade?.me.accepted ?? false
    }

    var meConfirmed: Bool {
        trade?.me.confirmed ?? false
    }

    var myRating: Int? {
        trade?.me.rating.map(Int.init)
    }

    var partnerRating: Int? {
        trade?.partner.rating.map(Int.init)
    }

    func load() async {
        isLoading = true
        loadError = nil
        do {
            switch try await APIClientProvider.shared.get_trade(path: .init(trade_id: tradeID)) {
            case let .ok(response):
                trade = try response.body.json
            case .unauthorized:
                loadError = .unauthorized
            case .forbidden:
                loadError = .forbidden
            case .notFound:
                loadError = .notFound
            case .undocumented:
                loadError = .other
            }
        } catch {
            let underlying = (error as? ClientError)?.underlyingError ?? error
            loadError = underlying is URLError ? .network : .other
        }
        isLoading = false
    }

    func accept() async {
        await run {
            try await Self.outcome(of: APIClientProvider.shared.accept_trade(path: .init(trade_id: self.tradeID)))
        }
    }

    func abandon() async {
        await run {
            try await Self.outcome(of: APIClientProvider.shared.abandon_trade(path: .init(trade_id: self.tradeID)))
        }
    }

    func confirmExchange() async {
        await run {
            try await Self.outcome(of: APIClientProvider.shared.confirm_trade(path: .init(trade_id: self.tradeID)))
        }
    }

    /// 0 to 5 stars. Zero is the mockup's "passer la notation": the trade still closes, the
    /// partner's average is simply left untouched by a skipped rating.
    func rate(_ rating: Int) async {
        await run {
            try await Self.outcome(of: APIClientProvider.shared.rate_trade(
                path: .init(trade_id: self.tradeID),
                body: .json(.init(rating: Int32(rating)))
            ))
        }
    }

    /// Drops one of the partner's cards from the trade. Only the "je reçois" side is editable
    /// from this screen — the cards the partner asked for are theirs to change.
    func removePartnerCard(_ card: TradeCard) async {
        let owner = partnerUsername
        await run {
            try await Self.outcome(of: APIClientProvider.shared.remove_trade_card(
                path: .init(trade_id: self.tradeID),
                body: .json(.init(
                    collector_number: card.collector_number,
                    foil: card.foil,
                    language_code: card.language_code,
                    owner_username: owner,
                    set_code: card.set_code
                ))
            ))
        }
    }

    /// Runs one action, then reloads whatever the backend made of it. A refusal (409 above
    /// all: the trade moved on in another session) surfaces as `actionError`.
    private func run(_ action: @escaping () async throws -> String?) async {
        guard !isBusy else { return }
        isBusy = true
        do {
            actionError = try await action()
        } catch {
            let underlying = (error as? ClientError)?.underlyingError ?? error
            actionError = underlying is URLError
                ? "Serveur injoignable. Réessaie une fois la connexion revenue."
                : "Une erreur est survenue."
        }
        await load()
        isBusy = false
    }

    /// Maps a 204-or-error action response to a user-facing message, `nil` meaning success.
    ///
    /// The five mutating endpoints share this exact response set, and the generated `Output`
    /// enums are distinct types with no common protocol, hence the tiny per-case shims above.
    private static func outcome(of output: Operations.accept_trade.Output) -> String? {
        switch output {
        case .noContent: nil
        case .unauthorized: "Session expirée. Reconnecte-toi."
        case .forbidden: "Tu n'es pas partie à cet échange."
        case .notFound: "Cet échange n'existe plus."
        case .conflict: "L'échange a changé entre-temps : cette action n'est plus possible."
        case let .undocumented(statusCode, _): "Le serveur a répondu \(statusCode)."
        }
    }

    private static func outcome(of output: Operations.abandon_trade.Output) -> String? {
        switch output {
        case .noContent: nil
        case .unauthorized: "Session expirée. Reconnecte-toi."
        case .forbidden: "Tu n'es pas partie à cet échange."
        case .notFound: "Cet échange n'existe plus."
        case .conflict: "L'échange est déjà finalisé : il ne peut plus être abandonné."
        case let .undocumented(statusCode, _): "Le serveur a répondu \(statusCode)."
        }
    }

    private static func outcome(of output: Operations.confirm_trade.Output) -> String? {
        switch output {
        case .noContent: nil
        case .unauthorized: "Session expirée. Reconnecte-toi."
        case .forbidden: "Tu n'es pas partie à cet échange."
        case .notFound: "Cet échange n'existe plus."
        case .conflict: "L'échange doit être verrouillé par les deux parties avant d'être confirmé."
        case let .undocumented(statusCode, _): "Le serveur a répondu \(statusCode)."
        }
    }

    private static func outcome(of output: Operations.rate_trade.Output) -> String? {
        switch output {
        case .noContent: nil
        case .badRequest: "Note invalide."
        case .unauthorized: "Session expirée. Reconnecte-toi."
        case .forbidden: "Tu n'es pas partie à cet échange."
        case .notFound: "Cet échange n'existe plus."
        case .conflict: "Tu as déjà noté cet échange."
        case let .undocumented(statusCode, _): "Le serveur a répondu \(statusCode)."
        }
    }

    private static func outcome(of output: Operations.remove_trade_card.Output) -> String? {
        switch output {
        case .noContent: nil
        case .badRequest: "Requête invalide."
        case .unauthorized: "Session expirée. Reconnecte-toi."
        case .forbidden: "Tu n'es pas partie à cet échange."
        case .notFound: "Cette carte ne fait plus partie de l'échange."
        case .conflict: "L'échange ne peut plus être modifié."
        case let .undocumented(statusCode, _): "Le serveur a répondu \(statusCode)."
        }
    }
}
