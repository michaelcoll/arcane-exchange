import WidgetKit

/// Asks WidgetKit to rebuild the widgets' timelines now. The app is awake anyway when it calls
/// this, so these refreshes cost no extra wake-up: they keep the widgets current, while
/// `HomeWidgetSchedule` only covers the app being closed.
///
/// The mutations listed in `WidgetsReloadMiddleware.table` reload through the middleware. The
/// direct calls are the end of an import (the server answers 202 before the collection changes)
/// and `reloadAll` for session changes and the app leaving the foreground.
package enum HomeWidgetReload {
    package static func reload(_ kind: HomeWidgetKind) {
        WidgetCenter.shared.reloadTimelines(ofKind: kind.rawValue)
    }

    package static func reloadAll() {
        WidgetCenter.shared.reloadAllTimelines()
    }
}
