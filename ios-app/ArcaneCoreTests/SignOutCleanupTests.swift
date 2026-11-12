import Foundation
import Testing

@testable import ArcaneCore

/// What `SignOutCleanup` leaves alone or wipes in the framework's own stores. The stores that
/// live in the app have their tests next to them (`SignOutCleanupCollectionFiltersTests`,
/// `SignOutCleanupSearchRecentsTests`).
struct SignOutCleanupTests {
    /// The widgets would otherwise keep showing the previous player's collection and trades.
    @Test func forgetsTheWidgetSnapshotsOnceSignedOut() {
        let widgetDefaults = isolatedDefaults()
        let widgetCache = HomeWidgetCache(defaults: widgetDefaults)
        widgetCache.save(TradesSnapshot(total: 0, trades: []), for: .trades)
        widgetCache.save(CollectionSnapshot(history: [], totalCards: 1, uniqueCards: 1), for: .collection)
        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: isolatedDefaults(), widgetDefaults: widgetDefaults)

        #expect(widgetCache.load(TradesSnapshot.self, for: .trades) == nil)
        #expect(widgetCache.load(CollectionSnapshot.self, for: .collection) == nil)
    }

    /// Public data and a developer setting: neither belongs to the player.
    @Test func keepsTheShowcaseAndTheAPIURLInEachContainer() {
        #expect(AppConfig.apiBaseURLDefaultsKey == RawDeviceKey.apiBaseURL.rawValue)
        let defaults = isolatedDefaults()
        let widgetDefaults = isolatedDefaults()
        for container in [defaults, widgetDefaults] {
            DeviceMemory(defaults: container).write(["/card-images/a.webp"], for: DeviceKey.showcaseImagePaths)
            container.set("https://staging.example.com/api/v1", forKey: AppConfig.apiBaseURLDefaultsKey)
        }
        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: defaults, widgetDefaults: widgetDefaults)

        for container in [defaults, widgetDefaults] {
            #expect(DeviceMemory(defaults: container).read([String].self, for: DeviceKey.showcaseImagePaths)
                == ["/card-images/a.webp"])
            #expect(container.string(forKey: AppConfig.apiBaseURLDefaultsKey) == "https://staging.example.com/api/v1")
        }
    }
}
