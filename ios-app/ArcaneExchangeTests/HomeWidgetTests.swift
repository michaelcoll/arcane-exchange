import APIClient
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

/// A cache on a defaults domain of its own per call.
private func makeCache() -> HomeWidgetCache {
    HomeWidgetCache(defaults: UserDefaults(suiteName: "HomeWidgetTests.\(UUID().uuidString)")!)
}

struct HomeWidgetLoaderTests {
    private struct Failure: Error {}

    private static let trades = TradesSnapshot(total: 1, trades: [
        ActiveTrade(id: "t1", partnerUsername: "mizzix_42", myCardCount: 2, partnerCardCount: 1, status: .pending),
    ])

    @Test func keepsTheTradeStatusesThroughTheCache() {
        let cache = makeCache()
        let snapshot = TradesSnapshot(total: 2, trades: [
            ActiveTrade(id: "t1", partnerUsername: "mizzix_42", myCardCount: 2, partnerCardCount: 1, status: .oneAccepted),
            ActiveTrade(id: "t2", partnerUsername: "golgari.jo", myCardCount: 1, partnerCardCount: 3, status: .fullyAccepted),
        ])
        cache.save(snapshot, for: .trades)

        #expect(cache.load(TradesSnapshot.self, for: .trades)?.trades.map(\.status) == [.oneAccepted, .fullyAccepted])
    }

    /// A status a newer API added must not lose the whole cached snapshot: it reads back as
    /// `.pending`, like `TradeStatus(apiValue:)`.
    @Test func readsAnUnknownCachedStatusAsPending() throws {
        let defaults = UserDefaults(suiteName: "HomeWidgetLoaderTests.\(UUID().uuidString)")!
        let json = """
        {"total":1,"trades":[{"id":"t1","partnerUsername":"mizzix_42","myCardCount":2,"partnerCardCount":1,\
        "status":"SOMETHING_ELSE"}]}
        """
        defaults.set(Data(json.utf8), forKey: HomeWidgetCache.key(.trades))

        let snapshot = try #require(HomeWidgetCache(defaults: defaults).load(TradesSnapshot.self, for: .trades))
        #expect(snapshot.trades.map(\.status) == [.pending])
    }

    @Test func showsAndRemembersWhatTheAPIReturned() async {
        let cache = makeCache()
        let content = await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: true) { Self.trades }

        #expect(content == .snapshot(Self.trades))
        #expect(cache.load(TradesSnapshot.self, for: .trades) == Self.trades)
    }

    @Test func fallsBackToTheLastSnapshotWhenTheCallFails() async {
        let cache = makeCache()
        cache.save(Self.trades, for: .trades)
        let content: HomeWidgetContent<TradesSnapshot> =
            await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: true) { throw Failure() }

        #expect(content == .snapshot(Self.trades))
    }

    @Test func isUnavailableWhenTheCallFailsWithNothingCached() async {
        let content: HomeWidgetContent<TradesSnapshot> =
            await HomeWidgetLoader.load(.trades, cache: makeCache(), isSignedIn: true) { throw Failure() }

        #expect(content == .unavailable)
    }

    @Test func asksToSignInWithoutCallingTheAPI() async {
        let cache = makeCache()
        cache.save(Self.trades, for: .trades)
        let content: HomeWidgetContent<TradesSnapshot> =
            await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: false) {
                Issue.record("the API must not be called while signed out")
                return Self.trades
            }

        #expect(content == .signedOut)
    }
}

struct HomeWidgetFetchTests {
    private static func tradesAPI(_ outcome: @escaping () throws -> Operations.list_trades.Output) -> FakeAPI {
        let api = FakeAPI()
        api.onListTrades = { _ in try outcome() }
        return api
    }

    @Test func buildsTheTradesSnapshotFromTheActiveTradesPage() async throws {
        let api = Self.tradesAPI {
            .ok(.init(body: .json(.init(
                items: [TradeSummary(
                    id: "t1",
                    my_card_count: 2,
                    partner_card_count: 1,
                    partner_username: "mizzix_42",
                    status: TradeStatus.oneAccepted.rawValue,
                    updated_at: "2026-10-01T10:00:00Z"
                )],
                page: 0,
                page_size: 3,
                total: 5
            ))))
        }

        let snapshot = try await HomeWidgetFetch.trades(api: api)

        #expect(snapshot == TradesSnapshot(total: 5, trades: [
            ActiveTrade(id: "t1", partnerUsername: "mizzix_42", myCardCount: 2, partnerCardCount: 1, status: .oneAccepted),
        ]))
        #expect(api.calls == ["list_trades"])
    }

    @Test func buildsTheCollectionSnapshotFromTheHistoryAndTheStats() async throws {
        let api = FakeAPI()
        let entry = PriceHistoryEntry(avg: 90, date: "2026-10-01", low: 80, trend: 100)
        api.onGetCollectionPriceHistory = { _ in .ok(.init(body: .json([entry]))) }
        api.onGetCollectionStats = { _ in
            .ok(.init(body: .json(.init(binders: [], sets: [], total_cards: 12, unique_cards: 9))))
        }

        let snapshot = try await HomeWidgetFetch.collection(api: api)

        #expect(snapshot == CollectionSnapshot(history: [entry], totalCards: 12, uniqueCards: 9))
        #expect(Set(api.calls) == ["get_collection_price_history", "get_collection_stats"])
    }

    /// An expired session shows the widget's "not signed in" state, whatever an earlier
    /// snapshot says; nothing here signs the app out.
    @Test func anExpiredSessionShowsTheSignedOutState() async {
        let cache = makeCache()
        cache.save(TradesSnapshot(total: 0, trades: []), for: .trades)
        let api = Self.tradesAPI { throw APIError(code: "unauthorized", status: 401) }

        let content = await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: true) {
            try await HomeWidgetFetch.trades(api: api)
        }

        #expect(content == .signedOut)
    }

    @Test func aNetworkFailureKeepsTheLastSnapshot() async {
        let cache = makeCache()
        let remembered = TradesSnapshot(total: 0, trades: [])
        cache.save(remembered, for: .trades)
        let api = Self.tradesAPI { throw APIError(code: nil, status: nil, isNetworkFailure: true) }

        let content = await HomeWidgetLoader.load(.trades, cache: cache, isSignedIn: true) {
            try await HomeWidgetFetch.trades(api: api)
        }

        #expect(content == .snapshot(remembered))
    }

    @Test func aNetworkFailureWithNothingCachedIsUnavailable() async {
        let api = Self.tradesAPI { throw APIError(code: nil, status: nil, isNetworkFailure: true) }

        let content = await HomeWidgetLoader.load(.trades, cache: makeCache(), isSignedIn: true) {
            try await HomeWidgetFetch.trades(api: api)
        }

        #expect(content == .unavailable)
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
