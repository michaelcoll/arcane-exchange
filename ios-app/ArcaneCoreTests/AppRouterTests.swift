import Testing

@testable import ArcaneCore

@MainActor
struct AppRouterTests {
    @Test func aTradeLinkOpensTheTradesTabAndQueuesTheTrade() {
        let router = AppRouter()
        let route = TradeDetailRoute(id: "t1", partnerUsername: "mizzix_42")
        router.open(.trade(route))

        #expect(router.tab == .trades)
        #expect(router.pendingTrade == route)
    }

    @Test func aTabLinkDropsAnyQueuedTrade() {
        let router = AppRouter()
        router.open(.trade(TradeDetailRoute(id: "t1", partnerUsername: "mizzix_42")))
        router.open(.collection)

        #expect(router.tab == .collection)
        #expect(router.pendingTrade == nil)
    }
}
