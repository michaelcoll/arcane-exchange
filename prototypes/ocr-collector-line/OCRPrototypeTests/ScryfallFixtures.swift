import Foundation

/// Trimmed Scryfall responses (kept fields only), taken from the 2026-11-05 research notes.
enum ScryfallFixtures {
    static let transformFrench = Data("""
    {"object":"card","set":"mom","collector_number":"88","layout":"transform","lang":"fr",
     "name":"Aetherblade Agent // Gitaxian Mindstinger",
     "printed_name":"Agent étherlame // Piqueur d'esprit gitaxien",
     "card_faces":[{"name":"Aetherblade Agent","printed_name":"Agent étherlame"},
                   {"name":"Gitaxian Mindstinger","printed_name":"Piqueur d'esprit gitaxien"}]}
    """.utf8)

    static let notFound = Data("""
    {"object":"error","code":"not_found","status":404,
     "details":"No card found with the given ID or set code and collector number."}
    """.utf8)
}
