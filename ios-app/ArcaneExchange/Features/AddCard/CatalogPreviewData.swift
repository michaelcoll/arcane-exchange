#if DEBUG
    import APIClient
    import ArcaneExchangeCore

    /// Fixtures shared by the AddCard previews: a common with a French name, a mythic with no
    /// French name and a mythic with a long double-faced name, so a preview shows the row's variants.
    extension CatalogCard {
        static let previewCatalog: [CatalogCard] = [
            CatalogCard(
                collector_number: "0087",
                french_name: "Gobelins abordeurs",
                image_url: "https://cards.scryfall.io/normal/front/4/4/4409a063-bf2a-4a49-803e-3ce6bd474353.jpg",
                name: "Goblin Boarders",
                rarity_code: "C",
                set: .init(code: "FDN", name: "Foundations")
            ),
            CatalogCard(
                collector_number: "0123",
                french_name: nil,
                image_url: "https://cards.scryfall.io/normal/front/7/a/7a79190f-de60-4eb6-b925-594eb76ca8c3.jpg",
                name: "Vampiric Tutor",
                rarity_code: "M",
                set: .init(code: "SOA", name: "Store Championships")
            ),
            CatalogCard(
                collector_number: "0013",
                french_name: "Eirdu, porteuse d'aube // Isilu, porteur de crépuscule",
                image_url: "https://cards.scryfall.io/normal/front/b/2/b2d9d5ca-7e15-437a-bdfc-5972b42148fe.jpg",
                name: "Eirdu, Carrier of Dawn // Isilu, Carrier of Twilight",
                rarity_code: "M",
                set: .init(code: "ECL", name: "Lorwyn Eclipsed")
            )
        ]
    }
#endif
