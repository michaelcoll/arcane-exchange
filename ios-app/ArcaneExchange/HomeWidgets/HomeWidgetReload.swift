import WidgetKit

/// Asks WidgetKit to rebuild the widgets' timelines now.
///
/// The app is awake anyway when it calls this, so these refreshes cost no extra wake-up —
/// they are what keeps the widgets current, `HomeWidgetSchedule` only covers the app being
/// closed.
enum HomeWidgetReload {
    static func reload(_ kind: HomeWidgetKind) {
        WidgetCenter.shared.reloadTimelines(ofKind: kind.rawValue)
    }

    static func reloadAll() {
        WidgetCenter.shared.reloadAllTimelines()
    }
}
