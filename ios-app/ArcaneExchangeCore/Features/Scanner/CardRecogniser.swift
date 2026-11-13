import Foundation

/// A card recognised from its printed line: what the scanner shows, and what a scan batch will keep.
package struct RecognisedCard: Hashable, Sendable {
    package let set: String
    package let number: String
    /// The language read on the card, not the catalog's.
    package let language: CardLanguage
    package let rarity: String
    package let name: String
    package let frenchName: String?

    package init(
        set: String,
        number: String,
        language: CardLanguage,
        rarity: String,
        name: String,
        frenchName: String?
    ) {
        self.set = set
        self.number = number
        self.language = language
        self.rarity = rarity
        self.name = name
        self.frenchName = frenchName
    }

    /// The French name when there is one, the English one otherwise.
    package var displayName: String {
        frenchName ?? name
    }
}

/// The recognition seam: the text lines Vision read in one image and the scan index in, a recognised
/// card or nothing out. Pure, no camera and no Vision.
///
/// A reading is dropped when its set and number are not in the index, when it gives no rarity, or
/// when the rarity read differs from the index's: a token whose `T` was misread (`tmkm/12` read as
/// `mkm/12`) lands on a real printing of another rarity, and a reading that lost its letter cannot
/// rule that out. No reading is better than a wrong one. A non numeric number never gets this far
/// (`PrintedLineReader`).
package enum CardRecogniser {
    package static func recognise(lines: [String], in index: ScanIndex) -> RecognisedCard? {
        guard let reading = PrintedLineReader.read(lines: lines),
              let entry = index.entry(set: reading.set, number: reading.number),
              let printed = reading.rarity, indexRarity(entry.rarity, isPrintedAs: printed)
        else { return nil }
        return RecognisedCard(set: entry.setCode, number: entry.collectorNumber, language: reading.language,
                              rarity: entry.rarity, name: entry.name, frenchName: entry.frenchName)
    }

    /// Basic lands print `L` where the index, like the catalog, holds a common.
    private static func indexRarity(_ rarity: String, isPrintedAs printed: String) -> Bool {
        rarity == printed || (printed == "L" && rarity == "C")
    }
}
