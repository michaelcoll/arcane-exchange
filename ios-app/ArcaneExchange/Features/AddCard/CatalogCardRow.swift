import ArcaneExchangeCore
import SwiftUI

/// One impression of the catalog in the search results: its image first, so the player tells two
/// impressions of a card apart, then its names, set, number and rarity, and the copies the player
/// already owns.
struct CatalogCardRow: View {
    let card: SearchCard

    private static let imageWidth: CGFloat = 52

    var body: some View {
        HStack(spacing: 12) {
            CatalogCardImage(card: card)
                .frame(width: Self.imageWidth)
            VStack(alignment: .leading, spacing: 4) {
                Text(card.displayName)
                    .font(.body.weight(.semibold))
                    .lineLimit(2)
                if card.french_name != nil {
                    Text(card.name)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                CatalogCardSetLine(card: card)
                if card.owned_quantity > 0 {
                    Text("Possédée en \(card.owned_quantity) exemplaire\(card.owned_quantity > 1 ? "s" : "")")
                        .font(.caption.weight(.semibold))
                        .foregroundStyle(Palette.secondary)
                }
            }
        }
        .accessibilityElement(children: .combine)
    }
}

/// The image of a catalog impression: a local one when a Card of the impression has it, the
/// Scryfall one otherwise.
struct CatalogCardImage: View {
    let card: SearchCard

    var body: some View {
        Color.clear
            .aspectRatio(5.0 / 7.0, contentMode: .fit)
            .overlay { CardImageView(url: CardArtwork.url(imagePath: card.image_url), foil: card.isFoilOnly) }
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
    }
}

/// The set symbol tinted by rarity, the set name and the collector number.
struct CatalogCardSetLine: View {
    let card: SearchCard

    var body: some View {
        HStack(spacing: 5) {
            Text(String(KeyruneGlyph.glyph(forSetCode: card.set_code)))
                .font(.custom("Keyrune", size: 13))
                .foregroundStyle(RarityColor.tint(forRarityCode: card.rarity_code))
            Text(card.set_name)
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)
            Text("#\(card.collector_number)")
                .font(.caption.monospaced())
        }
        .lineLimit(1)
    }
}

#Preview("Ligne") {
    List(SearchCard.previewCatalog, id: \.self) { CatalogCardRow(card: $0) }
        .listStyle(.plain)
}

#Preview("Image") {
    CatalogCardImage(card: SearchCard.previewCatalog[0])
        .frame(width: 120)
}

#Preview("Ligne de set") {
    CatalogCardSetLine(card: SearchCard.previewCatalog[0])
        .padding()
}
