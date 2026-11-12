/// The money figures of the copies the user owns: what they paid, and how the trend has moved
/// since. A copy without a purchase price has neither (ADR-0023): no `0 €`, no fictitious gain.
package struct OwnedCopiesFigures: Equatable {
    package let quantity: Int
    package let purchaseCents: Int?
    package let spread: PurchaseSpread?

    /// From the aggregated entry the card list came with: what the screen shows until the
    /// entries are read.
    package init(entry: CollectionEntry, trendCents: Int?) {
        quantity = Int(entry.quantity)
        purchaseCents = entry.purchase_price.map(Int.init)
        spread = PurchaseSpread(purchaseCents: purchaseCents, trendCents: trendCents)
    }

    /// From the entries read, so an edit, a move or a removal shows at once: the copies of the
    /// finish all binders together, at the average of the prices present weighted by quantity
    /// (the server's merge rule). `nil` when the player has none of the finish.
    package init?(entries: [CollectionEntryDetail], foil: Bool, trendCents: Int?) {
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

/// The words of an entry row: its binder and finish, and what was paid for its copies.
package enum OwnedEntryLabel {
    package static func text(for entry: CollectionEntryDetail) -> String {
        let binder = entry.binder?.name ?? BinderCopy.noBinder
        return entry.foil ? "\(binder) · Foil" : binder
    }

    /// The purchase price of the copies of the entry, `nil` when they have none (ADR-0023).
    package static func price(for entry: CollectionEntryDetail) -> String? {
        entry.purchase_price.map { Price.euros(cents: Int($0)) }
    }
}
