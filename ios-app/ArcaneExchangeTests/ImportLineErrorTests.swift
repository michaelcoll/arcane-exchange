import Testing

@testable import ArcaneExchange

struct ImportLineErrorTests {
    @Test func namesTheFieldOfALineErrorInFrench() {
        #expect(
            APIErrorMessages.importLineError(line: 2, field: "language_code", value: "xx")
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
            APIErrorMessages.importLineError(line: 3, field: field, value: "v")
                == "Ligne 3 : \(label) invalide (v)"
        )
    }

    @Test func labelsAnUnknownFieldGenerically() {
        #expect(
            APIErrorMessages.importLineError(line: 4, field: "foo", value: "v")
                == "Ligne 4 : valeur invalide (v)"
        )
    }
}
