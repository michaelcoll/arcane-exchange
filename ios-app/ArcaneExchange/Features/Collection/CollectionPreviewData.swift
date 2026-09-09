#if DEBUG
    import Foundation

    /// Fixtures shared by the Collection previews — a small spread of rarities, prices and foil
    /// states, so a preview shows the grid's real variety rather than a single happy path.
    extension CollectionCard {
        static let previewGrid: [CollectionCard] = [
            CollectionCard(
                collection_entry: .init(added_at: "2026-01-05T10:00:00Z", purchase_price: 2400, quantity: 2),
                collector_number: "0123",
                foil: true,
                language_code: "jp",
                name: "Vampiric Tutor",
                price_guide: .init(avg: 2900, low: 2500, trend: 2800),
                rarity_code: "M",
                reserved: true,
                scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
                set_code: "SOA"
            ),
            CollectionCard(
                collection_entry: .init(added_at: "2026-02-11T08:30:00Z", purchase_price: 800, quantity: 4),
                collector_number: "0087",
                foil: false,
                language_code: "fr",
                name: "Goblin Boarders",
                price_guide: .init(avg: 850, low: 600, trend: 900),
                rarity_code: "C",
                reserved: false,
                scryfall_id: "4409a063-bf2a-4a49-803e-3ce6bd474353",
                set_code: "FDN"
            ),
            CollectionCard(
                collection_entry: .init(added_at: "2026-02-20T14:00:00Z", purchase_price: 100, quantity: 1),
                collector_number: "0032",
                foil: false,
                language_code: "fr",
                name: "Repeal",
                price_guide: nil,
                rarity_code: "C",
                reserved: false,
                scryfall_id: "9e7dd929-4bba-46a6-86c9-b8ed853eb721",
                set_code: "GPT"
            ),
            CollectionCard(
                collection_entry: .init(added_at: "2026-03-01T09:15:00Z", purchase_price: 1235, quantity: 1),
                collector_number: "0013",
                foil: false,
                language_code: "fr",
                name: "Eirdu, Carrier of Dawn // Isilu, Carrier of Twilight",
                price_guide: .init(avg: 1100, low: 950, trend: 1050),
                rarity_code: "M",
                reserved: false,
                scryfall_id: "b2d9d5ca-7e15-437a-bdfc-5972b42148fe",
                set_code: "ECL"
            ),
            CollectionCard(
                collection_entry: .init(added_at: "2026-03-04T18:45:00Z", purchase_price: 175, quantity: 1),
                collector_number: "0007",
                foil: false,
                language_code: "fr",
                name: "Brigid, Clachan's Heart // Brigid, Doun's Mind",
                price_guide: .init(avg: 190, low: 150, trend: 210),
                rarity_code: "R",
                reserved: false,
                scryfall_id: "cb7d5bbb-4f68-4e38-8bb0-a95af21b24c8",
                set_code: "ECL"
            ),
            CollectionCard(
                collection_entry: .init(added_at: "2026-03-06T12:00:00Z", purchase_price: 76, quantity: 1),
                collector_number: "184s",
                foil: true,
                language_code: "fr",
                name: "Felothar, Dawn of the Abzan",
                price_guide: .init(avg: 60, low: 45, trend: 55),
                rarity_code: "R",
                reserved: false,
                scryfall_id: "09478378-c28b-4334-a0a1-157325ed8e5b",
                set_code: "PTDM"
            )
        ]
    }
#endif
