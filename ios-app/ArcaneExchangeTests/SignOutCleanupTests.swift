import Foundation
import Testing

@testable import ArcaneExchange

struct SignOutCleanupTests {
    private static func defaultsWithSavedFilters() -> UserDefaults {
        let defaults = UserDefaults(suiteName: "SignOutCleanupTests.\(UUID().uuidString)")!
        CollectionFiltersStore(defaults: defaults).save(CollectionFilters(sets: ["MH3"], sortBy: .added_at))
        return defaults
    }

    /// Covers both a sign-out in the app and a session that expired while it was closed.
    @Test func forgetsTheCollectionFiltersOnceSignedOut() {
        let defaults = Self.defaultsWithSavedFilters()
        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: defaults)

        #expect(CollectionFiltersStore(defaults: defaults).load() == CollectionFilters())
    }

    /// At launch Clerk has no user until it restores the session: that is not a sign-out.
    @Test func keepsThemWhileTheSessionIsStillLoading() {
        let defaults = Self.defaultsWithSavedFilters()
        SignOutCleanup.run(isLoaded: false, isSignedIn: false, defaults: defaults)

        #expect(CollectionFiltersStore(defaults: defaults).load().sets == ["MH3"])
    }

    /// The widgets would otherwise keep showing the previous player's collection and trades.
    @Test func forgetsTheWidgetSnapshotsOnceSignedOut() {
        let defaults = Self.defaultsWithSavedFilters()
        let widgetCache = HomeWidgetCache(defaults: defaults)
        widgetCache.save(TradesSnapshot(total: 0, trades: []), for: .trades)
        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: defaults, widgetDefaults: defaults)

        #expect(widgetCache.load(TradesSnapshot.self, for: .trades) == nil)
    }

    @Test func keepsThemWhileSignedIn() {
        let defaults = Self.defaultsWithSavedFilters()
        SignOutCleanup.run(isLoaded: true, isSignedIn: true, defaults: defaults)

        #expect(CollectionFiltersStore(defaults: defaults).load().sets == ["MH3"])
    }
}
