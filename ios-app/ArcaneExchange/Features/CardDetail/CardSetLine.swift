import SwiftUI

/// The set line under a card's name (`csub` in the mockup): the set's Keyrune symbol tinted
/// by rarity, the set name, then the collector number.
struct CardSetLine: View {
    let card: CollectionCard
    let setName: String
    let isSetKnown: Bool
    var size: CGFloat = 13

    var body: some View {
        HStack(spacing: 5) {
            Text(String(KeyruneGlyph.glyph(forSetCode: card.set_code)))
                .font(.custom("Keyrune", size: size))
                .foregroundStyle(isSetKnown ? RarityColor.tint(forRarityCode: card.rarity_code) : .secondary)
            Text("\(setName)")
                .font(.system(size: size - 0.5).monospaced())
                .foregroundStyle(.secondary)
            Text("·")
                .font(.system(size: size - 0.5).monospaced())
                .foregroundStyle(.tertiary)
            Text("#\(card.collector_number)")
                .font(.system(size: size - 0.5).monospaced())
                .foregroundStyle(.primary)
        }
        .lineLimit(1)
    }
}

#Preview {
    VStack(alignment: .leading, spacing: 12) {
        CardSetLine(
            card: .init(
                collector_number: "243",
                foil: true,
                language_code: "fr",
                name: "The Soul Stone",
                rarity_code: "R",
                reserved: false,
                scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
                set_code: "SOA"
            ),
            setName: "Final Fantasy",
            isSetKnown: true
        )
        CardSetLine(
            card: .init(
                collector_number: "17",
                foil: false,
                language_code: "fr",
                name: "Contrat divin",
                rarity_code: "M",
                reserved: false,
                scryfall_id: "d3f0f4d1-6f0c-4e0a-9f3d-9a6f2b8f5b34",
                set_code: "UNK"
            ),
            // The set name has not resolved yet: falls back to the raw code, no rarity tint.
            setName: "UNK",
            isSetKnown: false,
            size: 15
        )
    }
    .padding()
}
