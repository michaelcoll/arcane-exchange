import Testing

@testable import ArcaneExchange

struct ImportErrorMessageTests {
    @Test(arguments: [
        ("empty_file", "Le fichier est vide."),
        ("malformed_csv", "Ce fichier n'est pas un CSV lisible."),
        (
            "binder_export",
            "Ce fichier est un export de classeur. Exporte ta collection complète depuis ManaBox."
        ),
        ("unrecognized_format", "Ce fichier n'est pas un export collection ManaBox reconnu."),
        ("no_valid_line", "Aucune ligne du fichier n'a pu être lue."),
        ("internal", "Erreur technique, réessaie plus tard."),
    ])
    func translatesAKnownCode(code: String, message: String) {
        #expect(ImportErrorMessage.message(for: code) == message)
    }

    @Test(arguments: ["wrong_format", nil] as [String?])
    func fallsBackToAGenericMessage(code: String?) {
        #expect(ImportErrorMessage.message(for: code) == "L'import a échoué.")
    }

    @Test func namesTheFieldOfALineErrorInFrench() {
        #expect(
            ImportErrorMessage.lineError(line: 2, field: "language_code", value: "xx")
                == "Ligne 2 : langue invalide (xx)"
        )
    }

    @Test(arguments: [
        ("set_code", "set"),
        ("collector_number", "numéro de collection"),
        ("rarity", "rareté"),
        ("quantity", "quantité"),
        ("purchase_price", "prix d'achat"),
        ("scryfall_id", "identifiant Scryfall"),
        ("added_at", "date d'ajout"),
        ("proxy", "proxy"),
    ])
    func labelsEachField(field: String, label: String) {
        #expect(
            ImportErrorMessage.lineError(line: 3, field: field, value: "v")
                == "Ligne 3 : \(label) invalide (v)"
        )
    }

    @Test func labelsAnUnknownFieldGenerically() {
        #expect(
            ImportErrorMessage.lineError(line: 4, field: "foo", value: "v")
                == "Ligne 4 : valeur invalide (v)"
        )
    }
}
