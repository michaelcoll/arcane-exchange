import APIClient
import Foundation

/// A card `GET /search/card` found: what is public about it, with no purchase price and no date
/// of addition. The Collection has its own type, `CollectionCard`.
package typealias SearchCard = Components.Schemas.SearchCardResponse

/// A card shown in a grid of artwork, whichever list it comes from: what the prefetcher warms.
package protocol ArtworkCard: Hashable, Sendable {
    // The generated schemas name it so: the requirement has to match.
    // swiftlint:disable:next identifier_name
    var image_url: String? { get }
}

extension CollectionCard: ArtworkCard {}
extension SearchCard: ArtworkCard {}

package extension SearchCard {
    /// The card as the detail screens read it: they take a `CollectionCard`, and a search result
    /// carries no collection entry.
    var asCollectionCard: CollectionCard {
        CollectionCard(
            collection_entry: nil,
            collector_number: collector_number,
            foil: foil,
            image_back_url: image_back_url,
            image_url: image_url,
            language_code: language_code,
            name: name,
            price_guide: price_guide,
            rarity_code: rarity_code,
            reserved: reserved,
            scryfall_id: scryfall_id,
            set_code: set_code
        )
    }
}
