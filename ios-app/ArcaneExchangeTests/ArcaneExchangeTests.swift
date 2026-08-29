import Testing

@testable import ArcaneExchange

struct ArcaneExchangeTests {
    @Test func appConfigExposesBackendBaseURL() {
        #expect(AppConfig.apiBaseURL.absoluteString.hasSuffix("/api/v1"))
    }
}
