import NukeUI
import SwiftUI

/// One tile of the collection grid: artwork first, figures second — the mockup's
/// "la carte se lit d'abord, le chiffre ensuite".
struct CollectionCardCell: View {
    let card: CollectionCard

    private static let cornerRadius: CGFloat = 8

    private var quantity: Int {
        Int(card.collection_entry?.quantity ?? 0)
    }

    private var purchaseCents: Int? {
        card.collection_entry.map { Int($0.purchase_price) }
    }

    private var trendCents: Int? {
        card.price_guide.flatMap(\.trend).map(Int.init)
    }

    private var deal: CardDeal? {
        CardDeal(purchaseCents: purchaseCents, trendCents: trendCents)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            artwork
            VStack(alignment: .leading, spacing: 4) {
                Text(card.name)
                    .font(.caption)
                    .fontWeight(.semibold)
                    .lineLimit(1)
                priceRow
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .accessibilityElement(children: .combine)
    }

    private var artwork: some View {
        Color.clear
            .aspectRatio(5.0 / 7.0, contentMode: .fit)
            .overlay { image }
            .overlay { foilSheen }
            .clipShape(RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous))
            .overlay { border }
            .overlay(alignment: .topTrailing) { quantityBadge }
            .overlay(alignment: .top) { reservedFlag }
            .shadow(color: .black.opacity(0.3), radius: 5, y: 3)
    }

    /// `LazyImage` rather than `AsyncImage`: the grid re-creates cells as it scrolls, and
    /// `AsyncImage` has no decoded-image cache, so every reappearance meant a fresh decode
    /// and a placeholder flash. Nuke serves those from memory. See `ArtworkPipeline`.
    private var image: some View {
        let url = CardArtwork.url(gathererID: card.the_gatherer_id, scryfallID: card.scryfall_id)
        return LazyImage(url: url) { state in
            if let image = state.image {
                image.resizable().scaledToFill()
            } else if state.error != nil {
                placeholder
            } else {
                placeholder.overlay { ProgressView().controlSize(.small) }
            }
        }
    }

    private var placeholder: some View {
        Rectangle()
            .fill(.quaternary)
            .overlay {
                Text(card.name)
                    .font(.caption2)
                    .multilineTextAlignment(.center)
                    .foregroundStyle(.secondary)
                    .padding(6)
            }
    }

    private var border: some View {
        RoundedRectangle(cornerRadius: Self.cornerRadius, style: .continuous)
            .strokeBorder(
                card.reserved ? AnyShapeStyle(Color.purple) : AnyShapeStyle(.black.opacity(0.4)),
                lineWidth: card.reserved ? 2 : 1
            )
    }

    /// Stand-in for the mockup's animated foil: a static sheen reads as "foil" without a
    /// scroll-driven animation on every visible tile.
    @ViewBuilder private var foilSheen: some View {
        if card.foil {
            LinearGradient(
                colors: [.cyan.opacity(0.2), .clear, .purple.opacity(0.16), .clear],
                startPoint: .topLeading,
                endPoint: .bottomTrailing
            )
            .blendMode(.plusLighter)
            .allowsHitTesting(false)
        }
    }

    @ViewBuilder private var quantityBadge: some View {
        if quantity > 1 {
            Text("×\(quantity)")
                .font(.caption2)
                .fontWeight(.semibold)
                .monospacedDigit()
                .padding(.horizontal, 6)
                .padding(.vertical, 2)
                .background(.ultraThinMaterial, in: Capsule())
                .padding(6)
        }
    }

    @ViewBuilder private var reservedFlag: some View {
        if card.reserved {
            Label("Réservée", systemImage: "lock.fill")
                .font(.caption2)
                .fontWeight(.semibold)
                .foregroundStyle(.purple)
                .padding(.horizontal, 7)
                .padding(.vertical, 3)
                .background(.ultraThinMaterial, in: Capsule())
                .padding(6)
        }
    }

    /// Same rules as `Card/Cell.vue`: the comparison only shows when the swing is meaningful,
    /// otherwise a single price — trend when known, purchase price as a fallback.
    @ViewBuilder private var priceRow: some View {
        if let deal, deal.kind != .par, let trendCents {
            HStack(spacing: 6) {
                priceText(cents: trendCents)
                if let purchaseCents {
                    Text(Price.euros(cents: purchaseCents))
                        .font(.caption2)
                        .monospacedDigit()
                        .strikethrough()
                        .foregroundStyle(.tertiary)
                }
                dealBadge(deal)
            }
        } else if let cents = trendCents ?? purchaseCents {
            priceText(cents: cents)
        }
    }

    private func priceText(cents: Int) -> some View {
        Text(Price.euros(cents: cents))
            .font(.caption)
            .fontWeight(.semibold)
            .monospacedDigit()
    }

    private func dealBadge(_ deal: CardDeal) -> some View {
        let color: Color = deal.kind == .good ? .green : .red
        return Text(deal.label)
            .font(.caption2)
            .fontWeight(.bold)
            .monospacedDigit()
            .foregroundStyle(color)
            .padding(.horizontal, 5)
            .padding(.vertical, 1)
            .background(color.opacity(0.15), in: RoundedRectangle(cornerRadius: 4, style: .continuous))
    }
}

#Preview {
    CollectionCardCell(
        card: CollectionCard(
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
        )
    )
    .frame(width: 170)
    .padding()
}
