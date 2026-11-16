import APIClient
import Testing

@testable import ArcaneExchangeCore

struct ImpressionLinkResolverTests {
    private let key = ImpressionKey(set: "FDN", number: "001")

    private func page(_ items: [SearchCard]) -> Operations.search_cards.Output {
        .ok(.init(body: .json(.init(items: items, page: 0, page_size: 20, total: Int64(items.count)))))
    }

    @Test func findsTheImpressionOfAQueuedOne() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([searchCard("a")]) }

        let card = await ImpressionLinkResolver(api: api).card(for: .detail(key))

        #expect(card?.scryfall_id == "a")
    }

    @Test func findsNothingForAnImpressionTheCatalogueDoesNotHave() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in page([]) }

        #expect(await ImpressionLinkResolver(api: api).card(for: .detail(key)) == nil)
    }

    @Test func findsNothingWhenTheCallFails() async {
        let api = FakeAPI()
        api.onSearchCards = { _ in throw APIError(code: "internal", status: 500) }

        #expect(await ImpressionLinkResolver(api: api).card(for: .detail(key)) == nil)
    }
}
