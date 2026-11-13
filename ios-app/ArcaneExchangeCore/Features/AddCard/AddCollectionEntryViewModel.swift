import APIClient
import Foundation

/// Backs the form that adds copies of a catalog impression to the collection: language, finish,
/// quantity, binder and an optional purchase price per copy. The language defaults to the last one
/// the player added a card in, French at first.
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
    /// The sealed product the copies come from; `nil` for none. Set by `chooseSealedProduct(_:)`.
    package private(set) var sealedProductID: String?
    /// The purchase price of one copy in euros, as typed. Empty for a copy with no price, which
    /// records no purchase. Not used, and hidden by the form, once a sealed product is chosen.
    package var priceText = ""

    /// The player's binders. Empty until `loadBinders()` answered.
    package private(set) var binders: [Binder] = []
    /// The player's sealed products, most recent first. Empty until `loadSealedProducts()` answered.
    package private(set) var sealedProducts: [Purchase] = []
    /// Set when the sealed products could not be read in full. French message, ready to show.
    package private(set) var loadError: String?
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
        binders = await api.bindersOrEmpty()
    }

    package func loadSealedProducts() async {
        // A failed read leaves "Aucun" as the only choice, or the products read so far: the
        // copies can still be added.
        let read = await api.readSealedProducts()
        loadError = read.error
        sealedProducts = read.products
    }

    /// Picks the sealed product the copies come from, or none. The price typed so far goes: a
    /// sealed product carries none.
    package func chooseSealedProduct(_ id: String?) {
        sealedProductID = id
        priceText = ""
    }

    /// The price of one copy in cents; nil when a sealed product is chosen, or the field is empty
    /// or is not a positive amount.
    private var priceCents: Int? {
        sealedProductID == nil ? EuroInput.cents(from: priceText) : nil
    }

    /// The price is empty (no purchase), a positive amount, or ignored for a sealed product.
    package var canSubmit: Bool {
        sealedProductID != nil || priceText.trimmingCharacters(in: .whitespaces).isEmpty || priceCents != nil
    }

    /// Returns whether the copies were added, so the form can close.
    package func add() async -> Bool {
        guard canSubmit else { return false }
        isWriting = true
        defer { isWriting = false }
        do {
            let body = Components.Schemas.AddCollectionEntryRequest(
                binder_id: binderID,
                collector_number: card.collector_number,
                foil: foil,
                language_code: language.rawValue,
                purchase_price: priceCents.map { Int32($0) },
                quantity: Int32(quantity),
                sealed_product_id: sealedProductID,
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
