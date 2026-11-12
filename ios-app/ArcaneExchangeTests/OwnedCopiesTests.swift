import Testing

@testable import ArcaneCore
@testable import ArcaneExchange

struct OwnedCopiesFiguresTests {
    private func entry(purchasePrice: Int32?) -> CollectionEntry {
        .init(added_at: "2026-01-05T10:00:00Z", purchase_price: purchasePrice, quantity: 2)
    }

    @Test func showsThePurchasePriceAndTheSpreadWhenThereIsOne() {
        let figures = OwnedCopiesFigures(entry: entry(purchasePrice: 980), trendCents: 1240)
        #expect(figures.purchaseCents == 980)
        #expect(figures.spread?.kind == .gain)
    }

    @Test func showsNeitherAPurchasePriceNorASpreadWhenTheCopiesHaveNone() {
        let figures = OwnedCopiesFigures(entry: entry(purchasePrice: nil), trendCents: 1240)
        #expect(figures.purchaseCents == nil)
        #expect(figures.spread == nil)
    }

    private func detail(_ id: String, foil: Bool = false, quantity: Int32, price: Int32?) -> CollectionEntryDetail {
        CollectionEntryDetail(
            added_at: "2026-01-05T10:00:00Z",
            foil: foil,
            id: id,
            purchase_price: price,
            quantity: quantity
        )
    }

    @Test func derivesTheQuantityAndTheAveragePriceFromTheEntriesOfTheFinish() {
        let entries = [
            detail("a", quantity: 2, price: 1000),
            detail("b", quantity: 1, price: 400),
            detail("c", quantity: 3, price: nil),
            detail("d", foil: true, quantity: 5, price: 9999),
        ]

        let figures = OwnedCopiesFigures(entries: entries, foil: false, trendCents: 1240)

        #expect(figures?.quantity == 6)
        // (2 × 1000 + 1 × 400) / 3: the copies without a price stay out of the average.
        #expect(figures?.purchaseCents == 800)
        #expect(figures?.spread?.kind == .gain)
    }

    @Test func hasNoFiguresWhenNoEntryIsOfTheFinish() {
        let entries = [detail("d", foil: true, quantity: 5, price: 9999)]

        #expect(OwnedCopiesFigures(entries: entries, foil: false, trendCents: 1240) == nil)
        #expect(OwnedCopiesFigures(entries: [], foil: false, trendCents: 1240) == nil)
    }
}

struct OwnedEntryLabelTests {
    private func detail(price: Int32?) -> CollectionEntryDetail {
        CollectionEntryDetail(
            added_at: "2026-01-05T10:00:00Z",
            foil: false,
            id: "a",
            purchase_price: price,
            quantity: 2
        )
    }

    @Test func showsThePurchasePriceOfTheEntry() {
        #expect(OwnedEntryLabel.price(for: detail(price: 980)) == Price.euros(cents: 980))
    }

    @Test func showsNothingWhenTheEntryHasNoPurchasePrice() {
        #expect(OwnedEntryLabel.price(for: detail(price: nil)) == nil)
    }
}
