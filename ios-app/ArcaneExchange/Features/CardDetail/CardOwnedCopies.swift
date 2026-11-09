import SwiftUI

/// The money figures of the copies the user owns: what they paid, and how the trend has moved
/// since. A copy without a purchase price has neither (ADR-0023): no `0 €`, no fictitious gain.
struct OwnedCopiesFigures: Equatable {
    let purchaseCents: Int?
    let spread: PurchaseSpread?

    init(entry: CollectionEntry, trendCents: Int?) {
        purchaseCents = entry.purchase_price.map(Int.init)
        spread = PurchaseSpread(purchaseCents: purchaseCents, trendCents: trendCents)
    }
}

/// What the user owns of this card: how many, what they paid, and how the trend has moved
/// since. Shown only when the card is in their collection.
struct CardOwnedCopies: View {
    let entry: CollectionEntry
    let trendCents: Int?

    private var figures: OwnedCopiesFigures {
        OwnedCopiesFigures(entry: entry, trendCents: trendCents)
    }

    var body: some View {
        CardDetailGroup(header: "Dans ma collection") {
            CardDetailRow("Exemplaires") { Text("×\(entry.quantity)") }
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
    }

    private func spreadText(to trend: Int, purchase: Int, spread: PurchaseSpread) -> String {
        let delta = trend - purchase
        let sign = delta >= 0 ? "+" : "−"
        return "\(sign)\(Price.euros(cents: abs(delta))) · \(abs(spread.percent)) %"
    }
}

#Preview {
    CardOwnedCopies(
        entry: .init(added_at: "2026-01-05T10:00:00Z", purchase_price: 980, quantity: 2),
        trendCents: 1240
    )
    .padding()
}
