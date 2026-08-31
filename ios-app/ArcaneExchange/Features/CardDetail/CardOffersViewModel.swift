import APIClient
import Foundation

/// Backs `CardOffersView`: one page of `GET /card/offers` for the card at hand. Read-only —
/// starting a trade from an offer is a later slice.
@MainActor
@Observable
final class CardOffersViewModel {
    enum State: Equatable {
        case loading
        case loaded([CardOffer])
        case failed
    }

    let card: CollectionCard
    private(set) var state: State = .loading

    init(card: CollectionCard) {
        self.card = card
    }

    func load() async {
        state = .loading
        do {
            let output = try await APIClientProvider.shared.get_card_offers(
                query: .init(
                    set_code: card.set_code,
                    collector_number: card.collector_number,
                    language_code: card.language_code,
                    foil: card.foil,
                    sort_by: .selling_price,
                    page: 0,
                    page_size: 20
                )
            )
            switch output {
            case let .ok(response):
                state = try .loaded(response.body.json.items)
            case .badRequest, .notFound:
                // Nothing to show rather than an error screen.
                state = .loaded([])
            case .unauthorized, .undocumented:
                state = .failed
            }
        } catch {
            state = .failed
        }
    }
}
