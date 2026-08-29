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

    @Test func bundledURLTargetsTheVersionedAPI() {
        #expect(AppConfig.bundledAPIBaseURL.absoluteString.hasSuffix("/api/v1"))
    }

    private func withAPIBaseURLSetting(_ value: String, _ body: () -> Void) {
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
