import APIClient
import Foundation

/// Backs the form that adds copies of a catalog impression to the collection: language, finish,
/// quantity and binder. The language defaults to the last one the player added a card in, French
/// at first.
@MainActor
@Observable
package final class AddCollectionEntryViewModel {
    package let card: CatalogCard
    private let api: any APIProtocol
    private let memory: DeviceMemory

    package var language: CardLanguage
    package var foil = false
    package var quantity = 1
    /// The binder to store the copies in; `nil` for none.
    package var binderID: String?

    /// The player's binders. Empty until `loadBinders()` answered.
    package private(set) var binders: [Components.Schemas.BinderResponse] = []
    package private(set) var isWriting = false
    /// Set when the addition was refused. French message, ready to show.
    package var writeError: String?

    package init(card: CatalogCard, api: any APIProtocol, defaults: UserDefaults = .standard) {
        self.card = card
        self.api = api
        memory = DeviceMemory(defaults: defaults)
        language = memory.read(CardLanguage.self, for: PlayerKey.addCardLanguage) ?? .default
    }

    package func loadBinders() async {
        // A failed read leaves "Sans binder" as the only choice.
        binders = await (try? api.list_binders().ok.body.json.binders) ?? []
    }

    /// Returns whether the copies were added, so the form can close.
    package func add() async -> Bool {
        isWriting = true
        defer { isWriting = false }
        do {
            let body = Components.Schemas.AddCollectionEntryRequest(
                binder_id: binderID,
                collector_number: card.collector_number,
                foil: foil,
                language_code: language.rawValue,
                quantity: Int32(quantity),
                set_code: card.set.code
            )
            _ = try await api.add_collection_entry(body: .json(body)).created
            memory.write(language, for: PlayerKey.addCardLanguage)
            return true
        } catch {
            writeError = APIError.read(error).message
            return false
        }
    }
}
