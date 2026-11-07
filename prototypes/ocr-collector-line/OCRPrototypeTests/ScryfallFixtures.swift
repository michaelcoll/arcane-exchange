import Foundation

/// Trimmed Scryfall responses (kept fields only), taken from the 2026-11-05 research notes.
enum ScryfallFixtures {
    static let doubleFacedToken = Data("""
    {"object":"card","set":"tmom","collector_number":"16","layout":"double_faced_token","lang":"en",
     "name":"Incubator // Phyrexian",
     "card_faces":[{"name":"Incubator","type_line":"Token Artifact — Incubator"},
                   {"name":"Phyrexian","type_line":"Token Artifact Creature — Phyrexian"}]}
    """.utf8)

    static let transformFrench = Data("""
    {"object":"card","set":"mom","collector_number":"88","layout":"transform","lang":"fr",
     "name":"Aetherblade Agent // Gitaxian Mindstinger",
     "printed_name":"Agent étherlame // Piqueur d'esprit gitaxien",
     "card_faces":[{"name":"Aetherblade Agent","printed_name":"Agent étherlame"},
                   {"name":"Gitaxian Mindstinger","printed_name":"Piqueur d'esprit gitaxien"}]}
    """.utf8)

    static let regularToken = Data("""
    {"object":"card","set":"tmkm","collector_number":"12","layout":"token","lang":"en","name":"Spirit"}
    """.utf8)

    static let notFound = Data("""
    {"object":"error","code":"not_found","status":404,
     "details":"No card found with the given ID or set code and collector number."}
    """.utf8)
}
