import AppIntents
import CoreSpotlight

/// An impression as the system knows it, for Siri, Spotlight and Shortcuts. Its identity is the
/// key: an intent looks the impression up again, so the quantity, the price and the Owner count
/// it answers are never older than the question. The search result it was found as only serves
/// to display it, and is absent when nobody is signed in.
package struct ImpressionEntity: AppEntity, IndexedEntity {
    package static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Carte")
    package static let defaultQuery = ImpressionQuery()

    package let key: ImpressionKey
    package let card: SearchCard?
    /// The image of the card as a local file: Core Spotlight reads no remote URL. Set by the
    /// indexer, which keeps the files.
    package var thumbnailURL: URL?

    package init(_ card: SearchCard) {
        key = ImpressionKey(card)
        self.card = card
    }

    package init(key: ImpressionKey) {
        self.key = key
        card = nil
    }

    package var id: String {
        key.id
    }

    package var displayRepresentation: DisplayRepresentation {
        guard let card else { return DisplayRepresentation(title: "\(key.label)") }
        return DisplayRepresentation(
            title: "\(card.displayName)",
            subtitle: "\(ImpressionCopy.subtitle(of: card))"
        )
    }

    /// What Spotlight indexes: enough to find and recognise the impression, nothing that ages.
    /// The displayed name, the English one as a keyword when it differs, the set, the number
    /// and the thumbnail; never a price, a quantity or an Owner count, which the page shows as
    /// they are when it opens.
    package var attributeSet: CSSearchableItemAttributeSet {
        let attributes = CSSearchableItemAttributeSet(contentType: .content)
        guard let card else {
            attributes.displayName = key.label
            return attributes
        }
        attributes.displayName = card.displayName
        attributes.title = card.displayName
        attributes.contentDescription = "\(card.set_name) · \(key.label)"
        if card.displayName != card.name {
            attributes.keywords = [card.name]
        }
        attributes.thumbnailURL = thumbnailURL
        return attributes
    }
}

/// Resolves impressions on demand, over the network: it reads no Spotlight index. The indexer
/// fills that one apart (`SpotlightIndexer`).
package struct ImpressionQuery: EntityStringQuery {
    package init() {}

    package func entities(for identifiers: [String]) async throws -> [ImpressionEntity] {
        try await IntentSession.lookup.entities(for: identifiers.compactMap { ImpressionKey(id: $0) })
    }

    /// A typed name only becomes a key through the Catalogue, so nobody signed in fails here.
    package func entities(matching string: String) async throws -> [ImpressionEntity] {
        try await IntentSession.lookup.impressions(matching: string).map(ImpressionEntity.init)
    }
}
