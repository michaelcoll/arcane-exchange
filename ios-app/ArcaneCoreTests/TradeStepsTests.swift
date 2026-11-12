import Testing

@testable import ArcaneCore

struct TradeStepsTests {
    @Test func theStepsScreenHasOneEntryPerLifecycleStep() {
        #expect(TradeStatus.lifecycle.count == TradeSteps.all.count)
    }
}
