import Testing

@testable import ArcaneExchange

/// One row of the truth table: a status and my side of the trade.
private struct Case: CustomTestStringConvertible {
    let status: TradeStatus
    let meAccepted: Bool
    let meConfirmed: Bool
    let myRating: Int?

    var actions: TradeActions {
        TradeActions(status: status, meAccepted: meAccepted, meConfirmed: meConfirmed, myRating: myRating)
    }

    var testDescription: String {
        "\(status) accepted=\(meAccepted) confirmed=\(meConfirmed) rating=\(String(describing: myRating))"
    }
}

/// Every status against every state of my side: accepted or not, confirmed or not, rated
/// (including a skipped rating, 0) or not.
private let allCases: [Case] = TradeStatus.allCases.flatMap { status in
    [false, true].flatMap { accepted in
        [false, true].flatMap { confirmed in
            [nil, 0, 4].map { rating in
                Case(status: status, meAccepted: accepted, meConfirmed: confirmed, myRating: rating)
            }
        }
    }
}

struct TradeActionsTests {
    @Test(arguments: allCases)
    private func primaryActionFollowsTheStatusAndMySide(_ row: Case) {
        let expected: TradePrimaryAction = switch row.status {
        case .pending, .oneAccepted: row.meAccepted ? .awaitingAcceptance : .accept
        case .fullyAccepted: row.meConfirmed ? .awaitingConfirmation : .confirmExchange
        case .completed, .closed, .abandoned: .none
        }
        #expect(row.actions.primaryAction == expected)
    }

    @Test(arguments: allCases)
    private func abandoningIsOnlyPossibleBeforeTheExchangeIsConfirmed(_ row: Case) {
        let expected = [TradeStatus.pending, .oneAccepted, .fullyAccepted].contains(row.status)
        #expect(row.actions.canAbandon == expected)
    }

    @Test(arguments: allCases)
    private func cardsCanBeEditedOnlyWhileNegotiating(_ row: Case) {
        #expect(row.actions.canEdit == (row.status == .pending || row.status == .oneAccepted))
    }

    @Test(arguments: allCases)
    private func removingACardWarnsOnlyOnceOnePartyAccepted(_ row: Case) {
        #expect(row.actions.warnsOnRemove == (row.status == .oneAccepted))
    }

    @Test(arguments: allCases)
    private func theRatingShowsOnceTheExchangeTookPlace(_ row: Case) {
        #expect(row.actions.showsRating == (row.status == .completed || row.status == .closed))
    }

    @Test(arguments: allCases)
    private func iCanRateACompletedTradeOnceOnly(_ row: Case) {
        #expect(row.actions.canRate == (row.status == .completed && row.myRating == nil))
    }

    @Test func readsMySideFromTheTradeDetail() {
        let trade = TradeDetail(
            id: "t1",
            me: .init(accepted: true, confirmed: false),
            my_cards: [],
            partner: .init(accepted: false, confirmed: false),
            partner_cards: [],
            partner_username: "mizzix_42",
            status: "ONE_ACCEPTED"
        )
        #expect(TradeActions(trade: trade).primaryAction == .awaitingAcceptance)
    }
}
