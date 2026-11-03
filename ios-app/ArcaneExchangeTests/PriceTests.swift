import Testing

@testable import ArcaneExchange

struct PriceTests {
    @Test func dropsDecimalsOnlyWhenThereAreNone() {
        #expect(Price.euros(cents: 3100).hasPrefix("31"))
        #expect(Price.euros(cents: 3150).contains("31,5"))
        #expect(Price.euros(cents: 0).hasPrefix("0"))
    }
}

struct PurchaseSpreadTests {
    @Test func needsBothAPurchasePriceAndATrendPrice() {
        #expect(PurchaseSpread(purchaseCents: nil, trendCents: 3100) == nil)
        #expect(PurchaseSpread(purchaseCents: 2700, trendCents: nil) == nil)
        // A free card would make the percentage meaningless.
        #expect(PurchaseSpread(purchaseCents: 0, trendCents: 3100) == nil)
    }

    @Test func treatsSwingsUnderThreePercentAsNoise() {
        #expect(PurchaseSpread(purchaseCents: 10000, trendCents: 10200)?.kind == .par)
        #expect(PurchaseSpread(purchaseCents: 10000, trendCents: 9800)?.kind == .par)
    }

    @Test func flagsRealGainsAndLosses() {
        let gain = PurchaseSpread(purchaseCents: 2700, trendCents: 3100)
        #expect(gain?.kind == .gain)
        #expect(gain?.percent == 15)
        #expect(gain?.label == "+15%")

        let loss = PurchaseSpread(purchaseCents: 3100, trendCents: 2700)
        #expect(loss?.kind == .loss)
        #expect(loss?.label == "−13%")
    }
}
