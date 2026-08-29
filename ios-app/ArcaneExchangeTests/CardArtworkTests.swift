import Foundation
import Testing

@testable import ArcaneExchange

struct CardArtworkTests {
    @Test func prefersGathererWhenTheCardCarriesItsID() {
        let url = CardArtwork.url(gathererID: "abc-123", scryfallID: "def-456")
        #expect(url?.absoluteString == "https://gatherer-static.wizards.com/Cards/medium/abc-123.webp")
    }

    /// A missing *or blank* Gatherer id has to fall through — the API sends an empty string
    /// for cards it could not match, and `.../medium/.webp` is a guaranteed 404.
    @Test func fallsBackToScryfallWithoutAUsableGathererID() {
        let expected = "https://api.scryfall.com/cards/def-456?format=image&version=normal"
        #expect(CardArtwork.url(gathererID: nil, scryfallID: "def-456")?.absoluteString == expected)
        #expect(CardArtwork.url(gathererID: "", scryfallID: "def-456")?.absoluteString == expected)
    }

    @Test func mapsAPageToPrefetchableURLsInGridOrder() {
        let urls = CardArtwork.urls(for: [card(scryfallID: "aaa"), card(scryfallID: "bbb")])
        #expect(urls.count == 2)
        #expect(urls.first?.absoluteString.contains("aaa") == true)
        #expect(urls.last?.absoluteString.contains("bbb") == true)
    }

    @Test func mapsAnEmptyPageToNoWork() {
        #expect(CardArtwork.urls(for: []).isEmpty)
    }

    private func card(scryfallID: String) -> CollectionCard {
        CollectionCard(
            collector_number: "1",
            foil: false,
            language_code: "en",
            name: "Card \(scryfallID)",
            rarity_code: "C",
            reserved: false,
            scryfall_id: scryfallID,
            set_code: "SOA"
        )
    }
}
