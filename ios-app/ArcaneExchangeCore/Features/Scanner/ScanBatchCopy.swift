import Foundation

/// The words of the scan batch drawer and validation sheet, with the French plural (0 and 1 are
/// singular), kept off the `View`: it is `@MainActor`, and tests call this.
package enum ScanBatchCopy {
    package static func cards(_ count: Int) -> String {
        "\(count) carte\(count > 1 ? "s" : "")"
    }

    /// "5 cartes · 3 uniques".
    package static func summary(cards: Int, uniques: Int) -> String {
        "\(Self.cards(cards)) · \(uniques) unique\(uniques > 1 ? "s" : "")"
    }

    /// The drawer's button.
    package static func addToCollection(cards: Int) -> String {
        "Ajouter \(Self.cards(cards)) à la collection"
    }

    /// The validation sheet's button.
    package static func add(cards: Int) -> String {
        "Ajouter \(Self.cards(cards))"
    }

    /// The batch value, and how many lines it leaves out for want of a trend price.
    package static func valueText(cents: Int, unpricedLines: Int) -> String {
        let total = "Valeur \(Price.euros(cents: cents))"
        guard unpricedLines > 0 else { return total }
        return "\(total) · \(unpricedLines) ligne\(unpricedLines > 1 ? "s" : "") sans prix"
    }
}
