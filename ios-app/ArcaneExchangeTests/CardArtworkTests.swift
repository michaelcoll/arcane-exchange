import Foundation
import Testing

@testable import ArcaneExchange

struct CardArtworkTests {
    private let apiBaseURL = URL(string: "https://ae.piconsoft.fr/api/v1")!

    /// The frontend serves `/card-images` from the origin the API is proxied under, so the
    /// `/api/v1` prefix must not leak into the image URL.
    @Test func resolvesAnImagePathAgainstTheOriginOfTheAPI() {
        let url = CardArtwork.url(imagePath: "/card-images/FDN_1_EN.webp?v=gatherer", apiBaseURL: apiBaseURL)
        #expect(url?.absoluteString == "https://ae.piconsoft.fr/card-images/FDN_1_EN.webp?v=gatherer")
    }

    @Test func keepsThePortOfALocalServer() {
        let url = CardArtwork.url(
            imagePath: "/card-images/FDN_1_EN.webp?v=scryfall",
            apiBaseURL: URL(string: "http://localhost:3000/api/v1")!
        )
        #expect(url?.absoluteString == "http://localhost:3000/card-images/FDN_1_EN.webp?v=scryfall")
    }

    /// A pending card has no image yet: the view shows the card back instead.
    @Test func hasNoURLWhileTheImageIsPending() {
        #expect(CardArtwork.url(imagePath: nil, apiBaseURL: apiBaseURL) == nil)
    }

    @Test func mapsAPageToPrefetchableURLsInGridOrderSkippingPendingImages() {
        let urls = CardArtwork.urls(
            for: [card(imageURL: "/card-images/A.webp"), card(imageURL: nil), card(imageURL: "/card-images/B.webp")],
            apiBaseURL: apiBaseURL
        )
        #expect(urls.map(\.lastPathComponent) == ["A.webp", "B.webp"])
    }

    @Test func mapsAnEmptyPageToNoWork() {
        #expect(CardArtwork.urls(for: [], apiBaseURL: apiBaseURL).isEmpty)
    }

    private func card(imageURL: String?) -> CollectionCard {
        CollectionCard(
            collector_number: "1",
            foil: false,
            image_url: imageURL,
            language_code: "en",
            name: "Card",
            rarity_code: "C",
            reserved: false,
            scryfall_id: "aaa",
            set_code: "SOA"
        )
    }
}
