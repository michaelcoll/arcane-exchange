/// French messages for the import errors the API reports by code (ADR 0018) — mirrors
/// `frontend-vue/app/utils/import-error.ts`. The API's technical message is never shown.
enum ImportErrorMessage {
    private static let messages: [String: String] = [
        "empty_file": "Le fichier est vide.",
        "malformed_csv": "Ce fichier n'est pas un CSV lisible.",
        "binder_export":
            "Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox.",
        "unrecognized_format": "Ce fichier n'est pas un export collection ManaBox reconnu.",
        "no_valid_line": "Aucune ligne du fichier n'a pu être lue.",
        "internal": "Erreur technique, réessaie plus tard."
    ]

    private static let fieldLabels: [String: String] = [
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

    /// An unknown or missing code gets a generic message.
    static func message(for code: String?) -> String {
        code.flatMap { messages[$0] } ?? "L'import a échoué."
    }

    static func lineError(line: Int, field: String, value: String) -> String {
        "Ligne \(line) : \(fieldLabels[field] ?? "valeur") invalide (\(value))"
    }
}
