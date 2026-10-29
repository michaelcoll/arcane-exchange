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
    /// I abandoned the trade. Neither flag is set for an automatic abandon.
    let meAbandoned: Bool
    let partnerAbandoned: Bool

    init(
        status: TradeStatus, meAccepted: Bool, meConfirmed: Bool, myRating: Int?,
        meAbandoned: Bool, partnerAbandoned: Bool
    ) {
        self.status = status
        self.meAccepted = meAccepted
        self.meConfirmed = meConfirmed
        self.myRating = myRating
        self.meAbandoned = meAbandoned
        self.partnerAbandoned = partnerAbandoned
    }

    init(trade: TradeDetail) {
        self.init(
            status: TradeStatus(apiValue: trade.status),
            meAccepted: trade.me.accepted,
            meConfirmed: trade.me.confirmed,
            myRating: trade.me.rating.map(Int.init),
            meAbandoned: trade.me.abandoned,
            partnerAbandoned: trade.partner.abandoned
        )
    }

    var primaryAction: TradePrimaryAction {
        switch status {
        case .pending, .oneAccepted: meAccepted ? .awaitingAcceptance : .accept
        case .fullyAccepted: meConfirmed ? .awaitingConfirmation : .confirmExchange
        case .completed, .closed, .abandoned: .none
        }
    }

    /// A trade not loaded yet: it offers no action. An abandoned trade with nobody to rate is the
    /// one state that offers nothing.
    static let none = TradeActions(
        status: .abandoned, meAccepted: false, meConfirmed: false, myRating: nil,
        meAbandoned: false, partnerAbandoned: false
    )

    /// Before the exchange is confirmed; after that, the backend answers 409.
    var canAbandon: Bool {
        status.isActive
    }

    /// Cards can still be added or removed.
    var canEdit: Bool {
        status == .pending || status == .oneAccepted
    }

    /// Removing a card once one party accepted restarts the negotiation.
    var warnsOnRemove: Bool {
        status == .oneAccepted
    }

    /// After an exchange, or a voluntary abandon: an automatic abandon has nobody to rate.
    var showsRating: Bool {
        switch status {
        case .completed, .closed: true
        case .abandoned: meAbandoned || partnerAbandoned
        case .pending, .oneAccepted, .fullyAccepted: false
        }
    }

    /// Once only. After an abandon, only the party who did not abandon can rate.
    var canRate: Bool {
        guard myRating == nil else { return false }
        return status == .completed || (status == .abandoned && partnerAbandoned)
    }

    /// Both Notes close the Trade after an exchange; an abandoned Trade stays abandoned.
    var closesTradeOnRating: Bool {
        status != .abandoned
    }

    /// I abandoned the trade: I cannot rate, only my partner can.
    var awaitsPartnerRating: Bool {
        status == .abandoned && meAbandoned
    }
}
