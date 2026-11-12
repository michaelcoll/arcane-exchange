import WidgetKit

/// Asks WidgetKit to rebuild the widgets' timelines now.
///
/// The app is awake anyway when it calls this, so these refreshes cost no extra wake-up —
/// they are what keeps the widgets current, `HomeWidgetSchedule` only covers the app being
/// closed.
///
/// A mutation that changes what a widget shows does not call this: the authenticated API client
/// reloads the widget it touches, from the operation table of the widgets reload middleware
/// (rating a trade and the trade settings are deliberately absent). The one direct call of `reload`
/// is the end of the import in `ImportViewModel`, as it is no mutation (the server answers 202
/// before the collection changes); `reloadAll` covers the session changes and the app leaving
/// the foreground.
package enum HomeWidgetReload {
    package static func reload(_ kind: HomeWidgetKind) {
        WidgetCenter.shared.reloadTimelines(ofKind: kind.rawValue)
    }

    package static func reloadAll() {
        WidgetCenter.shared.reloadAllTimelines()
    }
}
