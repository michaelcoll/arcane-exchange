import AppIntents

/// An impression as the system knows it, for Siri, Spotlight and Shortcuts. Its identity is the
/// key: an intent looks the impression up again, so the quantity, the price and the Owner count
/// it answers are never older than the question. The search result it was found as only serves
/// to display it, and is absent when nobody is signed in.
package struct ImpressionEntity: AppEntity {
    package static let typeDisplayRepresentation = TypeDisplayRepresentation(name: "Carte")
    package static let defaultQuery = ImpressionQuery()

    package let key: ImpressionKey
    package let card: SearchCard?

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
}

/// Resolves impressions on demand, over the network: no Spotlight index.
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
