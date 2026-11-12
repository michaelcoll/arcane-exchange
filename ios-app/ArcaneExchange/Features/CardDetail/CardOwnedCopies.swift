import ArcaneExchangeCore
import SwiftUI

/// What the user owns of this card: how many, what they paid, and how the trend has moved
/// since. Shown only when the card is in their collection.
struct CardOwnedCopies: View {
    let entry: CollectionEntry
    let foil: Bool
    let trendCents: Int?
    let entries: CollectionEntriesViewModel

    @State private var editing: CollectionEntryDetail?

    /// The figures of the card's finish: those of the entries once read, so they follow an edit,
    /// a move or a removal; the card list's until then.
    private var figures: OwnedCopiesFigures? {
        entries.hasLoaded
            ? OwnedCopiesFigures(entries: entries.entries, foil: foil, trendCents: trendCents)
            : OwnedCopiesFigures(entry: entry, trendCents: trendCents)
    }

    var body: some View {
        CardDetailGroup(header: "Dans ma collection") {
            if let figures {
                CardDetailRow("Exemplaires") { Text("×\(figures.quantity)") }
                if let purchase = figures.purchaseCents {
                    Divider()
                    CardDetailRow("Prix d'achat") { Text(Price.euros(cents: purchase)) }
                }
                if let purchase = figures.purchaseCents, let spread = figures.spread, let trendCents {
                    Divider()
                    CardDetailRow("Écart depuis l'achat") {
                        Text(spreadText(to: trendCents, purchase: purchase, spread: spread))
                            .foregroundStyle(spread.kind == .loss ? Palette.down : Palette.primary)
                    }
                }
            }
            ForEach(Array(entries.entries.enumerated()), id: \.element.id) { index, detail in
                if figures != nil || index > 0 {
                    Divider()
                }
                Button { editing = detail } label: {
                    CardDetailRow(OwnedEntryLabel.text(for: detail)) {
                        HStack(spacing: 6) {
                            if let price = OwnedEntryLabel.price(for: detail) {
                                Text(price).foregroundStyle(.secondary)
                            }
                            Text("×\(detail.quantity)")
                            Image(systemName: "chevron.right").font(.caption2).foregroundStyle(.tertiary)
                        }
                    }
                }
                .buttonStyle(.plain)
            }
        }
        .sheet(item: $editing) { detail in
            CollectionEntryEditSheet(entry: detail, model: entries)
        }
    }

    private func spreadText(to trend: Int, purchase: Int, spread: PurchaseSpread) -> String {
        let delta = trend - purchase
        let sign = delta >= 0 ? "+" : "−"
        return "\(sign)\(Price.euros(cents: abs(delta))) · \(abs(spread.percent)) %"
    }
}

extension CollectionEntryDetail: @retroactive Identifiable {}

#Preview {
    CardOwnedCopies(
        entry: .init(added_at: "2026-01-05T10:00:00Z", purchase_price: 980, quantity: 2),
        foil: false,
        trendCents: 1240,
        entries: CollectionEntriesViewModel(card: CollectionCard.previewGrid[0], api: APIClientFactory.preview)
    )
    .padding()
}
