import Foundation
import Testing

@testable import ArcaneExchange

struct HomeWidgetScheduleTests {
    private static func utc(_ iso: String) -> Date {
        ISO8601DateFormatter().date(from: iso)!
    }

    @Test func aimsHalfAnHourAfterTheMidnightImport() {
        let next = HomeWidgetSchedule.nextCollectionRefresh(after: Self.utc("2026-09-30T00:10:00Z"))
        #expect(next == Self.utc("2026-09-30T00:30:00Z"))
    }

    @Test func aimsHalfAnHourAfterTheNoonImportBetweenTheTwo() {
        let next = HomeWidgetSchedule.nextCollectionRefresh(after: Self.utc("2026-09-30T08:00:00Z"))
        #expect(next == Self.utc("2026-09-30T12:30:00Z"))
    }

    @Test func rollsOverToTheNextDayAfterTheNoonSlot() {
        let next = HomeWidgetSchedule.nextCollectionRefresh(after: Self.utc("2026-09-30T18:45:00Z"))
        #expect(next == Self.utc("2026-10-01T00:30:00Z"))
    }

    /// A timeline rebuilt right on a slot must not ask for itself again immediately.
    @Test func neverReturnsTheCurrentInstant() {
        let next = HomeWidgetSchedule.nextCollectionRefresh(after: Self.utc("2026-09-30T12:30:00Z"))
        #expect(next == Self.utc("2026-10-01T00:30:00Z"))
    }

    @Test func tradesAreRefreshedTwoHoursLater() {
        let now = Self.utc("2026-09-30T08:00:00Z")
        #expect(HomeWidgetSchedule.nextTradesRefresh(after: now) == Self.utc("2026-09-30T10:00:00Z"))
    }
}

struct CollectionSnapshotTests {
    private static func snapshot(trends: [Int64]) -> CollectionSnapshot {
        let entries = trends.enumerated().map { index, trend in
            PriceHistoryEntry(avg: trend, date: String(format: "2026-09-%02d", index + 1), low: trend, trend: trend)
        }
        return CollectionSnapshot(history: entries, totalCards: 12, uniqueCards: 9)
    }

    /// Same window as the web tile's `lastNDaysRange(30)`: today and the 29 days before, local
    /// dates — not the endpoint's default, which starts a day earlier.
    @Test func historyCoversTodayAndThe29DaysBefore() {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "Europe/Paris")!
        let now = ISO8601DateFormatter().date(from: "2026-09-30T23:30:00Z")! // 1 Oct in Paris
        let range = CollectionSnapshot.historyRange(endingAt: now, calendar: calendar)

        #expect(range.start == "2026-09-02")
        #expect(range.end == "2026-10-01")
    }

    @Test func valueIsTheLastTrend() {
        #expect(Self.snapshot(trends: [10000, 12345]).valueCents == 12345)
    }

    @Test func variationComparesTheLastTrendWithTheFirst() {
        let variation = Self.snapshot(trends: [10000, 9000, 11000]).variation
        #expect(variation.deltaCents == 1000)
        #expect(abs(variation.percent - 10) < 0.0001)
        #expect(variation.isRising)
    }

    @Test func aDropIsNotRising() {
        let variation = Self.snapshot(trends: [10000, 8000]).variation
        #expect(variation.deltaCents == -2000)
        #expect(!variation.isRising)
    }

    /// Same as the web's `computeVariation`: no history, no movement.
    @Test func variationIsFlatWithFewerThanTwoPoints() {
        let snapshot = Self.snapshot(trends: [10000])
        #expect(snapshot.variation == ValueVariation(deltaCents: 0, percent: 0))
        #expect(!snapshot.hasEnoughHistory)
    }

    @Test func variationPercentIsZeroFromAnEmptyCollection() {
        #expect(Self.snapshot(trends: [0, 500]).variation.percent == 0)
    }

    @Test func variationLabelMatchesTheWebTile() {
        let variation = ValueVariation(deltaCents: 4210, percent: 3.456)
        #expect(variation.label == "▴ \(Price.euros(cents: 4210)) · +3,5 % (30 j)")
        #expect(ValueVariation(deltaCents: -150, percent: -1.25).label == "▾ \(Price.euros(cents: 150)) · −1,3 % (30 j)")
    }
}

struct HomeWidgetLoaderTests {
    private struct Failure: Error {}

    private static func cache() -> HomeWidgetCache {
        HomeWidgetCache(defaults: UserDefaults(suiteName: "HomeWidgetLoaderTests.\(UUID().uuidString)")!)
    }

    private static let trades = TradesSnapshot(total: 1, trades: [
        ActiveTrade(id: "t1", partnerUsername: "mizzix_42", myCardCount: 2, partnerCardCount: 1, status: "PENDING"),
    ])

    @Test func showsAndRemembersWhatTheAPIReturned() async {
        let cache = Self.cache()
        let content = await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: true) { Self.trades }

        #expect(content == .snapshot(Self.trades))
        #expect(cache.load(TradesSnapshot.self, for: .trades) == Self.trades)
    }

    @Test func fallsBackToTheLastSnapshotWhenTheCallFails() async {
        let cache = Self.cache()
        cache.save(Self.trades, for: .trades)
        let content: HomeWidgetContent<TradesSnapshot> =
            await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: true) { throw Failure() }

        #expect(content == .snapshot(Self.trades))
    }

    @Test func isUnavailableWhenTheCallFailsWithNothingCached() async {
        let content: HomeWidgetContent<TradesSnapshot> =
            await HomeWidgetLoader.load(.trades, cache: Self.cache(), isSignedIn: true) { throw Failure() }

        #expect(content == .unavailable)
    }

    @Test func asksToSignInWithoutCallingTheAPI() async {
        let cache = Self.cache()
        cache.save(Self.trades, for: .trades)
        let content: HomeWidgetContent<TradesSnapshot> =
            await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: false) {
                Issue.record("the API must not be called while signed out")
                return Self.trades
            }

        #expect(content == .signedOut)
    }
}

struct HomeWidgetLinkTests {
    @Test(arguments: [
        HomeWidgetLink.collection,
        .trades,
        .trade(TradeDetailRoute(id: "0190-abc", partnerUsername: "golgari.jo")),
    ])
    func survivesARoundTripThroughItsURL(link: HomeWidgetLink) {
        #expect(HomeWidgetLink(url: link.url) == link)
    }

    @Test func escapesThePartnerName() {
        let link = HomeWidgetLink.trade(TradeDetailRoute(id: "t1", partnerUsername: "jo & co"))
        #expect(HomeWidgetLink(url: link.url) == link)
    }

    @Test func ignoresOtherSchemes() {
        #expect(HomeWidgetLink(url: URL(string: "fr.piconsoft.arcane-exchange://callback")!) == nil)
    }

    @Test func ignoresAnUnknownDestination() {
        #expect(HomeWidgetLink(url: URL(string: "arcane-exchange://search")!) == nil)
    }
}

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
