import APIClient
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
    /// The owner whose trade is being created, so only that row shows a spinner.
    private(set) var startingWith: String?
    /// Set when starting a trade is refused; the view shows it in an alert and clears it.
    var startError: String?
    /// Trades this screen has already opened, keyed by owner — see `startTrade(with:)`.
    private var openedTrades: [String: TradeDetailRoute] = [:]

    /// The set name once `/sets/{set_code}` resolves; falls back to the raw code
    /// (mirrors `frontend-vue`'s `DetailModal.vue` `setName`/`isSetKnown`).
    private(set) var setName: String
    private(set) var isSetKnown = false

    init(card: CollectionCard) {
        self.card = card
        setName = card.set_code.uppercased()
    }

    /// Resolves the human-readable set name. A failure just keeps showing the code.
    func loadSetName() async {
        if let name = await SetName.resolve(card.set_code) {
            setName = name
            isSetKnown = true
        }
    }

    /// Loads just the cheapest few offers; the header count comes from the page's `total`.
    func loadOffers() async {
        do {
            let output = try await APIClientProvider.shared.get_card_offers(
                query: .init(
                    set_code: card.set_code,
                    collector_number: card.collector_number,
                    language_code: card.language_code,
                    foil: card.foil,
                    sort_by: .selling_price,
                    page: 0,
                    page_size: Int32(Self.previewCount)
                )
            )
            switch output {
            case let .ok(response):
                let page = try response.body.json
                offers = .loaded(preview: page.items, total: Int(page.total))
            case .badRequest, .notFound:
                // Nobody to show rather than an error.
                offers = .loaded(preview: [], total: 0)
            case .unauthorized, .undocumented:
                offers = .failed
            }
        } catch {
            offers = .failed
        }
    }

    /// Opens (or reuses) the trade with this offer's owner, this card already on their side.
    ///
    /// A second tap on a row this screen already acted on just reopens that trade. The offers
    /// are fetched once — the stack keeps this screen alive, so coming back from the trade
    /// leaves the row still reading "Échanger" — and `POST /trades/{id}/cards` *adds* to the
    /// quantity already on the table, so calling it again would either slip in a second copy
    /// or fail with "ce joueur ne propose plus assez de copies".
    func startTrade(with offer: CardOffer) async -> TradeDetailRoute? {
        if let opened = openedTrades[offer.owner_username] {
            return opened
        }
        guard startingWith == nil else { return nil }
        startingWith = offer.owner_username
        defer { startingWith = nil }
        switch await CardOfferTrade.start(card: card, with: offer) {
        case let .success(route):
            openedTrades[offer.owner_username] = route
            return route
        case let .failure(refusal):
            startError = refusal.message
            return nil
        }
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
