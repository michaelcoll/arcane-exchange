import APIClient
import Foundation

@testable import ArcaneExchange

/// A domain of its own per call: no test reads `.standard` or another test's writes.
func isolatedDefaults() -> UserDefaults {
    UserDefaults(suiteName: "ArcaneExchangeTests.\(UUID().uuidString)")!
}

func card(_ id: String) -> CollectionCard {
    CollectionCard(
        collector_number: "001",
        foil: false,
        image_url: nil,
        language_code: "fr",
        name: "Card \(id)",
        rarity_code: "C",
        reserved: false,
        scryfall_id: id,
        set_code: "FDN"
    )
}

func testCard() -> CollectionCard {
    CollectionCard(
        collector_number: "243",
        foil: false,
        language_code: "fr",
        name: "The Soul Stone",
        rarity_code: "R",
        reserved: false,
        scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
        set_code: "FIN"
    )
}

func offer(_ owner: String, reserved: Bool = false) -> CardOffer {
    CardOffer(owner_username: owner, quantity: 1, reserved: reserved, selling_price: 500)
}

func offersPage(_ items: [CardOffer], total: Int64, pageSize: Int32 = 20) -> Operations.get_card_offers.Output {
    .ok(.init(body: .json(.init(items: items, page: 0, page_size: pageSize, total: total))))
}
