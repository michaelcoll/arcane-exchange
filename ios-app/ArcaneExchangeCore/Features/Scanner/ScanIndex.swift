import APIClient
import Foundation

/// One printing of the scan index (ADR-0024), keyed by set and collector number.
package struct ScanIndexEntry: Codable, Hashable, Sendable {
    package let setCode: String
    package let collectorNumber: String
    /// `C`, `U`, `R`, `M` or `S`.
    package let rarity: String
    package let name: String
    package let frenchName: String?

    package init(setCode: String, collectorNumber: String, rarity: String, name: String, frenchName: String?) {
        self.setCode = setCode
        self.collectorNumber = collectorNumber
        self.rarity = rarity
        self.name = name
        self.frenchName = frenchName
    }

    init(_ response: Components.Schemas.ScanIndexEntryResponse) {
        self.init(setCode: response.set_code, collectorNumber: response.collector_number, rarity: response.rarity,
                  name: response.name, frenchName: response.french_name)
    }
}

/// The catalog as the scanner knows it, in memory: what `GET /catalog/scan-index` returned.
package struct ScanIndex: Sendable {
    private struct Key: Hashable, Sendable {
        let set: String
        let number: String
    }

    private let entries: [Key: ScanIndexEntry]

    package init(entries: [ScanIndexEntry]) {
        self.entries = Dictionary(
            entries.map { (Key(set: $0.setCode.uppercased(), number: $0.collectorNumber), $0) },
            uniquingKeysWith: { first, _ in first }
        )
    }

    init(responses: [Components.Schemas.ScanIndexEntryResponse]) {
        self.init(entries: responses.map(ScanIndexEntry.init))
    }

    package func entry(set: String, number: String) -> ScanIndexEntry? {
        entries[Key(set: set.uppercased(), number: number)]
    }
}
