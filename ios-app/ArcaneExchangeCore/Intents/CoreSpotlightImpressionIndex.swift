import CoreSpotlight

/// The index of the app: the default Core Spotlight index.
package struct CoreSpotlightImpressionIndex: ImpressionIndex {
    package init() {}

    package func index(_ impressions: [ImpressionEntity]) async throws {
        try await CSSearchableIndex.default().indexAppEntities(impressions)
    }

    package func remove(_ keys: [ImpressionKey]) async throws {
        guard !keys.isEmpty else { return }
        try await CSSearchableIndex.default().deleteAppEntities(
            identifiedBy: keys.map(\.id),
            ofType: ImpressionEntity.self
        )
    }

    package func removeAll() async throws {
        try await CSSearchableIndex.default().deleteAppEntities(ofType: ImpressionEntity.self)
    }
}
