import APIClient
import Foundation

/// Backs `CardDetailView`: one `GET /card/{scryfall_id}/price-history` call for the chart.
/// The price guide and collection figures come from the `CollectionCard` the list passed in,
/// so nothing else needs fetching here.
@MainActor
@Observable
final class CardDetailViewModel {
    /// What the chart area shows. The distinction the mockup makes is "loading" vs "not enough
    /// history yet" vs a real chart; a failed request folds into a plain unavailable message.
    enum HistoryState: Equatable {
        case loading
        case ready([PricePoint])
        case notEnoughData
        case failed
    }

    let card: CollectionCard
    private(set) var history: HistoryState = .loading

    init(card: CollectionCard) {
        self.card = card
    }

    /// Loads the default window (the API falls back to the last 30 days on its own).
    func loadHistory() async {
        history = .loading
        do {
            let output = try await APIClientProvider.shared.get_card_price_history(
                path: .init(scryfall_id: card.scryfall_id)
            )
            switch output {
            case let .ok(response):
                let points = try PriceHistorySeries.points(from: response.body.json)
                history = points.count >= 2 ? .ready(points) : .notEnoughData
            case .badRequest, .notFound:
                // A bad date range or an unknown card here just means "nothing to plot".
                history = .notEnoughData
            case .unauthorized, .undocumented:
                history = .failed
            }
        } catch {
            history = .failed
        }
    }
}
