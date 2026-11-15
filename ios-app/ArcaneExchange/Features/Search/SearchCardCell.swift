import ArcaneExchangeCore
import SwiftUI

/// One tile of the search results grid: artwork first, reference price second. A search result
/// holds no collection entry, so there is no quantity and no purchase price to show.
struct SearchCardCell: View {
    let card: SearchCard

    private static let cornerRadius: CGFloat = 8

    private var reserved: Bool {
        card.reserved ?? false
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            artwork
            VStack(alignment: .leading, spacing: 4) {
                Text(card.displayName)
                    .font(.caption)
                    .fontWeight(.semibold)
                    .lineLimit(1)
                if let priceCents = card.referencePriceCents {
                    Text(Price.euros(cents: priceCents))
                        .font(.caption)
                        .fontWeight(.semibold)
                        .monospacedDigit()
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
    }

    private var artwork: some View {
        Color.clear
            .aspectRatio(5.0 / 7.0, contentMode: .fit)
            .overlay { CardImageView(url: CardArtwork.url(imagePath: card.image_url), foil: card.isFoilOnly) }
            .clipShape(RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous))
            .overlay { border }
            .overlay(alignment: .top) { reservedFlag }
            .cardArtworkLift(
                tint: nil,
                shadowOpacity: 0.3,
                shadowRadius: 4,
                shadowY: 3,
                haloRadius: 3
            )
    }

    private var border: some View {
        RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous)
            .strokeBorder(
                reserved ? AnyShapeStyle(Palette.secondary) : AnyShapeStyle(.black.opacity(0.4)),
                lineWidth: reserved ? 2 : 1
            )
    }

    @ViewBuilder private var reservedFlag: some View {
        if reserved {
            Label("Réservée", systemImage: "lock.fill")
                .font(.caption2)
                .fontWeight(.semibold)
                .padding(.horizontal, 7)
                .padding(.vertical, 3)
                .reservedBadgeChip(in: Capsule())
                .padding(6)
        }
    }
}

#Preview {
    SearchCardCell(
        card: SearchCard(
            collector_number: "0123",
            foil_price_guide: .init(avg: 2900, low: 2500, trend: 2800),
            name: "Vampiric Tutor",
            offered_languages: ["FR"],
            owner_count: 3,
            rarity_code: "M",
            reserved: true,
            scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
            set_code: "SOA",
            set_name: "Secrets of Arcana"
        )
    )
    .frame(width: 170)
    .padding()
}
