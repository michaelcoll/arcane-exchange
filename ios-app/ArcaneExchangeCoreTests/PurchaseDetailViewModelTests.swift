import APIClient
import Testing

@testable import ArcaneExchangeCore

private func line(
    _ name: String,
    foil: Bool = false,
    quantity: Int32 = 1,
    trend: Int32? = nil
) -> Components.Schemas.PurchaseContentLineResponse {
    .init(
        collector_number: "1",
        foil: foil,
        image_url: nil,
        language_code: "FR",
        name: name,
        quantity: quantity,
        set_code: "FDN",
        trend_cents: trend
    )
}

private func detail(
    _ lines: [Components.Schemas.PurchaseContentLineResponse] = [],
    value: Int64 = 0,
    profitability: Int64 = -499,
    percent: Double = -100,
    withoutTrend: Int32 = 0
) -> Operations.get_purchase.Output {
    .ok(.init(body: .json(.init(
        content: lines,
        content_value_cents: value,
        lines_without_trend: withoutTrend,
        profitability_cents: profitability,
        profitability_percent: percent,
        purchase: purchase("p-1")
    ))))
}

@MainActor
struct PurchaseDetailViewModelTests {
    @Test func showsThePurchaseItWasOpenedOnUntilTheContentIsLoaded() {
        let model = PurchaseDetailViewModel(purchase: purchase("p-1"), api: FakeAPI())

        #expect(model.purchase.id == "p-1")
        #expect(model.detail == nil)
        #expect(!model.hasLoaded)
    }

    @Test func loadsTheContentTheValueAndTheProfitabilityOfThePurchase() async {
        let api = FakeAPI()
        nonisolated(unsafe) var asked: String?
        api.onGetPurchase = { input in
            asked = input.path.id
            return detail(
                [line("Lightning Bolt", quantity: 2, trend: 150), line("Sol Ring", foil: true)],
                value: 300,
                profitability: -199,
                percent: -39.9,
                withoutTrend: 1
            )
        }
        let model = PurchaseDetailViewModel(purchase: purchase("p-1"), api: api)

        await model.load()

        #expect(asked == "p-1")
        #expect(model.hasLoaded)
        #expect(model.detail?.content.map(\.name) == ["Lightning Bolt", "Sol Ring"])
        #expect(model.valueLabel == "3\u{00A0}€")
        #expect(model.profitabilityLabel == "−1,99\u{00A0}€ (−40 %)")
        #expect(model.withoutTrendLabel == "1 carte sans prix de tendance, exclue de la valeur")
    }

    @Test func aContentWhoseEveryCardHasATrendSaysNothingAboutMissingOnes() async {
        let api = FakeAPI()
        api.onGetPurchase = { _ in detail([line("Lightning Bolt", trend: 150)], value: 150) }
        let model = PurchaseDetailViewModel(purchase: purchase("p-1"), api: api)

        await model.load()

        #expect(model.withoutTrendLabel == nil)
    }

    @Test func aFailedLoadShowsTheTranslatedMessageAndNoContent() async {
        let api = FakeAPI()
        api.onGetPurchase = { _ in throw APIError(code: "purchase_not_found", status: 404) }
        let model = PurchaseDetailViewModel(purchase: purchase("p-1"), api: api)

        await model.load()

        #expect(model.loadError == "Cet achat n'existe plus.")
        #expect(model.detail == nil)
        #expect(!model.hasLoaded)
    }

    @Test func aReloadAfterAFailureClearsTheMessage() async {
        let api = FakeAPI()
        api.onGetPurchase = { _ in throw APIError(code: "purchase_not_found", status: 404) }
        let model = PurchaseDetailViewModel(purchase: purchase("p-1"), api: api)
        await model.load()
        api.onGetPurchase = { _ in detail() }

        await model.load()

        #expect(model.loadError == nil)
        #expect(model.hasLoaded)
    }
}
