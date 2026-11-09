import SwiftUI

/// The money figures of the copies the user owns: what they paid, and how the trend has moved
/// since. A copy without a purchase price has neither (ADR-0023): no `0 €`, no fictitious gain.
struct OwnedCopiesFigures: Equatable {
    let quantity: Int
    let purchaseCents: Int?
    let spread: PurchaseSpread?

    /// From the aggregated entry the card list came with: what the screen shows until the
    /// entries are read.
    init(entry: CollectionEntry, trendCents: Int?) {
        quantity = Int(entry.quantity)
        purchaseCents = entry.purchase_price.map(Int.init)
        spread = PurchaseSpread(purchaseCents: purchaseCents, trendCents: trendCents)
    }

    /// From the entries read, so an edit, a move or a removal shows at once: the copies of the
    /// finish all binders together, at the average of the prices present weighted by quantity
    /// (the server's merge rule). `nil` when the player has none of the finish.
    init?(entries: [CollectionEntryDetail], foil: Bool, trendCents: Int?) {
        let ofFinish = entries.filter { $0.foil == foil }
        guard !ofFinish.isEmpty else { return nil }

        quantity = ofFinish.reduce(0) { $0 + Int($1.quantity) }
        let priced = ofFinish.compactMap { entry in
            entry.purchase_price.map { (price: Int($0), quantity: Int(entry.quantity)) }
        }
        let pricedQuantity = priced.reduce(0) { $0 + $1.quantity }
        purchaseCents = pricedQuantity > 0
            ? priced.reduce(0) { $0 + $1.price * $1.quantity } / pricedQuantity
            : nil
        spread = PurchaseSpread(purchaseCents: purchaseCents, trendCents: trendCents)
    }
}

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

/// The words of an entry row: its binder and finish, and what was paid for its copies.
enum OwnedEntryLabel {
    static func text(for entry: CollectionEntryDetail) -> String {
        let binder = entry.binder?.name ?? BinderCopy.noBinder
        return entry.foil ? "\(binder) · Foil" : binder
    }

    /// The purchase price of the copies of the entry, `nil` when they have none (ADR-0023).
    static func price(for entry: CollectionEntryDetail) -> String? {
        entry.purchase_price.map { Price.euros(cents: Int($0)) }
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
