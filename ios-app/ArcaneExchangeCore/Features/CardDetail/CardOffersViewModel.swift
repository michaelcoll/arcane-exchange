import APIClient
import Foundation

/// Backs `CardOffersView`: one page of `GET /card/offers` for the card at hand, plus the
/// "propose an exchange" action each row offers.
@MainActor
@Observable
package final class CardOffersViewModel {
    package enum State: Equatable {
        case loading
        case loaded([CardOffer])
        case failed
    }

    package let card: CollectionCard
    package private(set) var state: State = .loading
    /// Opens a trade from an offer row; the view reads its spinner and error from here.
    package let tradeOpener: CardOfferTrade

    /// The set name once `/sets/{set_code}` resolves; falls back to the raw code.
    package private(set) var setName: String
    package private(set) var isSetKnown = false

    private let api: any APIProtocol

    package init(card: CollectionCard, api: any APIProtocol) {
        self.card = card
        self.api = api
        tradeOpener = CardOfferTrade(card: card, api: api)
        setName = card.set_code.uppercased()
    }

    /// Resolves the human-readable set name for the card row. A failure keeps the code.
    package func loadSetName() async {
        if let name = await SetName.resolve(card.set_code, api: api) {
            setName = name
            isSetKnown = true
        }
    }

    package func load() async {
        state = .loading
        do {
            let page = try await api.get_card_offers(
                query: .init(
                    set_code: card.set_code,
                    collector_number: card.collector_number,
                    language_code: card.language_code,
                    foil: card.foil,
                    sort_by: .selling_price,
                    page: 0,
                    page_size: 20
                )
            ).ok.body.json
            state = .loaded(page.items)
        } catch {
            // An unknown card has nobody offering it, which is not an error.
            state = APIError.read(error).code == "card_not_found" ? .loaded([]) : .failed
        }
    }
}
