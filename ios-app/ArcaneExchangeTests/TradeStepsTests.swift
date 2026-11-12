import Testing

@testable import ArcaneCore
@testable import ArcaneExchange

struct TradeStepsTests {
    @Test func theStepsScreenHasOneEntryPerLifecycleStep() {
        #expect(TradeStatus.lifecycle.count == TradeSteps.all.count)
    }
}
