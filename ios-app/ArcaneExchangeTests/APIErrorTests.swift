import Foundation
import OpenAPIRuntime
import Testing

@testable import ArcaneExchange

struct APIErrorTests {
    /// Word for word the table of `frontend/app/utils/api-error.ts`.
    @Test(arguments: [
        ("empty_file", "Le fichier est vide."),
        ("malformed_csv", "Ce fichier n'est pas un CSV lisible."),
        (
            "binder_export",
            "Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox."
        ),
        ("unrecognized_format", "Ce fichier n'est pas un export collection ManaBox reconnu."),
        ("no_valid_line", "Aucune ligne du fichier n'a pu être lue."),
        ("import_already_running", "Un import est déjà en cours."),
        ("import_not_found", "Cet import n'existe pas."),
        ("self_trade", "Tu ne peux pas ouvrir un échange avec toi-même."),
        ("trade_not_modifiable", "Cet échange n'est plus modifiable."),
        ("trade_not_found", "Cet échange n'existe pas."),
        ("trade_access_denied", "Tu n'as pas accès à cet échange."),
        ("trade_not_acceptable", "Cet échange ne peut plus être accepté."),
        ("trade_already_accepted", "Tu as déjà accepté cet échange."),
        ("trade_already_finalized", "Cet échange est terminé, il ne peut plus être abandonné."),
        ("trade_not_fully_accepted", "Les deux joueurs doivent accepter l'échange avant de le confirmer."),
        ("trade_already_confirmed", "Tu as déjà confirmé cet échange."),
        ("trade_not_completed", "Tu pourras noter cet échange une fois qu'il sera terminé."),
        ("trade_already_rated", "Tu as déjà noté cet échange."),
        ("trade_concurrently_modified", "L'échange vient d'être modifié. Recharge la page avant de recommencer."),
        ("trade_empty", "Ajoute au moins une carte avant d'accepter cet échange."),
        ("trade_card_not_found", "Cette carte ne fait plus partie de l'échange."),
        ("card_already_reserved", "Cette carte est déjà réservée dans un autre échange."),
        ("binder_not_found", "Ce binder n'existe plus dans ta collection."),
        ("user_not_found", "Ce joueur est introuvable."),
        ("card_not_found", "Cette carte est introuvable ou n'est plus disponible."),
        ("set_not_found", "Ce set est introuvable."),
        ("unauthorized", "Ta session a expiré, reconnecte-toi."),
        ("internal", "Erreur technique, réessaie plus tard."),
    ])
    func translatesAKnownCode(code: String, message: String) {
        #expect(APIError(code: code, status: 409).message == message)
    }

    @Test(arguments: ["brand_new_code", nil] as [String?])
    func fallsBackToTheGenericMessage(code: String?) {
        #expect(APIError(code: code, status: 502).message == "Une erreur est survenue.")
    }

    @Test func readsAPassedErrorAsIs() {
        let error = APIError(code: "trade_empty", status: 409)
        #expect(APIError.read(error) == error)
    }

    @Test func readsAnErrorWrappedByTheClient() {
        let inner = APIError(code: "trade_empty", status: 409)
        let wrapped = ClientError(
            operationID: "accept_trade", operationInput: 0, causeDescription: "", underlyingError: inner
        )
        #expect(APIError.read(wrapped) == inner)
    }

    @Test func readsAnUnreachableServerAsANetworkFailure() {
        let error = APIError.read(URLError(.notConnectedToInternet))
        #expect(error.isNetworkFailure)
        #expect(error.status == nil)
        #expect(error.code == nil)
        #expect(error.message == "Serveur injoignable. Vérifie ta connexion, et l'URL de l'API dans Réglages ▸ Arcane Exchange.")
    }

    @Test func readsAClientWrappedURLErrorAsANetworkFailure() {
        let wrapped = ClientError(
            operationID: "x", operationInput: 0, causeDescription: "", underlyingError: URLError(.timedOut)
        )
        #expect(APIError.read(wrapped).isNetworkFailure)
    }

    @Test func readsAnythingElseAsTheGenericError() {
        struct Boom: Error {}
        let error = APIError.read(Boom())
        #expect(error.message == "Une erreur est survenue.")
        #expect(!error.isNetworkFailure)
    }
}
