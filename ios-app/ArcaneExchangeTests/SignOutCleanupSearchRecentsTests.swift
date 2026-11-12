import Foundation
import Testing

@testable import ArcaneCore
@testable import ArcaneExchange

struct SignOutCleanupSearchRecentsTests {
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
}
