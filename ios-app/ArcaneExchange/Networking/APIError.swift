import Foundation
import OpenAPIRuntime

/// The one shape an API failure reaches a caller in (ADR 0018, ADR 0020): the iOS counterpart
/// of the web's `ApiError`. A caller never looks at a generated error case, nor at a raw
/// status: it reads any caught value with `APIError.read` and shows `message`.
struct APIError: Error, Equatable, LocalizedError {
    /// Stable code of the API, as the backend emits it. Absent without a response body carrying one.
    let code: String?
    /// HTTP status, for diagnosis only. Absent when the API did not answer (network failure).
    let status: Int?
    /// The API did not answer: unreachable server, timeout…
    let isNetworkFailure: Bool
    /// French message, ready to show to the player.
    let message: String

    init(code: String?, status: Int?, isNetworkFailure: Bool = false) {
        self.code = code
        self.status = status
        self.isNetworkFailure = isNetworkFailure
        message = isNetworkFailure ? APIErrorMessages.network : APIErrorMessages.message(for: code)
    }

    var errorDescription: String? {
        message
    }

    /// Turns any caught value into an `APIError`: one already raised by the client middleware
    /// (possibly wrapped in a `ClientError`), a `URLError`, or anything else, which gets the
    /// generic message.
    static func read(_ error: any Error) -> APIError {
        if let apiError = error as? APIError {
            return apiError
        }
        if let clientError = error as? ClientError {
            return read(clientError.underlyingError)
        }
        if error is URLError {
            return APIError(code: nil, status: nil, isNetworkFailure: true)
        }
        return APIError(code: nil, status: nil)
    }
}

/// French messages of the stable codes the API returns, copied word for word from
/// `frontend/app/utils/api-error.ts` (ADR 0018): keep the two tables aligned. An unknown code
/// shows the generic message, so a code added by the backend never breaks an older app.
enum APIErrorMessages {
    static let generic = "Une erreur est survenue."
    static let network = "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange."

    /// `fallback` replaces the generic message for a missing or unknown code.
    static func message(for code: String?, fallback: String = generic) -> String {
        code.flatMap { messages[$0] } ?? fallback
    }

    private static let importFieldLabels: [String: String] = [
        "set_code": "set",
        "collector_number": "numéro de collection",
        "rarity": "rareté",
        "language_code": "langue",
        "quantity": "quantité",
        "purchase_price": "prix d'achat",
        "scryfall_id": "identifiant Scryfall",
        "added_at": "date d'ajout",
        "proxy": "proxy"
    ]

    /// Message of one rejected line of a ManaBox import, mirrors `frontend/app/utils/import-error.ts`.
    static func importLineError(line: Int, field: String, value: String) -> String {
        "Ligne \(line) : \(importFieldLabels[field] ?? "valeur") invalide (\(value))"
    }

    private static let messages: [String: String] = [
        "empty_file": "Le fichier est vide.",
        "malformed_csv": "Ce fichier n'est pas un CSV lisible.",
        "binder_export":
            "Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox.",
        "unrecognized_format": "Ce fichier n'est pas un export collection ManaBox reconnu.",
        "no_valid_line": "Aucune ligne du fichier n'a pu être lue.",
        "import_already_running": "Un import est déjà en cours.",
        "import_not_found": "Cet import n'existe pas.",
        "self_trade": "Tu ne peux pas ouvrir un échange avec toi-même.",
        "trade_not_modifiable": "Cet échange n'est plus modifiable.",
        "trade_not_found": "Cet échange n'existe pas.",
        "trade_access_denied": "Tu n'as pas accès à cet échange.",
        "trade_not_acceptable": "Cet échange ne peut plus être accepté.",
        "trade_already_accepted": "Tu as déjà accepté cet échange.",
        "trade_already_finalized": "Cet échange est terminé, il ne peut plus être abandonné.",
        "trade_not_fully_accepted": "Les deux joueurs doivent accepter l'échange avant de le confirmer.",
        "trade_already_confirmed": "Tu as déjà confirmé cet échange.",
        "trade_not_completed": "Tu pourras noter cet échange une fois qu'il sera terminé.",
        "trade_already_rated": "Tu as déjà noté cet échange.",
        "trade_concurrently_modified": "L'échange vient d'être modifié. Recharge la page avant de recommencer.",
        "trade_empty": "Ajoute au moins une carte avant d'accepter cet échange.",
        "trade_card_not_found": "Cette carte ne fait plus partie de l'échange.",
        "card_already_reserved": "Cette carte est déjà réservée dans un autre échange.",
        "binder_not_found": "Ce binder n'existe plus dans ta collection.",
        "user_not_found": "Ce joueur est introuvable.",
        // Also what adding a card to a trade answers when its owner no longer offers it.
        "card_not_found": "Cette carte est introuvable ou n'est plus disponible.",
        "set_not_found": "Ce set est introuvable.",
        "unauthorized": "Ta session a expiré, reconnecte-toi.",
        "internal": "Erreur technique, réessaie plus tard."
    ]
}
