import Testing

@testable import ArcaneExchangeCore

@MainActor
struct AppRouterTests {
    private let key = ImpressionKey(set: "FDN", number: "001")

    @Test func aTradeLinkOpensTheTradesTabAndQueuesTheTrade() {
        let router = AppRouter()
        let route = TradeDetailRoute(id: "t1", partnerUsername: "mizzix_42")
        router.open(.trade(route))

        #expect(router.tab == .trades)
        #expect(router.pendingTrade == route)
    }

    @Test func anImpressionLinkOpensTheSearchTabAndQueuesTheImpression() {
        let router = AppRouter()
        router.open(.impression(key))

        #expect(router.tab == .search)
        #expect(router.pendingImpression == .detail(key))
    }

    @Test func aQueuedImpressionKnowsItsKey() {
        #expect(AppRouter.PendingImpression.detail(key).key == key)
    }

    @Test func aTabLinkDropsAnyQueuedImpression() {
        let router = AppRouter()
        router.open(.impression(key))
        router.open(.trades)

        #expect(router.tab == .trades)
        #expect(router.pendingImpression == nil)
    }

    @Test func aTabLinkDropsAnyQueuedTrade() {
        let router = AppRouter()
        router.open(.trade(TradeDetailRoute(id: "t1", partnerUsername: "mizzix_42")))
        router.open(.collection)

        #expect(router.tab == .collection)
        #expect(router.pendingTrade == nil)
    }
}
