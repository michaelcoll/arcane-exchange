import Foundation

/// When each home-screen widget asks WidgetKit to be refreshed next.
///
/// These are only the earliest dates: iOS picks the actual moment, folding it into a wake-up it
/// already had planned. Most refreshes come from the app itself anyway (`HomeWidgetReload`).
enum HomeWidgetSchedule {
    /// The collection value only moves when the backend imports the Cardmarket prices, at 00:00
    /// and 12:00 UTC (`schedule_price_import_job`, cron `0 0 */12 * * *`): aim half an hour
    /// after each, once the job is done.
    static func nextCollectionRefresh(after date: Date) -> Date {
        var calendar = Calendar(identifier: .gregorian)
        calendar.timeZone = TimeZone(identifier: "UTC")!
        let slots = [DateComponents(hour: 0, minute: 30), DateComponents(hour: 12, minute: 30)]
        return slots
            .compactMap { calendar.nextDate(after: date, matching: $0, matchingPolicy: .nextTime) }
            .min()!
    }

    /// The other player's moves are only picked up here while the app is closed: the app
    /// reloads the widget itself after each of the user's own.
    static func nextTradesRefresh(after date: Date) -> Date {
        date.addingTimeInterval(2 * 60 * 60)
    }
}
