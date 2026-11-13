import APIClient
import Foundation

package typealias PurchaseDetail = Components.Schemas.PurchaseDetailResponse
package typealias PurchaseContentLine = Components.Schemas.PurchaseContentLineResponse

/// Backs the detail of a sealed product (`GET /purchases/{id}`): what it contained, the value of
/// that content at today's trend prices, and how it compares with the price paid.
@MainActor
@Observable
package final class PurchaseDetailViewModel {
    private let api: any APIProtocol

    /// The purchase the screen was opened on, shown until the detail is loaded.
    package let purchase: Purchase
    package private(set) var detail: PurchaseDetail?
    /// Set when the load failed. French message, ready to show.
    package private(set) var loadError: String?

    package init(purchase: Purchase, api: any APIProtocol) {
        self.purchase = purchase
        self.api = api
    }

    package var hasLoaded: Bool {
        detail != nil
    }

    package func load() async {
        loadError = nil
        do {
            detail = try await api.get_purchase(path: .init(id: purchase.id)).ok.body.json
        } catch {
            detail = nil
            loadError = APIError.read(error).message
        }
    }

    /// The value of the content: the cards without a trend price count for nothing.
    package var valueLabel: String? {
        detail.map { Price.euros(cents: Int($0.content_value_cents)) }
    }

    /// The value minus the price paid, in euros and in percent.
    package var profitabilityLabel: String? {
        detail.map {
            PurchaseCopy.profitability(cents: Int($0.profitability_cents), percent: $0.profitability_percent)
        }
    }

    /// How many cards are left out of the value; nil when none is.
    package var withoutTrendLabel: String? {
        detail.flatMap { PurchaseCopy.withoutTrend(Int($0.lines_without_trend)) }
    }
}
