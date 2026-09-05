import SwiftUI

/// The mockup's `TradeRatio` band, shown under the trade rules on the Réglages screen: how the
/// copies owned inside the selected binders split between proposed, kept and closed rarities.
struct TradeRatioBand: View {
    let ratio: TradeRatio

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            GeometryReader { proxy in
                HStack(spacing: 0) {
                    Color.accentColor
                        .frame(width: proxy.size.width * ratio.share(ratio.proposed))
                    Color.secondary
                        .frame(width: proxy.size.width * ratio.share(ratio.kept))
                    Color(.tertiaryLabel)
                        .frame(width: proxy.size.width * ratio.share(ratio.excluded))
                    Spacer(minLength: 0)
                }
            }
            .frame(height: 10)
            .background(Color(.tertiarySystemFill))
            .clipShape(Capsule())

            Text(ratio.summary)
                .font(.footnote)
                .fontWeight(.semibold)

            VStack(alignment: .leading, spacing: 4) {
                legend(color: .accentColor, text: "\(ratio.proposed) proposés")
                legend(color: .secondary, text: "\(ratio.kept) gardés par tes règles")
                legend(color: Color(.tertiaryLabel), text: "\(ratio.excluded) en raretés fermées")
                // No swatch, unlike the three above: it isn't a slice of the band, just the sum.
                Text("\(ratio.total) exemplaires au total")
                    .foregroundStyle(Color(.tertiaryLabel))
            }
            .font(.caption)
            .foregroundStyle(.secondary)
        }
        .padding(.vertical, 6)
    }

    private func legend(color: Color, text: String) -> some View {
        HStack(spacing: 8) {
            RoundedRectangle(cornerRadius: 2)
                .fill(color)
                .frame(width: 8, height: 8)
            Text(text)
        }
    }
}

#Preview {
    List {
        Section("Répartition mixte") {
            TradeRatioBand(ratio: TradeRatio(rarities: [
                .init(copies: 420, is_open: true, kept_copies: 1, proposed: 260, rarity: "C"),
                .init(copies: 180, is_open: true, kept_copies: 2, proposed: 90, rarity: "U"),
                .init(copies: 64, is_open: true, kept_copies: 3, proposed: 12, rarity: "R"),
                .init(copies: 15, is_open: false, kept_copies: 0, proposed: 0, rarity: "M")
            ]))
        }

        Section("Tout proposé") {
            TradeRatioBand(ratio: TradeRatio(rarities: [
                .init(copies: 300, is_open: true, kept_copies: 0, proposed: 300, rarity: "C"),
                .init(copies: 120, is_open: true, kept_copies: 0, proposed: 120, rarity: "U")
            ]))
        }

        Section("Tout fermé") {
            TradeRatioBand(ratio: TradeRatio(rarities: [
                .init(copies: 300, is_open: false, kept_copies: 0, proposed: 0, rarity: "C"),
                .init(copies: 40, is_open: false, kept_copies: 0, proposed: 0, rarity: "M")
            ]))
        }

        Section("Vide") {
            TradeRatioBand(ratio: TradeRatio(rarities: []))
        }
    }
}
