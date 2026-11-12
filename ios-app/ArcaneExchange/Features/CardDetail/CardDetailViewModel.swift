import APIClient
import ArcaneCore
import Foundation

/// Backs `CardDetailView`: the chart's price history, plus the first few players offering
/// the card — the mockup shows three of them inline before the link to the full list.
/// The price guide and collection figures come from the `CollectionCard` the list passed in.
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

    /// The inline owners list. `total` is the whole count behind the section header, not the
    /// handful of rows shown.
    enum OffersState: Equatable {
        case loading
        case loaded(preview: [CardOffer], total: Int)
        case failed
    }

    /// How many offers the card screen shows before "Voir tous les possesseurs".
    static let previewCount = 3

    let card: CollectionCard
    private(set) var history: HistoryState = .loading
    private(set) var offers: OffersState = .loading
    /// Opens a trade from an offer row; the view reads its spinner and error from here.
    let tradeOpener: CardOfferTrade

    /// The set name once `/sets/{set_code}` resolves; falls back to the raw code
    /// (mirrors `frontend/`'s `DetailModal.vue` `setName`/`isSetKnown`).
    private(set) var setName: String
    private(set) var isSetKnown = false

    private let api: any APIProtocol

    init(card: CollectionCard, api: any APIProtocol) {
        self.card = card
        self.api = api
        tradeOpener = CardOfferTrade(card: card, api: api)
        setName = card.set_code.uppercased()
    }

    /// Resolves the human-readable set name. A failure just keeps showing the code.
    func loadSetName() async {
        if let name = await SetName.resolve(card.set_code, api: api) {
            setName = name
            isSetKnown = true
        }
    }

    /// Loads just the cheapest few offers; the header count comes from the page's `total`.
    func loadOffers() async {
        do {
            let page = try await api.get_card_offers(
                query: .init(
                    set_code: card.set_code,
                    collector_number: card.collector_number,
                    language_code: card.language_code,
                    foil: card.foil,
                    sort_by: .selling_price,
                    page: 0,
                    page_size: Int32(Self.previewCount)
                )
            ).ok.body.json
            offers = .loaded(preview: page.items, total: Int(page.total))
        } catch {
            // An unknown card has nobody offering it, which is not an error.
            offers = APIError.read(error).code == "card_not_found" ? .loaded(preview: [], total: 0) : .failed
        }
    }

    /// Loads the default window (the API falls back to the last 30 days on its own).
    func loadHistory() async {
        history = .loading
        do {
            let entries = try await api.get_card_price_history(
                path: .init(scryfall_id: card.scryfall_id),
                query: .init(foil: card.foil)
            ).ok.body.json
            let points = PriceHistorySeries.points(from: entries)
            history = points.count >= 2 ? .ready(points) : .notEnoughData
        } catch {
            // An unknown card here just means "nothing to plot".
            history = APIError.read(error).code == "card_not_found" ? .notEnoughData : .failed
        }
    }
}
