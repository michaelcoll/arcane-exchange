#if DEBUG
    import APIClient
    import ArcaneExchangeCore

    /// Fixtures shared by the AddCard previews: a common with a French name that the player owns, a
    /// mythic with no French name and a mythic with a long double-faced name, so a preview shows the
    /// row's variants.
    extension SearchCard {
        static let previewCatalog: [SearchCard] = [
            SearchCard(
                collector_number: "0087",
                french_name: "Gobelins abordeurs",
                image_url: "https://cards.scryfall.io/normal/front/4/4/4409a063-bf2a-4a49-803e-3ce6bd474353.jpg",
                name: "Goblin Boarders",
                offered_languages: [],
                owned_quantity: 2,
                owner_count: 3,
                rarity_code: "C",
                scryfall_id: "4409a063-bf2a-4a49-803e-3ce6bd474353",
                set_code: "FDN",
                set_name: "Foundations"
            ),
            SearchCard(
                collector_number: "0123",
                image_url: "https://cards.scryfall.io/normal/front/7/a/7a79190f-de60-4eb6-b925-594eb76ca8c3.jpg",
                name: "Vampiric Tutor",
                offered_languages: [],
                owned_quantity: 0,
                owner_count: 0,
                rarity_code: "M",
                scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
                set_code: "SOA",
                set_name: "Store Championships"
            ),
            SearchCard(
                collector_number: "0013",
                french_name: "Eirdu, porteuse d'aube // Isilu, porteur de crépuscule",
                image_url: "https://cards.scryfall.io/normal/front/b/2/b2d9d5ca-7e15-437a-bdfc-5972b42148fe.jpg",
                name: "Eirdu, Carrier of Dawn // Isilu, Carrier of Twilight",
                offered_languages: [],
                owned_quantity: 0,
                owner_count: 1,
                rarity_code: "M",
                scryfall_id: "b2d9d5ca-7e15-437a-bdfc-5972b42148fe",
                set_code: "ECL",
                set_name: "Lorwyn Eclipsed"
            )
        ]
    }
#endif
