import Testing

@testable import ArcaneExchange

private func card(trend: Int32?, quantity: Int32 = 1) -> TradeCard {
    TradeCard(
        collector_number: "243",
        foil: false,
        language_code: "fr",
        name: "The Soul Stone",
        price_guide: trend.map { .init(avg: nil, low: nil, trend: $0) },
        quantity: quantity,
        scryfall_id: "7a79190f-de60-4eb6-b925-594eb76ca8c3",
        set_code: "FIN"
    )
}

struct TradeStatusTests {
    @Test func narrowsTheApiStringAndFallsBackToPending() {
        #expect(TradeStatus(apiValue: "FULLY_ACCEPTED") == .fullyAccepted)
        // The API types `status` as a plain string; anything unknown must not crash the screen.
        #expect(TradeStatus(apiValue: "SOMETHING_ELSE") == .pending)
    }

    @Test func onlyPendingAndOneAcceptedCanBeEdited() {
        #expect(TradeStatus.pending.isEditable)
        #expect(TradeStatus.oneAccepted.isEditable)
        #expect(!TradeStatus.fullyAccepted.isEditable)
        #expect(!TradeStatus.abandoned.isEditable)
    }

    @Test func cardsAreReservedFromTheFirstAcceptanceUntilTheExchangeIsConfirmed() {
        #expect(TradeStatus.oneAccepted.isReserved)
        #expect(TradeStatus.fullyAccepted.isReserved)
        #expect(!TradeStatus.pending.isReserved)
        #expect(!TradeStatus.completed.isReserved)
    }

    @Test func theArchiveHoldsClosedAndAbandonedTradesOnly() {
        #expect(!TradeStatus.closed.isOngoing)
        #expect(!TradeStatus.abandoned.isOngoing)
        #expect(TradeStatus.completed.isOngoing)
    }

    @Test func abandonedIsNotAStepOfTheLifecycle() {
        #expect(TradeStatus.abandoned.lifecycleIndex == nil)
        #expect(TradeStatus.pending.lifecycleIndex == 0)
        #expect(TradeStatus.closed.lifecycleIndex == 4)
        #expect(TradeStatus.lifecycle.count == TradeSteps.all.count)
    }
}

struct TradeBalanceTests {
    @Test func totalsTrendPriceTimesQuantityAndIgnoresUnpricedCards() {
        #expect(TradeBalance.total(of: [card(trend: 900, quantity: 2), card(trend: nil)]) == 1800)
        #expect(TradeBalance.total(of: []) == 0)
    }

    @Test func aDifferenceUnderThreeEurosReadsAsEven() {
        let balance = TradeBalance(give: [card(trend: 1000)], get: [card(trend: 1200)])
        #expect(balance.isEven)
        #expect(balance.verdict == "Équilibré")
        #expect(balance.settlementLabel == nil)
    }

    @Test func theUserOwesWhenTheCardsComingInAreWorthMore() {
        let balance = TradeBalance(give: [card(trend: 1900)], get: [card(trend: 4000)])
        #expect(balance.diffCents == 2100)
        #expect(!balance.isEven)
        #expect(balance.verdict.hasPrefix("Tu dois"))
        #expect(balance.settlementLabel?.hasPrefix("payer") == true)
    }

    @Test func theUserIsOwedWhenTheCardsGoingOutAreWorthMore() {
        let balance = TradeBalance(give: [card(trend: 4000)], get: [card(trend: 1900)])
        #expect(balance.diffCents == -2100)
        #expect(balance.verdict.hasPrefix("On te doit"))
        #expect(balance.settlementLabel?.hasPrefix("recevoir") == true)
    }

    @Test func anEmptyTradeSplitsTheBarDownTheMiddle() {
        #expect(TradeBalance(give: [], get: []).receivedShare == 0.5)
        #expect(TradeBalance(give: [card(trend: 1000)], get: [card(trend: 3000)]).receivedShare == 0.75)
    }
}

struct TradeCopyTests {
    @Test func parsesTheApiTimestampWithOrWithoutFractionalSeconds() {
        #expect(TradeTimestamp.date(from: "2026-09-03T10:15:30.123Z") != nil)
        #expect(TradeTimestamp.date(from: "2026-09-03T10:15:30Z") != nil)
        #expect(TradeTimestamp.date(from: "pas une date") == nil)
        #expect(TradesCopy.relativeDate(from: "pas une date") == "")
    }

    @Test func pluralisesTheCardCount() {
        #expect(TradesCopy.cardCount(1) == "1 carte")
        #expect(TradesCopy.cardCount(3) == "3 cartes")
    }

    @Test func identifiesACardLineByItsFourKeyFields() {
        #expect(card(trend: 900).lineID == "FIN-243-fr-false")
    }
}
