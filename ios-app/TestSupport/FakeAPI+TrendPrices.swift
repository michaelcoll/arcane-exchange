import APIClient
import Foundation

extension FakeAPI {
    /// A fake that answers `POST /catalog/trend-prices` with the given trend and foil trend of each
    /// card (by collector number), nil meaning unknown.
    static func trendPrices(_ prices: [String: (Int32?, Int32?)]) -> FakeAPI {
        let api = FakeAPI()
        api.onGetTrendPrices = { input in
            guard case let .json(request) = input.body else { throw URLError(.badServerResponse) }
            return .ok(.init(body: .json(.init(prices: request.cards.map {
                .init(collector_number: $0.collector_number, foil_trend_cents: prices[$0.collector_number]?.1,
                      set_code: $0.set_code, trend_cents: prices[$0.collector_number]?.0)
            }))))
        }
        return api
    }
}
