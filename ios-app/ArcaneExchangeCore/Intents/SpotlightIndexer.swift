import APIClient
import Foundation

/// Where the indexed impressions live: Core Spotlight in the app, a stand-in in the tests.
package protocol ImpressionIndex: Sendable {
    func index(_ impressions: [ImpressionEntity]) async throws
    func remove(_ keys: [ImpressionKey]) async throws
    func removeAll() async throws
}

/// Keeps the impressions of the player's Collection, and those whose page they opened lately, in
/// the system search, so that typing a card name in Spotlight finds it. Spotlight never asks an
/// app while the player types: only what is in its local index can be found, hence this indexer.
///
/// It remembers the keys of the last reconciliation and the viewed history (in `DeviceMemory`:
/// the index offers no readable list) to know what left the Collection or the history since.
package actor SpotlightIndexer {
    /// The Collection is read 100 impressions at a time, as many as the API allows.
    private static let pageSize: Int32 = 100
    /// The API refuses `page * page_size` beyond 10 000: the impressions past it are not indexed.
    private static let maxPages = 100
    /// The viewed history keeps the last 50 pages the player opened.
    private static let historyLimit = 50
    private static let minimumInterval: TimeInterval = 15 * 60

    private let api: any APIProtocol
    /// Reads an impression by its key, as the intents do. Nobody signed out gets here: the app
    /// only runs the indexer for a signed-in player, and a 401 fails the read like any error.
    private let lookup: ImpressionLookup
    private let index: any ImpressionIndex
    private let thumbnails: any ThumbnailStore
    private let memory: DeviceMemory
    private let now: @Sendable () -> Date
    /// When the last reconciliation started: claimed before the first await, so that two calls
    /// at once make one walk.
    private var lastReconciliation: Date?
    /// Bumped by every purge: what a call started before it read or computed is not written back.
    private var generation = 0

    package init(
        api: any APIProtocol,
        index: any ImpressionIndex,
        thumbnails: any ThumbnailStore,
        defaults: UserDefaults = .standard,
        now: @escaping @Sendable () -> Date = { Date() }
    ) {
        self.api = api
        lookup = ImpressionLookup(api: api, isSignedIn: { true })
        self.index = index
        self.thumbnails = thumbnails
        memory = DeviceMemory(defaults: defaults)
        self.now = now
    }

    /// Indexes the impressions the player owns and removes those that left the Collection. At
    /// most once every 15 minutes. A failure (network, session) leaves the index as it was, and
    /// does not start the wait: the next call tries again.
    package func reconcileCollection() async {
        let start = now()
        let previousStart = lastReconciliation
        if let previousStart, start.timeIntervalSince(previousStart) < Self.minimumInterval {
            return
        }
        lastReconciliation = start
        let started = generation
        do {
            let owned = try await ownedImpressions()
            guard generation == started else { return }
            let ownedKeys = Set(owned.map(\.key))
            let left = Set(keys(for: .spotlightOwnedKeys)).subtracting(ownedKeys)
            try await index.index(owned)
            guard generation == started else {
                await discard(Array(ownedKeys))
                return
            }
            let gone = left.subtracting(keys(for: .spotlightViewedKeys))
            try await index.remove(Array(gone))
            thumbnails.remove(Array(gone))
            guard generation == started else { return }
            memory.write(ownedKeys.map(\.id).sorted(), for: PlayerKey.spotlightOwnedKeys)
        } catch {
            if generation == started {
                lastReconciliation = previousStart
            }
        }
    }

    /// Puts an impression whose page the player opened at the front of the history, and indexes
    /// it, owned or not. The one that falls out of the history (past 50) leaves the index unless
    /// the Collection holds it. An impression that cannot be read changes nothing.
    package func noteViewed(_ key: ImpressionKey) async {
        let started = generation
        do {
            guard let card = try await lookup.impression(for: key), generation == started else { return }
            let entity = await entity(for: card)
            guard generation == started else { return }
            let history = [key] + keys(for: .spotlightViewedKeys).filter { $0 != key }
            let evicted = Set(history.dropFirst(Self.historyLimit)).subtracting(keys(for: .spotlightOwnedKeys))
            try await index.index([entity])
            guard generation == started else {
                await discard([key])
                return
            }
            try await index.remove(Array(evicted))
            thumbnails.remove(Array(evicted))
            guard generation == started else { return }
            memory.write(history.prefix(Self.historyLimit).map(\.id), for: PlayerKey.spotlightViewedKeys)
        } catch {
            return
        }
    }

    /// Empties the index and forgets what was indexed and viewed: nobody is signed in any more.
    package func purge() async {
        generation += 1
        try? await index.removeAll()
        thumbnails.removeAll()
        memory.remove(.spotlightOwnedKeys)
        memory.remove(.spotlightViewedKeys)
        lastReconciliation = nil
    }

    /// What a call wrote while a purge ran: nobody is signed in any more, so it must not stay.
    private func discard(_ keys: [ImpressionKey]) async {
        try? await index.remove(keys)
        thumbnails.remove(keys)
    }

    private func keys(for key: PlayerKey) -> [ImpressionKey] {
        (memory.read([String].self, for: key) ?? []).compactMap { ImpressionKey(id: $0) }
    }

    /// The impression as Spotlight gets it, with its image downloaded once; without it when
    /// the download fails.
    private func entity(for card: SearchCard) async -> ImpressionEntity {
        var entity = ImpressionEntity(card)
        if let remote = card.image_url.flatMap({ URL(string: $0) }) {
            entity.thumbnailURL = await thumbnails.thumbnail(for: entity.key, from: remote)
        }
        return entity
    }

    private func ownedImpressions() async throws -> [ImpressionEntity] {
        var entities: [ImpressionEntity] = []
        for number in 0 ..< Self.maxPages {
            let query = Operations.search_cards.Input.Query(
                page: Int32(number),
                page_size: Self.pageSize,
                owned: true
            )
            let body = try await api.search_cards(query: query).ok.body.json
            for card in body.items {
                await entities.append(entity(for: card))
            }
            if body.items.isEmpty || entities.count >= body.total {
                break
            }
        }
        return entities
    }
}
