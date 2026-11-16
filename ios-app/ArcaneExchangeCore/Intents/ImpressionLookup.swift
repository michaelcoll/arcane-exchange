import APIClient
import Foundation

/// Resolves impressions from the Catalogue search, for the App Intents: by a typed name, or by
/// the set and number of one already chosen or linked. The app's own link opening uses the
/// same lookup, so both land on exactly the same impression.
///
/// A player who is not signed in never reaches the API; a 401 reads the same way. Neither signs
/// anyone out: the client of the intents shares the app's session and only reads it.
package struct ImpressionLookup: Sendable {
    private let api: any APIProtocol
    private let isSignedIn: @Sendable () async -> Bool

    package init(api: any APIProtocol, isSignedIn: @escaping @Sendable () async -> Bool) {
        self.api = api
        self.isSignedIn = isSignedIn
    }

    /// One result per impression for a typed name, owned or not, offered or not: one page.
    package func impressions(matching text: String) async throws -> [SearchCard] {
        try await search(.init(q: text))
    }

    /// The entities of these keys, in order; one that is not found is left out. Signed out, the
    /// keys pass through as bare entities, without a call: the intent then offers to open the
    /// app, which a failing query could not.
    package func entities(for keys: [ImpressionKey]) async throws -> [ImpressionEntity] {
        guard await isSignedIn() else { return keys.map { ImpressionEntity(key: $0) } }
        var entities: [ImpressionEntity] = []
        for key in keys {
            if let card = try await impression(for: key) {
                entities.append(ImpressionEntity(card))
            }
        }
        return entities
    }

    /// What an intent answers from: the impression as it is now, or why there is none.
    package func card(for key: ImpressionKey) async -> Result<SearchCard, IntentFailure> {
        do {
            guard let card = try await impression(for: key) else { return .failure(.notFound) }
            return .success(card)
        } catch let failure as IntentFailure {
            return .failure(failure)
        } catch {
            return .failure(.unavailable(APIError.read(error).message))
        }
    }

    package func impression(for key: ImpressionKey) async throws -> SearchCard? {
        try await search(.init(sets: key.set, collector_number: key.number)).first
    }

    private func search(_ query: Operations.search_cards.Input.Query) async throws -> [SearchCard] {
        guard await isSignedIn() else { throw IntentFailure.signedOut }
        do {
            return try await api.search_cards(query: query).ok.body.json.items
        } catch {
            let apiError = APIError.read(error)
            throw apiError.code == "unauthorized" ? IntentFailure.signedOut : .unavailable(apiError.message)
        }
    }
}
