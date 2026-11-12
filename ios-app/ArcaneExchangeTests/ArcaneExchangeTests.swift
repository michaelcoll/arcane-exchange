import Testing

@testable import ArcaneExchange

struct ArcaneExchangeTests {
    @Test func appConfigExposesBackendBaseURL() {
        let info = ["API_BASE_URL": "https://ae.piconsoft.fr/api/v1"]
        #expect(AppConfig.apiBaseURL(info: info).absoluteString.hasSuffix("/api/v1"))
    }
}
