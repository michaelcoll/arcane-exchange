import Foundation

/// The login screen's own strings. Everything inside the Clerk auth view is Clerk's.
enum LoginCopy {
    /// The tagline for `proposedCopies` copies offered for trade across the platform, `nil` when
    /// the public stats did not answer. The count is rounded down to the hundred and written the
    /// French way whatever the device locale; under a hundred, or unknown, the tagline carries
    /// no number.
    static func tagline(proposedCopies: Int?) -> String {
        guard let proposedCopies, proposedCopies >= 100 else {
            return "Des cartes vous attendent dans les classeurs des joueurs."
        }
        let hundreds = (proposedCopies / 100 * 100).formatted(.number.locale(french))
        return "Plus de \(hundreds) cartes vous attendent dans les classeurs des joueurs."
    }

    private static let french = Locale(identifier: "fr_FR")
}
