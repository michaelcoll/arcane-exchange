import Foundation

/// The languages a copy can be added in: every one the backend's domain knows, whatever Scryfall
/// has of the card (the catalog has no language). Raw values are the API's language codes.
package enum CardLanguage: String, CaseIterable, Codable, Identifiable {
    case french = "FR"
    case english = "EN"
    case german = "DE"
    case italian = "IT"
    case japanese = "JA"
    case spanish = "SP"

    /// The language picked when the player has not chosen one yet.
    package static let `default` = CardLanguage.french

    package var id: String {
        rawValue
    }

    package var label: String {
        switch self {
        case .french: "Français"
        case .english: "Anglais"
        case .german: "Allemand"
        case .italian: "Italien"
        case .japanese: "Japonais"
        case .spanish: "Espagnol"
        }
    }

    /// The name a card of this language shows: the English one for an English card, the French one
    /// for any other language, the English one when there is no French name.
    package func displayName(name: String, frenchName: String?) -> String {
        self == .english ? name : frenchName ?? name
    }
}
