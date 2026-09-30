/// Player initials for `PlayerAvatar` and the trades widget. A plain enum, not a `View` member:
/// `View` is `@MainActor`, and this pure string logic is called from off-main contexts (tests).
/// A file of its own so the widget extension can compile it without `PlayerAvatar`'s image
/// loading.
enum PlayerMonogram {
    /// Up to two letters: the initials of the first two `_`/`.`/`-`/space-separated chunks,
    /// falling back to the first two characters of the raw handle.
    static func initials(from username: String) -> String {
        let words = username.split { !$0.isLetter && !$0.isNumber }
        let letters = words.prefix(2).compactMap(\.first)
        return letters.isEmpty
            ? String(username.prefix(2)).uppercased()
            : String(letters).uppercased()
    }
}
