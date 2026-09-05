import APIClient
import Foundation

/// Backs `CardOffersView`: one page of `GET /card/offers` for the card at hand, plus the
/// "propose an exchange" action each row offers.
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
    /// The owner whose trade is being created, so only that row shows a spinner.
    private(set) var startingWith: String?
    /// Set when starting a trade is refused; the view shows it in an alert and clears it.
    var startError: String?
    /// Trades this screen has already opened, keyed by owner — see `startTrade(with:)`.
    private var openedTrades: [String: TradeDetailRoute] = [:]

    /// The set name once `/sets/{set_code}` resolves; falls back to the raw code.
    private(set) var setName: String
    private(set) var isSetKnown = false

    init(card: CollectionCard) {
        self.card = card
        setName = card.set_code.uppercased()
    }

    /// Resolves the human-readable set name for the card row. A failure keeps the code.
    func loadSetName() async {
        if let name = await SetName.resolve(card.set_code) {
            setName = name
            isSetKnown = true
        }
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
}
