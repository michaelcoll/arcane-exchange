import Testing

@testable import ArcaneExchangeCore

struct TradeStepsTests {
    @Test func theStepsScreenHasOneEntryPerLifecycleStep() {
        #expect(TradeStatus.lifecycle.count == TradeSteps.all.count)
    }
}
