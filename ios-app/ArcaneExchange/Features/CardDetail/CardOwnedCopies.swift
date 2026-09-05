import SwiftUI

/// What the user owns of this card: how many, what they paid, and how the trend has moved
/// since. Shown only when the card is in their collection.
struct CardOwnedCopies: View {
    let entry: CollectionEntry
    let trendCents: Int?

    private var purchase: Int {
        Int(entry.purchase_price)
    }

    private var deal: CardDeal? {
        CardDeal(purchaseCents: purchase, trendCents: trendCents)
    }

    var body: some View {
        CardDetailGroup(header: "Dans ma collection") {
            CardDetailRow("Exemplaires") { Text("×\(entry.quantity)") }
            Divider()
            CardDetailRow("Prix d'achat") { Text(Price.euros(cents: purchase)) }
            if let deal, let trendCents {
                Divider()
                CardDetailRow("Écart depuis l'achat") {
                    Text(spread(to: trendCents, deal: deal))
                        .foregroundStyle(deal.kind == .bad ? Color.red : Color.accentColor)
                }
            }
        }
    }

    private func spread(to trend: Int, deal: CardDeal) -> String {
        let delta = trend - purchase
        let sign = delta >= 0 ? "+" : "−"
        return "\(sign)\(Price.euros(cents: abs(delta))) · \(abs(deal.percent)) %"
    }
}

#Preview {
    CardOwnedCopies(
        entry: .init(added_at: "2026-01-05T10:00:00Z", purchase_price: 980, quantity: 2),
        trendCents: 1240
    )
    .padding()
}
