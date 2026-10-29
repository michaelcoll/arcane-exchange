import APIClient
import Testing

@testable import ArcaneExchange

struct LoginStatsTests {
    @Test func proposedCopiesAreTheCountTheStatsAnswer() async {
        let api = FakeAPI()
        api.onGetStats = { _ in .ok(.init(body: .json(.init(card_number: 10, card_price_number: 20, db_size_mb: 1, proposed_copy_number: 1234)))) }

        let copies = await LoginStats.proposedCopies(api: api)

        #expect(copies == 1234)
        #expect(api.calls == ["get_stats"])
    }

    @Test func unavailableStatsShowNoNumber() async {
        let api = FakeAPI()
        api.onGetStats = { _ in throw APIError(code: nil, status: 500) }

        #expect(await LoginStats.proposedCopies(api: api) == nil)
    }
}
