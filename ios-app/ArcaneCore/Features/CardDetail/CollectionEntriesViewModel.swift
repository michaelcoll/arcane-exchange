import APIClient
import Foundation

package typealias CollectionEntryDetail = Components.Schemas.CollectionEntryDetailResponse

/// Backs the entries of a card in "Dans ma collection": what the player owns of the card in each
/// finish and binder, and the edit (quantity, finish) or removal of one entry. Every write answers
/// with the entries of the card, which replace the list: the screen shows the edit at once.
@MainActor
@Observable
package final class CollectionEntriesViewModel {
    private let card: Components.Schemas.CollectionCardResponse
    private let api: any APIProtocol

    package init(card: Components.Schemas.CollectionCardResponse, api: any APIProtocol) {
        self.card = card
        self.api = api
    }

    package private(set) var entries: [CollectionEntryDetail] = []
    /// Whether the server answered with the entries: until then, and after a failed read, the
    /// screen keeps what the card list showed.
    package private(set) var hasLoaded = false
    /// Copies committed by the accepted trades, per finish: the owned quantity cannot go under.
    package private(set) var reservedNormal = 0
    package private(set) var reservedFoil = 0
    /// The player's binders, the targets of a move. Empty until `loadBinders()` answered.
    package private(set) var binders: [Binder] = []
    package private(set) var isWriting = false
    /// Set when a write was refused: the list is intact. French message, ready to show.
    package var writeError: String?

    package func load() async {
        do {
            let output = try await api.get_collection_entries(path: .init(
                set_code: card.set_code,
                collector_number: card.collector_number,
                language_code: card.language_code
            ))
            try apply(output.ok.body.json)
        } catch {
            // The entries are an addition to the card screen: a failed read leaves it without them.
            entries = []
            hasLoaded = false
        }
    }

    package func reserved(foil: Bool) -> Int {
        foil ? reservedFoil : reservedNormal
    }

    /// Returns whether the edit was accepted, so the sheet that asked can close.
    package func edit(_ entry: CollectionEntryDetail, quantity: Int, foil: Bool) async -> Bool {
        await write {
            let body = Components.Schemas.UpdateCollectionEntryRequest(
                foil: foil == entry.foil ? nil : foil,
                quantity: quantity == Int(entry.quantity) ? nil : Int32(quantity)
            )
            let output = try await self.api.update_collection_entry(path: .init(id: entry.id), body: .json(body))
            try self.apply(output.ok.body.json)
        }
    }

    package func loadBinders() async {
        // A failed read leaves the move with "Sans binder" as its only target.
        binders = await api.bindersOrEmpty()
    }

    /// Moves `quantity` copies of the entry into the binder, or out of any binder when
    /// `binderID` is nil. Returns whether the move was accepted, so the sheet that asked can close.
    package func move(_ entry: CollectionEntryDetail, to binderID: String?, quantity: Int) async -> Bool {
        await write {
            let body = Components.Schemas.MoveCollectionEntryRequest(
                binder_id: binderID,
                quantity: Int32(quantity)
            )
            let output = try await self.api.move_collection_entry(path: .init(id: entry.id), body: .json(body))
            try self.apply(output.ok.body.json)
        }
    }

    /// Returns whether the entry was removed, so the sheet that asked can close.
    package func remove(_ entry: CollectionEntryDetail) async -> Bool {
        await write {
            _ = try await self.api.remove_collection_entry(path: .init(id: entry.id)).noContent
            // The removal answers nothing: read the card again for the entries and the reserved
            // quantities.
            await self.load()
        }
    }

    private func apply(_ response: Components.Schemas.CardEntriesResponse) {
        entries = response.entries
        hasLoaded = true
        reservedNormal = Int(response.reserved_quantity.normal)
        reservedFoil = Int(response.reserved_quantity.foil)
    }

    private func write(_ operation: () async throws -> Void) async -> Bool {
        isWriting = true
        defer { isWriting = false }
        do {
            try await operation()
            return true
        } catch {
            writeError = APIError.read(error).message
            return false
        }
    }
}
