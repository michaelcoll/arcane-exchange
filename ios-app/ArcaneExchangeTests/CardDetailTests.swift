import Foundation
import Testing

@testable import ArcaneExchange

struct PriceHistorySeriesTests {
    private func entry(_ date: String, low: Int64, avg: Int64, trend: Int64) -> PriceHistoryEntry {
        PriceHistoryEntry(avg: avg, date: date, low: low, trend: trend)
    }

    @Test func convertsCentsToEurosAndSortsOldestFirst() {
        let points = PriceHistorySeries.points(from: [
            entry("2026-08-02", low: 830, avg: 870, trend: 900),
            entry("2026-08-01", low: 800, avg: 850, trend: 880),
        ])
        #expect(points.count == 2)
        #expect(points[0].date < points[1].date)
        #expect(points[0].low == 8.0)
        #expect(points[1].trend == 9.0)
    }

    @Test func dropsEntriesWithAnUnparseableDate() {
        let points = PriceHistorySeries.points(from: [entry("not-a-date", low: 1, avg: 1, trend: 1)])
        #expect(points.isEmpty)
    }

    @Test func yDomainPadsTheValueSpanByTwelvePercent() {
        let points = PriceHistorySeries.points(from: [
            entry("2026-08-01", low: 1000, avg: 2000, trend: 1500),
            entry("2026-08-02", low: 1200, avg: 1800, trend: 1600),
        ])
        let domain = PriceHistorySeries.yDomain(for: points)
        #expect(abs(domain.lowerBound - 8.8) < 0.0001)
        #expect(abs(domain.upperBound - 21.2) < 0.0001)
    }

    @Test func yDomainCoversTheTrendLineWhenItLeavesTheLowToAvgBand() {
        let points = PriceHistorySeries.points(from: [
            entry("2026-08-01", low: 1000, avg: 1200, trend: 3000),
            entry("2026-08-02", low: 900, avg: 1100, trend: 800),
        ])
        let domain = PriceHistorySeries.yDomain(for: points)
        // trend spans 8…30 €, both must sit inside the (padded) domain.
        #expect(domain.lowerBound <= 8.0)
        #expect(domain.upperBound >= 30.0)
    }

    @Test func yDomainStaysValidWhenEveryValueIsEqual() {
        let points = PriceHistorySeries.points(from: [
            entry("2026-08-01", low: 1000, avg: 1000, trend: 1000),
            entry("2026-08-02", low: 1000, avg: 1000, trend: 1000),
        ])
        let domain = PriceHistorySeries.yDomain(for: points)
        #expect(domain.lowerBound < domain.upperBound)
    }

    @Test func nearestPicksTheClosestPointInTime() {
        let points = PriceHistorySeries.points(from: [
            entry("2026-08-01", low: 100, avg: 100, trend: 100),
            entry("2026-08-10", low: 200, avg: 200, trend: 200),
        ])
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC")!
        let probe = calendar.date(from: DateComponents(year: 2026, month: 8, day: 3))!
        #expect(PriceHistorySeries.nearest(to: probe, in: points)?.trend == 1.0)
    }
}
