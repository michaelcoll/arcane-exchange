import APIClient
import Foundation

/// An impression of the Catalogue `GET /search/card` found (set and number, every language and
/// finish together): what is public about it, with no purchase price and no date of addition.
/// The Collection has its own type, `CollectionCard`.
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
    /// The name a result shows: French when known, English otherwise.
    var displayName: String {
        french_name ?? name
    }

    /// An impression with no normal price but a foil one: its reference price is the foil trend.
    var isFoilOnly: Bool {
        price_guide?.trend == nil && foil_price_guide?.trend != nil
    }

    /// The reference price in cents: the normal trend, or the foil one without it.
    var referencePriceCents: Int? {
        (price_guide?.trend ?? foil_price_guide?.trend).map(Int.init)
    }

    /// The card as the detail screens read it: they take a `CollectionCard`, which has a language
    /// and a finish where an impression has none. It opens on the French card when it is offered,
    /// the English one otherwise, in foil when the impression only has a foil price. A search
    /// result carries no collection entry.
    var asCollectionCard: CollectionCard {
        CollectionCard(
            collection_entry: nil,
            collector_number: collector_number,
            foil: isFoilOnly,
            image_back_url: image_back_url,
            image_url: image_url,
            language_code: offered_languages.contains("FR") ? "FR" : "EN",
            name: displayName,
            price_guide: isFoilOnly ? foil_price_guide : price_guide,
            rarity_code: rarity_code,
            reserved: reserved ?? false,
            scryfall_id: scryfall_id,
            set_code: set_code
        )
    }
}
