/// The one step the action bar offers, same cases as the web's `TradePrimaryAction`.
enum TradePrimaryAction: Equatable {
    case accept
    case awaitingAcceptance
    case confirmExchange
    case awaitingConfirmation
    case none
}

/// What the player can do on a trade, from its status and the state of their own side — the
/// single place the screen reads instead of testing the status itself. Mirrors
/// `.agents/trade-workflow.instructions.md`, the web's `isTradeAbandonable` and `useTradeView`,
/// and the backend's `abandon`.
struct TradeActions: Equatable {
    let status: TradeStatus
    let meAccepted: Bool
    let meConfirmed: Bool
    /// My own rating of the partner; 0 is a skipped rating.
    let myRating: Int?

    init(status: TradeStatus, meAccepted: Bool, meConfirmed: Bool, myRating: Int?) {
        self.status = status
        self.meAccepted = meAccepted
        self.meConfirmed = meConfirmed
        self.myRating = myRating
    }

    init(trade: TradeDetail) {
        self.init(
            status: TradeStatus(apiValue: trade.status),
            meAccepted: trade.me.accepted,
            meConfirmed: trade.me.confirmed,
            myRating: trade.me.rating.map(Int.init)
        )
    }

    var primaryAction: TradePrimaryAction {
        switch status {
        case .pending, .oneAccepted: meAccepted ? .awaitingAcceptance : .accept
        case .fullyAccepted: meConfirmed ? .awaitingConfirmation : .confirmExchange
        case .completed, .closed, .abandoned: .none
        }
    }

    /// Before the exchange is confirmed; after that, the backend answers 409.
    var canAbandon: Bool {
        switch status {
        case .pending, .oneAccepted, .fullyAccepted: true
        case .completed, .closed, .abandoned: false
        }
    }

    /// Cards can still be added or removed.
    var canEdit: Bool {
        status == .pending || status == .oneAccepted
    }

    /// Removing a card once one party accepted restarts the negotiation.
    var warnsOnRemove: Bool {
        status == .oneAccepted
    }

    var showsRating: Bool {
        status == .completed || status == .closed
    }

    var canRate: Bool {
        status == .completed && myRating == nil
    }
}
