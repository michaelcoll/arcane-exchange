import APIClient
import Foundation

package enum CardArtwork {
    /// A card image stored by the platform. The API gives its path relative to the
    /// frontend (`image_url`, `image_back_url`), which serves `/card-images` from the same origin
    /// it proxies the API under: the path resolves against the origin of the API base URL.
    /// `nil` while the card's image is pending.
    ///
    /// The base URL is an autoclosure: it is read from the process's Info.plist only once a card
    /// has an image, so a page of pending cards (and the tests of a paginator) never needs it.
    package static func url(imagePath: String?, apiBaseURL: @autoclosure () -> URL = AppConfig.apiBaseURL()) -> URL? {
        guard let imagePath else { return nil }
        return URL(string: imagePath, relativeTo: apiBaseURL())?.absoluteURL
    }

    /// The English front image of a printing, addressed by its set and number: the scan batch has
    /// no `image_url`. The file is named `{SET}_{number}_EN.webp` (the image a card in fallback
    /// shares), and only exists once the card is in the catalog: the caller keeps the set symbol
    /// for the failed load. The number is numeric, as in the scan index: nothing to escape.
    package static func url(
        forSet set: String,
        number: String,
        apiBaseURL: @autoclosure () -> URL = AppConfig.apiBaseURL()
    ) -> URL? {
        url(imagePath: "/card-images/\(set.uppercased())_\(number)_EN.webp", apiBaseURL: apiBaseURL())
    }

    /// The front image URLs of a page of cards, in grid order — what the prefetcher warms.
    /// Cards still pending drop out; their cell shows the card back either way.
    package static func urls(
        for cards: [CollectionCard],
        apiBaseURL: @autoclosure () -> URL = AppConfig.apiBaseURL()
    ) -> [URL] {
        cards.compactMap { url(imagePath: $0.image_url, apiBaseURL: apiBaseURL()) }
    }
}
