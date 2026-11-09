import Foundation

/// The languages a copy can be added in: every one the backend's domain knows, whatever Scryfall
/// has of the card (the catalog has no language). Raw values are the API's language codes.
enum CardLanguage: String, CaseIterable, Codable, Identifiable {
    case french = "FR"
    case english = "EN"
    case german = "DE"
    case italian = "IT"
    case japanese = "JA"
    case spanish = "SP"

    /// The language picked when the player has not chosen one yet.
    static let `default` = CardLanguage.french

    var id: String {
        rawValue
    }

    var label: String {
        switch self {
        case .french: "Français"
        case .english: "Anglais"
        case .german: "Allemand"
        case .italian: "Italien"
        case .japanese: "Japonais"
        case .spanish: "Espagnol"
        }
    }
}
