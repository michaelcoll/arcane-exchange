/// The words of a binder row, kept off the `View`: it is `@MainActor`, and tests call this.
package enum BinderCopy {
    /// The choice, and the row label, of copies that sit in no binder.
    package static let noBinder = "Sans binder"

    /// "3 exemplaires · ouvert à l'échange", or "Vide" for an empty binder.
    package static func detail(copies: Int, isOpen: Bool) -> String {
        let size = copies == 0 ? "Vide" : TradeSettingsCopy.copies(copies)
        return isOpen ? "\(size) · ouvert à l'échange" : size
    }
}
