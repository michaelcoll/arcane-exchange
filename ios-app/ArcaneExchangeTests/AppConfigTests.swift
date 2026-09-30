import Foundation
import Testing

@testable import ArcaneExchange

/// Serialized: these tests move `UserDefaults.standard`, which every other test — and the
/// hosting app — reads from.
@Suite(.serialized)
struct AppConfigTests {
    @Test func rejectsWhatTheTransportCouldNotUse() {
        #expect(AppConfig.parseBaseURL("") == nil)
        #expect(AppConfig.parseBaseURL("   ") == nil)
        #expect(AppConfig.parseBaseURL("localhost:8080") == nil)
        #expect(AppConfig.parseBaseURL("ftp://example.com") == nil)
    }

    @Test func acceptsAnAbsoluteHTTPURLAndTrimsIt() {
        let url = AppConfig.parseBaseURL("  https://ae.piconsoft.fr/api/v1  ")
        #expect(url?.absoluteString == "https://ae.piconsoft.fr/api/v1")
        #expect(AppConfig.parseBaseURL("http://localhost:8080/api/v1") != nil)
    }

    @Test func settingsOverrideWinsOverTheBundledURL() {
        withAPIBaseURLSetting("https://staging.example.com/api/v1") {
            #expect(AppConfig.apiBaseURL.absoluteString == "https://staging.example.com/api/v1")
        }
    }

    @Test func aBlankOrBrokenOverrideFallsBackToTheBundledURL() {
        withAPIBaseURLSetting("") {
            #expect(AppConfig.apiBaseURL == AppConfig.bundledAPIBaseURL)
        }
        withAPIBaseURLSetting("nope") {
            #expect(AppConfig.apiBaseURL == AppConfig.bundledAPIBaseURL)
        }
    }

    /// The widget extension's own defaults are empty: it reads what the app shared.
    @Test func withoutASettingTheSharedCopyIsUsed() {
        let shared = SharedContainer.defaults
        let key = AppConfig.apiBaseURLDefaultsKey
        let previousShared = shared.string(forKey: key)
        shared.set("https://shared.example.com/api/v1", forKey: key)
        defer { shared.set(previousShared, forKey: key) }

        withAPIBaseURLSetting(nil) {
            #expect(AppConfig.apiBaseURL.absoluteString == "https://shared.example.com/api/v1")
        }
    }

    @Test func shareCopiesTheURLInUseForTheWidgets() {
        let shared = SharedContainer.defaults
        let key = AppConfig.apiBaseURLDefaultsKey
        let previousShared = shared.string(forKey: key)
        defer { shared.set(previousShared, forKey: key) }

        withAPIBaseURLSetting("https://staging.example.com/api/v1") {
            AppConfig.shareAPIBaseURL()
            #expect(shared.string(forKey: key) == "https://staging.example.com/api/v1")
        }
    }

    @Test func bundledURLTargetsTheVersionedAPI() {
        #expect(AppConfig.bundledAPIBaseURL.absoluteString.hasSuffix("/api/v1"))
    }

    /// `nil` removes the setting for the duration of `body`.
    private func withAPIBaseURLSetting(_ value: String?, _ body: () -> Void) {
        let defaults = UserDefaults.standard
        let key = AppConfig.apiBaseURLDefaultsKey
        let previous = defaults.string(forKey: key)
        defaults.set(value, forKey: key)
        defer {
            if let previous {
                defaults.set(previous, forKey: key)
            } else {
                defaults.removeObject(forKey: key)
            }
        }
        body()
    }
}
