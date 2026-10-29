import APIClient
import Foundation

/// Backs `TradeDetailView`: `GET /trades/{id}` plus the five lifecycle actions.
///
/// Every action ends with a reload rather than a local mutation. The backend owns the state
/// machine — accepting can flip the trade to `FULLY_ACCEPTED`, removing a card can knock it
/// back to `PENDING` and release the reservations — so guessing the next state here would
/// only be a second, less reliable implementation of `trade-workflow.instructions.md`.
@MainActor
@Observable
final class TradeDetailViewModel {
    let tradeID: String

    private(set) var trade: TradeDetail?
    private(set) var isLoading = false
    /// French message of the last failed load, ready to show.
    private(set) var loadError: String?
    /// An action is in flight — the action bar disables itself rather than queueing taps.
    private(set) var isBusy = false
    /// Set when an action is refused; the view shows it in an alert and clears it.
    var actionError: String?

    private let api: any APIProtocol

    init(tradeID: String, api: any APIProtocol) {
        self.tradeID = tradeID
        self.api = api
    }

    var status: TradeStatus {
        TradeStatus(apiValue: trade?.status ?? "PENDING")
    }

    var partnerUsername: String {
        trade?.partner_username ?? ""
    }

    /// Cards leaving the user's collection — asked for by the partner, so not removable here.
    var myCards: [TradeCard] {
        trade?.my_cards ?? []
    }

    /// Cards the user asked the partner for.
    var partnerCards: [TradeCard] {
        trade?.partner_cards ?? []
    }

    var balance: TradeBalance {
        TradeBalance(give: myCards, get: partnerCards)
    }

    var meAccepted: Bool {
        trade?.me.accepted ?? false
    }

    var meConfirmed: Bool {
        trade?.me.confirmed ?? false
    }

    var myRating: Int? {
        trade?.me.rating.map(Int.init)
    }

    var partnerRating: Int? {
        trade?.partner.rating.map(Int.init)
    }

    func load() async {
        isLoading = true
        loadError = nil
        do {
            trade = try await api.get_trade(path: .init(trade_id: tradeID)).ok.body.json
        } catch {
            loadError = APIError.read(error).message
        }
        isLoading = false
    }

    func accept() async {
        await run {
            _ = try await self.api.accept_trade(path: .init(trade_id: self.tradeID)).noContent
        }
    }

    func abandon() async {
        await run {
            _ = try await self.api.abandon_trade(path: .init(trade_id: self.tradeID)).noContent
        }
    }

    func confirmExchange() async {
        await run {
            _ = try await self.api.confirm_trade(path: .init(trade_id: self.tradeID)).noContent
        }
    }

    /// 0 to 5 stars. Zero is the mockup's "passer la notation": the trade still closes, the
    /// partner's average is simply left untouched by a skipped rating.
    func rate(_ rating: Int) async {
        await run {
            _ = try await self.api.rate_trade(
                path: .init(trade_id: self.tradeID),
                body: .json(.init(rating: Int32(rating)))
            ).noContent
        }
    }

    /// Drops one of the partner's cards from the trade. Only the "je reçois" side is editable
    /// from this screen — the cards the partner asked for are theirs to change.
    func removePartnerCard(_ card: TradeCard) async {
        let owner = partnerUsername
        await run {
            _ = try await self.api.remove_trade_card(
                path: .init(trade_id: self.tradeID),
                body: .json(.init(
                    collector_number: card.collector_number,
                    foil: card.foil,
                    language_code: card.language_code,
                    owner_username: owner,
                    set_code: card.set_code
                ))
            ).noContent
        }
    }

    /// Runs one action, then reloads whatever the backend made of it. A refusal (409 above
    /// all: the trade moved on in another session) surfaces as `actionError`, translated from
    /// the API's code.
    private func run(_ action: @escaping () async throws -> Void) async {
        guard !isBusy else { return }
        isBusy = true
        do {
            try await action()
            actionError = nil
        } catch {
            actionError = APIError.read(error).message
        }
        await load()
        isBusy = false
        HomeWidgetReload.reload(.trades)
    }
}
