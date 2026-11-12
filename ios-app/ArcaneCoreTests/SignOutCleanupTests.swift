import Foundation
import Testing

@testable import ArcaneCore

struct SignOutCleanupTests {
    private static func defaultsWithSavedFilters() -> UserDefaults {
        let defaults = isolatedDefaults()
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
        let widgetDefaults = isolatedDefaults()
        let widgetCache = HomeWidgetCache(defaults: widgetDefaults)
        widgetCache.save(TradesSnapshot(total: 0, trades: []), for: .trades)
        widgetCache.save(CollectionSnapshot(history: [], totalCards: 1, uniqueCards: 1), for: .collection)
        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: isolatedDefaults(), widgetDefaults: widgetDefaults)

        #expect(widgetCache.load(TradesSnapshot.self, for: .trades) == nil)
        #expect(widgetCache.load(CollectionSnapshot.self, for: .collection) == nil)
    }

    @MainActor private static func searchModel(_ defaults: UserDefaults) -> SearchViewModel {
        SearchViewModel(api: FakeAPI(), defaults: defaults)
    }

    @MainActor @Test func forgetsTheSearchRecentsOnceSignedOut() {
        let defaults = isolatedDefaults()
        let model = Self.searchModel(defaults)
        model.rememberQuery("Sol Ring")
        model.rememberPlayer(UserSuggestion(card_count: 3, note: 5, username: "alice"))
        SignOutCleanup.run(isLoaded: true, isSignedIn: false, defaults: defaults, widgetDefaults: isolatedDefaults())

        let next = Self.searchModel(defaults)
        #expect(next.recentQueries.isEmpty)
        #expect(next.recentPlayers.isEmpty)
    }

    @MainActor @Test func keepsTheSearchRecentsWhileLoadingOrSignedIn() {
        let defaults = isolatedDefaults()
        let alice = UserSuggestion(card_count: 3, note: 5, username: "alice")
        let model = Self.searchModel(defaults)
        model.rememberQuery("Sol Ring")
        model.rememberPlayer(alice)
        SignOutCleanup.run(isLoaded: false, isSignedIn: false, defaults: defaults, widgetDefaults: isolatedDefaults())
        let whileLoading = Self.searchModel(defaults)
        SignOutCleanup.run(isLoaded: true, isSignedIn: true, defaults: defaults, widgetDefaults: isolatedDefaults())
        let signedIn = Self.searchModel(defaults)

        #expect(whileLoading.recentQueries == ["Sol Ring"])
        #expect(whileLoading.recentPlayers == [alice])
        #expect(signedIn.recentQueries == ["Sol Ring"])
        #expect(signedIn.recentPlayers == [alice])
    }

    @MainActor @Test func readsRecentsWrittenInTheCurrentFormat() {
        let defaults = isolatedDefaults()
        defaults.set(Data("[\"Sol Ring\"]".utf8), forKey: "search.recentCardQueries")
        defaults.set(
            Data(#"[{"card_count":3,"note":5,"username":"alice"}]"#.utf8),
            forKey: "search.recentPlayers"
        )

        let model = Self.searchModel(defaults)
        #expect(model.recentQueries == ["Sol Ring"])
        #expect(model.recentPlayers == [UserSuggestion(card_count: 3, note: 5, username: "alice")])
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

    @Test func keepsThemWhileSignedIn() {
        let defaults = Self.defaultsWithSavedFilters()
        SignOutCleanup.run(isLoaded: true, isSignedIn: true, defaults: defaults)

        #expect(CollectionFiltersStore(defaults: defaults).load().sets == ["MH3"])
    }
}
