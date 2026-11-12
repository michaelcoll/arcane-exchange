import ArcaneCore
import SwiftUI

/// One impression of the catalog in the search results: its Scryfall image first, so the player
/// tells two printings of a card apart, then its names, set, number and rarity.
struct CatalogCardRow: View {
    let card: CatalogCard

    private static let imageWidth: CGFloat = 52

    var body: some View {
        HStack(spacing: 12) {
            CatalogCardImage(card: card)
                .frame(width: Self.imageWidth)
            VStack(alignment: .leading, spacing: 4) {
                Text(card.french_name ?? card.name)
                    .font(.body.weight(.semibold))
                    .lineLimit(2)
                if card.french_name != nil {
                    Text(card.name)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                        .lineLimit(1)
                }
                CatalogCardSetLine(card: card)
            }
        }
        .accessibilityElement(children: .combine)
    }
}

/// The Scryfall image of a catalog impression. Scryfall's CDN serves it as is: the URL is
/// absolute, unlike the paths of a Card's own images.
struct CatalogCardImage: View {
    let card: CatalogCard

    var body: some View {
        Color.clear
            .aspectRatio(5.0 / 7.0, contentMode: .fit)
            .overlay { CardImageView(url: card.image_url.flatMap(URL.init(string:))) }
            .clipShape(RoundedRectangle(cornerRadius: 4, style: .continuous))
    }
}

/// The set symbol tinted by rarity, the set name and the collector number.
struct CatalogCardSetLine: View {
    let card: CatalogCard

    var body: some View {
        HStack(spacing: 5) {
            Text(String(KeyruneGlyph.glyph(forSetCode: card.set.code)))
                .font(.custom("Keyrune", size: 13))
                .foregroundStyle(RarityColor.tint(forRarityCode: card.rarity_code))
            Text(card.set.name)
                .font(.caption.monospaced())
                .foregroundStyle(.secondary)
            Text("#\(card.collector_number)")
                .font(.caption.monospaced())
        }
        .lineLimit(1)
    }
}

#Preview("Ligne") {
    List(CatalogCard.previewCatalog, id: \.self) { CatalogCardRow(card: $0) }
        .listStyle(.plain)
}

#Preview("Image") {
    CatalogCardImage(card: CatalogCard.previewCatalog[0])
        .frame(width: 120)
}

#Preview("Ligne de set") {
    CatalogCardSetLine(card: CatalogCard.previewCatalog[0])
        .padding()
}
