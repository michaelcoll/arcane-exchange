import Foundation

enum CardArtwork {
    /// Artwork for a card, from the same two sources as the web client (`MtgCard.vue`):
    /// Gatherer's static image when the card carries a Gatherer id, Scryfall's image redirect
    /// otherwise. The API returns ids, never image URLs.
    static func url(gathererID: String?, scryfallID: String) -> URL? {
        if let gathererID, !gathererID.isEmpty {
            return URL(string: "https://gatherer-static.wizards.com/Cards/medium/\(gathererID).webp")
        }
        return URL(string: "https://api.scryfall.com/cards/\(scryfallID)?format=image&version=normal")
    }

    /// The artwork URLs of a page of cards, in grid order — what the prefetcher warms.
    /// Cards whose ids yield no usable URL simply drop out; the cell falls back to its
    /// placeholder either way.
    static func urls(for cards: [CollectionCard]) -> [URL] {
        cards.compactMap { url(gathererID: $0.the_gatherer_id, scryfallID: $0.scryfall_id) }
    }
}
