import Foundation

/// One line of a scan batch: a card in a language and a finish, and how many copies of it were
/// scanned. Carries the names the list shows, so it reads with no network and no catalog.
package struct ScanBatchLine: Codable, Hashable, Identifiable, Sendable {
    /// What makes a line unique: the card, in a language and a finish.
    package struct Key: Codable, Hashable, Sendable {
        package let set: String
        package let number: String
        package let language: CardLanguage
        package let foil: Bool

        package init(set: String, number: String, language: CardLanguage, foil: Bool) {
            self.set = set
            self.number = number
            self.language = language
            self.foil = foil
        }
    }

    package let set: String
    package let number: String
    package let language: CardLanguage
    package let foil: Bool
    package fileprivate(set) var quantity: Int
    package let rarity: String
    package let name: String
    package let frenchName: String?

    package var id: Key {
        Key(set: set, number: number, language: language, foil: foil)
    }

    /// The French name when there is one, the English one otherwise.
    package var displayName: String {
        frenchName ?? name
    }
}

/// In an extension, so the type keeps its memberwise initializer.
private extension ScanBatchLine {
    init(card: RecognisedCard, language: CardLanguage, foil: Bool, quantity: Int) {
        self.init(set: card.set, number: card.number, language: language, foil: foil, quantity: quantity,
                  rarity: card.rarity, name: card.name, frenchName: card.frenchName)
    }

    /// The same line in another language and/or finish.
    func with(language: CardLanguage? = nil, foil: Bool? = nil) -> ScanBatchLine {
        ScanBatchLine(set: set, number: number, language: language ?? self.language, foil: foil ?? self.foil,
                      quantity: quantity, rarity: rarity, name: name, frenchName: frenchName)
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
        let lineID: ScanBatchLine.Key
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
        guard lastCameraAdd?.isSameCard(as: card) != true,
              let line = addOneCopy(of: card, language: card.language, foil: foil)
        else { return false }
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
    package mutating func setQuantity(_ quantity: Int, ofLine id: ScanBatchLine.Key) {
        guard let index = lines.firstIndex(where: { $0.id == id }) else { return }
        let bounded = min(max(quantity, 0), Self.maxQuantity)
        if bounded == 0 {
            lines.remove(at: index)
        } else {
            lines[index].quantity = bounded
        }
        lastCameraAdd?.canUndo = false
    }

    /// Changes the language of a line. When another line is then the same card in the same language and
    /// finish, the two merge and their quantities add up.
    package mutating func setLanguage(_ language: CardLanguage, ofLine id: ScanBatchLine.Key) {
        guard let index = lines.firstIndex(where: { $0.id == id }) else { return }
        correct(lineAt: index, to: lines[index].with(language: language))
    }

    /// Changes the finish of a line, merging it with the line it then equals.
    package mutating func setFoil(_ foil: Bool, ofLine id: ScanBatchLine.Key) {
        guard let index = lines.firstIndex(where: { $0.id == id }) else { return }
        correct(lineAt: index, to: lines[index].with(foil: foil))
    }

    /// Replaces the card of a line, a wrong reading, by another one. Quantity, language and finish stay.
    package mutating func replaceCard(ofLine id: ScanBatchLine.Key, with card: RecognisedCard) {
        guard let index = lines.firstIndex(where: { $0.id == id }) else { return }
        let line = lines[index]
        correct(lineAt: index, to: ScanBatchLine(card: card, language: line.language, foil: line.foil,
                                                 quantity: line.quantity))
    }

    /// Adds a card picked in the catalog by hand: one copy, at the top. A card the camera cannot read
    /// goes this way. A line already at the maximum quantity stays as it is.
    package mutating func addFromCatalog(_ card: RecognisedCard, language: CardLanguage, foil: Bool) {
        if addOneCopy(of: card, language: language, foil: foil) != nil {
            lastCameraAdd?.canUndo = false
        }
    }

    /// Puts one more copy of the card, in this language and finish, at the top of the list, merged
    /// with its line when there is one. Nil, and nothing changed, when that line is at the maximum.
    private mutating func addOneCopy(of card: RecognisedCard, language: CardLanguage, foil: Bool) -> ScanBatchLine? {
        var line = ScanBatchLine(card: card, language: language, foil: foil, quantity: 1)
        let existing = lines.firstIndex { $0.id == line.id }
        line.quantity += existing.map { lines[$0].quantity } ?? 0
        guard line.quantity <= Self.maxQuantity else { return nil }
        if let existing {
            lines.remove(at: existing)
        }
        lines.insert(line, at: 0)
        return line
    }

    /// Puts a corrected line in place of the line at `index`, merging it with the line it equals.
    private mutating func correct(lineAt index: Int, to corrected: ScanBatchLine) {
        var corrected = corrected
        if let other = lines.firstIndex(where: { $0.id == corrected.id }), other != index {
            corrected.quantity = min(corrected.quantity + lines[other].quantity, Self.maxQuantity)
            lines[index] = corrected
            lines.remove(at: other)
        } else {
            lines[index] = corrected
        }
        lastCameraAdd?.canUndo = false
    }

    /// Empties the batch. It gets a new identifier: it is a new batch.
    package mutating func clear() {
        self = ScanBatch()
    }
}
