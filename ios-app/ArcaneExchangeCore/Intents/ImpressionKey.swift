import Foundation

/// What names an impression across the system and the app: its set and collector number.
/// Never a name: two impressions of one card share it, and a neighbour with a close name must
/// never be taken for it.
package struct ImpressionKey: Hashable, Sendable {
    package let set: String
    package let number: String

    package init(set: String, number: String) {
        self.set = set
        self.number = number
    }

    package init(_ card: SearchCard) {
        self.init(set: card.set_code, number: card.collector_number)
    }

    /// The identifier of the App Entity: a set code holds no `/`, so the first one splits.
    package var id: String {
        "\(set)/\(number)"
    }

    package init?(id: String) {
        let parts = id.split(separator: "/", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2, !parts[0].isEmpty, !parts[1].isEmpty else { return nil }
        self.init(set: String(parts[0]), number: String(parts[1]))
    }

    /// "FDN · 001": how the set and the number read to a player.
    package var label: String {
        "\(set) · \(number)"
    }
}
