import Foundation

enum CardArtwork {
    /// A card image stored by the platform (ADR 0017). The API gives its path relative to the
    /// frontend (`image_url`, `image_back_url`), which serves `/card-images` from the same origin
    /// it proxies the API under: the path resolves against the origin of the API base URL.
    /// `nil` while the card's image is pending.
    static func url(imagePath: String?, apiBaseURL: URL = AppConfig.apiBaseURL) -> URL? {
        guard let imagePath else { return nil }
        return URL(string: imagePath, relativeTo: apiBaseURL)?.absoluteURL
    }

    /// The front image URLs of a page of cards, in grid order — what the prefetcher warms.
    /// Cards still pending drop out; their cell shows the card back either way.
    static func urls(for cards: [CollectionCard], apiBaseURL: URL = AppConfig.apiBaseURL) -> [URL] {
        cards.compactMap { url(imagePath: $0.image_url, apiBaseURL: apiBaseURL) }
    }
}
