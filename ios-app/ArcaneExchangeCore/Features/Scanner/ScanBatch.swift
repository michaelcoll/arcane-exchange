import Foundation

/// One line of a scan batch: a card in a language and a finish, and how many copies of it were
/// scanned. Carries the names the list shows, so it reads with no network and no catalog.
package struct ScanBatchLine: Codable, Hashable, Identifiable, Sendable {
    package let set: String
    package let number: String
    package let language: CardLanguage
    package let foil: Bool
    package fileprivate(set) var quantity: Int
    package let rarity: String
    package let name: String
    package let frenchName: String?

    /// A line is unique by (set, number, language, finish): that is its identity.
    package var id: String {
        "\(set)-\(number)-\(language.rawValue)-\(foil ? "foil" : "normal")"
    }

    /// The French name when there is one, the English one otherwise.
    package var displayName: String {
        frenchName ?? name
    }
}

/// The scan batch (« lot de scan »): the cards the camera recognised, waiting to be corrected and
/// validated. A pure value, with no camera, no storage and no clock: the view model feeds it and
/// saves it.
///
/// The most recent line comes first. A card added again goes to the top with its quantity raised.
package struct ScanBatch: Codable, Equatable, Sendable {
    package static let maxQuantity = 255

    /// Generated when the batch is created and kept until it is validated or emptied: it will be the
    /// idempotence key of the validation.
    package private(set) var id: UUID
    package private(set) var lines: [ScanBatchLine] = []
    /// The last card the camera added: what a re-read is compared to, and what an undo takes off.
    private var lastCameraAdd: CameraAdd?

    private struct CameraAdd: Codable, Equatable, Sendable {
        let set: String
        let number: String
        let language: CardLanguage
        let lineID: String
        /// False once a line was corrected by hand: the copy is no longer "the one just added".
        var canUndo = true

        func isSameCard(as card: RecognisedCard) -> Bool {
            set == card.set && number == card.number && language == card.language
        }
    }

    package init(id: UUID = UUID()) {
        self.id = id
    }

    /// The number of cards, the copies counted.
    package var cardCount: Int {
        lines.reduce(0) { $0 + $1.quantity }
    }

    /// Adds a card the camera recognised, in the finish the Foil switch had when it was read.
    /// Returns false when nothing was added: the same card as the last one the camera added (the
    /// finish does not matter), or a line already at the maximum quantity.
    @discardableResult
    package mutating func addFromCamera(_ card: RecognisedCard, foil: Bool) -> Bool {
        if lastCameraAdd?.isSameCard(as: card) == true {
            return false
        }
        var line = ScanBatchLine(set: card.set, number: card.number, language: card.language, foil: foil,
                                 quantity: 0, rarity: card.rarity, name: card.name, frenchName: card.frenchName)
        let existing = lines.firstIndex { $0.id == line.id }
        line.quantity = (existing.map { lines[$0].quantity } ?? 0) + 1
        guard line.quantity <= Self.maxQuantity else { return false }
        if let existing {
            lines.remove(at: existing)
        }
        lines.insert(line, at: 0)
        lastCameraAdd = CameraAdd(set: card.set, number: card.number, language: card.language, lineID: line.id)
        return true
    }

    /// Takes off the copy the camera just added. False when there is none to take off: already
    /// undone, or the batch was corrected by hand since.
    @discardableResult
    package mutating func undoLastCameraAdd() -> Bool {
        guard let last = lastCameraAdd, last.canUndo,
              let index = lines.firstIndex(where: { $0.id == last.lineID })
        else { return false }
        lines[index].quantity -= 1
        if lines[index].quantity == 0 {
            lines.remove(at: index)
        }
        // The same card read again counts again.
        lastCameraAdd = nil
        return true
    }

    /// Sets a line's quantity, kept between 0 and 255. 0 removes the line.
    package mutating func setQuantity(_ quantity: Int, ofLine id: String) {
        guard let index = lines.firstIndex(where: { $0.id == id }) else { return }
        let bounded = min(max(quantity, 0), Self.maxQuantity)
        if bounded == 0 {
            lines.remove(at: index)
        } else {
            lines[index].quantity = bounded
        }
        lastCameraAdd?.canUndo = false
    }

    /// Empties the batch. It gets a new identifier: it is a new batch.
    package mutating func clear() {
        self = ScanBatch()
    }
}
