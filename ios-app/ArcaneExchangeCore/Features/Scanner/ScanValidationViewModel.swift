import APIClient
import Foundation

/// Backs the sheet that validates the scan batch into the collection, the one step of the scanner
/// that needs the network. It sends the batch of the scanner with its identifier, so a resend after
/// a failure never duplicates the copies, and empties the batch once the API accepted it.
@MainActor
@Observable
package final class ScanValidationViewModel {
    private let scanner: ScannerViewModel
    private let api: any APIProtocol

    /// The binder to store the copies in; `nil` for none.
    package var binderID: String?
    /// The sealed product the copies come from; `nil` for none.
    package var sealedProductID: String?

    /// The player's binders. Empty until `load()` answered.
    package private(set) var binders: [Binder] = []
    /// The player's sealed products, most recent first. Empty until `load()` answered.
    package private(set) var sealedProducts: [Purchase] = []
    /// Set when the sealed products could not be read in full. French message, ready to show.
    package private(set) var loadError: String?

    package private(set) var isSending = false
    /// Set when the validation failed. French message, ready to show.
    package var error: String?
    /// The cards the catalog refused on the last send, as the batch named them.
    private var unknownCards: [Components.Schemas.UnknownCardResponse] = []

    package init(scanner: ScannerViewModel, api: any APIProtocol) {
        self.scanner = scanner
        self.api = api
    }

    package func load() async {
        // A failed read leaves "Aucun" as the only choice: the batch can still be validated.
        binders = await api.bindersOrEmpty()
        let read = await api.readSealedProducts()
        loadError = read.error
        sealedProducts = read.products
    }

    /// The lines of the batch the catalog refused, for the list to flag. Follows the batch: a line
    /// the player removed or corrected away is no longer flagged.
    package var rejectedLineIDs: Set<ScanBatchLine.Key> {
        Set(scanner.batch.lines.filter { line in
            unknownCards.contains {
                $0.set_code == line.set && $0.collector_number == line.number
                    && $0.language_code == line.language.rawValue
            }
        }.map(\.id))
    }

    package var canSend: Bool {
        !isSending && !scanner.batch.lines.isEmpty
    }

    /// Returns whether the batch was validated and emptied, so the sheet can close. On a failure the
    /// batch is left as it was, with the same identifier.
    package func send() async -> Bool {
        guard canSend else { return false }
        isSending = true
        defer { isSending = false }
        error = nil
        let batch = scanner.batch
        let body = Components.Schemas.ValidateScanBatchRequest(
            batch_id: batch.id.uuidString.lowercased(),
            binder_id: binderID,
            lines: batch.lines.map {
                .init(collector_number: $0.number, foil: $0.foil, language_code: $0.language.rawValue,
                      quantity: Int32($0.quantity), set_code: $0.set)
            },
            sealed_product_id: sealedProductID
        )
        do {
            // 201 the first time, 200 for a batch already validated: both mean it is in.
            let response = try await api.validate_scan_batch(body: .json(body))
            if (try? response.created) == nil {
                _ = try response.ok
            }
            scanner.clear()
            unknownCards = []
            binderID = nil
            sealedProductID = nil
            return true
        } catch {
            let apiError = APIError.read(error)
            unknownCards = apiError.unknownCards
            self.error = unknownCards.isEmpty ? apiError.message : Self.unknownCardsMessage
            return false
        }
    }

    static let unknownCardsMessage =
        "Des cartes du lot n'existent pas dans le catalogue : elles sont signalées dans la liste."
}
