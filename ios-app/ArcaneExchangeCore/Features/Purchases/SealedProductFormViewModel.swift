import APIClient
import Foundation

/// Backs the form that records a sealed product, or edits one (`POST` and `PATCH /purchases`).
@MainActor
@Observable
package final class SealedProductFormViewModel {
    private let api: any APIProtocol
    /// The purchase being edited, nil for a creation.
    private let editing: Purchase?

    package var productType: SealedProductType
    /// Code of the chosen set, nil until one is picked.
    package var setCode: String?
    /// Optional label, e.g. the name of a preconstructed deck.
    package var name: String
    /// The amount in euros, as typed.
    package var priceText: String
    /// The day of the purchase: today for a creation.
    package var date: Date

    /// The sets the player picks from.
    package private(set) var sets: [SetInfo] = []
    /// Set when the sets could not be loaded. French message, ready to show.
    package private(set) var loadError: String?
    /// Set when the write failed. French message, ready to show.
    package private(set) var writeError: String?
    package private(set) var isSaving = false

    package init(api: any APIProtocol, editing purchase: Purchase? = nil) {
        self.api = api
        editing = purchase
        productType = purchase?.product_type ?? .BOOSTER
        setCode = purchase?.set?.code
        name = purchase?.name ?? ""
        priceText = purchase.map { EuroInput.text(cents: Int($0.amount_cents)) } ?? ""
        date = purchase.flatMap { PurchaseDate.date(from: $0.purchased_on) } ?? Date()
    }

    package var isEditing: Bool {
        editing != nil
    }

    /// The chosen set, shown by name.
    package var selectedSet: SetInfo? {
        sets.first { $0.code == setCode } ?? editing?.set.flatMap { $0.code == setCode ? $0 : nil }
    }

    /// A set is chosen and the price is a positive amount.
    package var canSubmit: Bool {
        setCode != nil && EuroInput.cents(from: priceText) != nil
    }

    package func loadSets() async {
        guard sets.isEmpty else { return }
        loadError = nil
        do {
            sets = try await api.list_sets().ok.body.json
        } catch {
            loadError = APIError.read(error).message
        }
    }

    /// Returns whether the purchase was saved, so the form can close.
    package func submit() async -> Bool {
        guard let setCode, let cents = EuroInput.cents(from: priceText) else { return false }
        isSaving = true
        writeError = nil
        defer { isSaving = false }
        let amount = Int32(cents)
        let day = PurchaseDate.string(from: date)
        do {
            if let editing {
                _ = try await api.update_purchase(
                    path: .init(id: editing.id),
                    body: .json(.init(
                        amount_cents: amount,
                        name: name,
                        product_type: productType,
                        purchased_on: day,
                        set_code: setCode
                    ))
                ).ok
            } else {
                _ = try await api.create_purchase(body: .json(.init(
                    amount_cents: amount,
                    name: name.isEmpty ? nil : name,
                    product_type: productType,
                    purchased_on: day,
                    set_code: setCode
                ))).created
            }
            return true
        } catch {
            writeError = APIError.read(error).message
            return false
        }
    }
}
