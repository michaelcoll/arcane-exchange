import Testing

@testable import ArcaneCore

/// One row of the truth table: a status and my side of the trade.
private struct Case: CustomTestStringConvertible {
    let status: TradeStatus
    let meAccepted: Bool
    let meConfirmed: Bool
    let myRating: Int?
    let abandonedBy: Abandoner

    var actions: TradeActions {
        TradeActions(
            status: status,
            meAccepted: meAccepted,
            meConfirmed: meConfirmed,
            myRating: myRating,
            meAbandoned: abandonedBy == .me,
            partnerAbandoned: abandonedBy == .partner
        )
    }

    var testDescription: String {
        "\(status) accepted=\(meAccepted) confirmed=\(meConfirmed) rating=\(String(describing: myRating)) abandonedBy=\(abandonedBy)"
    }
}

/// Who abandoned the trade: me, my partner, or nobody (an automatic abandon, or not abandoned).
private enum Abandoner: CaseIterable {
    case nobody, me, partner
}

/// Every status against every state of my side: accepted or not, confirmed or not, rated
/// (including a skipped rating, 0) or not, abandoned by me, by my partner or by nobody.
private let allCases: [Case] = TradeStatus.allCases.flatMap { status in
    [false, true].flatMap { accepted in
        [false, true].flatMap { confirmed in
            [nil, 0, 4].flatMap { rating in
                Abandoner.allCases.map { abandonedBy in
                    Case(
                        status: status, meAccepted: accepted, meConfirmed: confirmed,
                        myRating: rating, abandonedBy: abandonedBy
                    )
                }
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
    private func theRatingShowsOnceTheExchangeTookPlaceOrAPartyAbandoned(_ row: Case) {
        let expected = row.status == .completed || row.status == .closed
            || (row.status == .abandoned && row.abandonedBy != .nobody)
        #expect(row.actions.showsRating == expected)
    }

    @Test(arguments: allCases)
    private func iCanRateOnceOnlyACompletedTradeOrOneMyPartnerAbandoned(_ row: Case) {
        let expected = row.myRating == nil
            && (row.status == .completed || (row.status == .abandoned && row.abandonedBy == .partner))
        #expect(row.actions.canRate == expected)
    }

    @Test(arguments: allCases)
    private func ratingClosesTheTradeOnlyWhenTheExchangeTookPlace(_ row: Case) {
        #expect(row.actions.closesTradeOnRating == (row.status != .abandoned))
    }

    @Test(arguments: allCases)
    private func onlyTheAbandonerWaitsForTheirPartnerToRate(_ row: Case) {
        #expect(row.actions.awaitsPartnerRating == (row.status == .abandoned && row.abandonedBy == .me))
    }

    @Test func aTradeNotLoadedYetOffersNothing() {
        let none = TradeActions.none
        #expect(none.primaryAction == .none)
        #expect(!none.canAbandon)
        #expect(!none.canEdit)
        #expect(!none.warnsOnRemove)
        #expect(!none.showsRating)
        #expect(!none.canRate)
    }

    @Test func readsMySideFromTheTradeDetail() {
        let trade = TradeDetail(
            id: "t1",
            me: .init(abandoned: false, accepted: true, confirmed: false),
            my_cards: [],
            partner: .init(abandoned: false, accepted: false, confirmed: false),
            partner_cards: [],
            partner_username: "mizzix_42",
            status: "ONE_ACCEPTED"
        )
        #expect(TradeActions(trade: trade).primaryAction == .awaitingAcceptance)
    }
}
