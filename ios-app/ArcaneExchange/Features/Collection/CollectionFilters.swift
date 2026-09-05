import APIClient

/// Short names for the generated schema types this screen works with — the fully qualified
/// `Components.Schemas.…` spelling is unreadable inside SwiftUI bodies.
typealias CollectionCard = Components.Schemas.CollectionCardResponse
typealias RarityCode = Components.Schemas.RarityCodeParam
typealias SetInfo = Components.Schemas.SetInfoResponse
typealias SortField = Components.Schemas.SortByParam
typealias SortDirection = Components.Schemas.SortDirParam

/// What `GET /collection` is asked for, minus the pagination the view model owns.
///
/// `Hashable` on purpose: the view drives reloads with `.task(id:)`, so any change here — a
/// rarity toggled, a different sort — re-runs the request without an explicit refresh call.
struct CollectionFilters: Hashable {
    var rarities: Set<RarityCode> = []
    var sets: Set<String> = []
    var sortBy: SortField = .trend
    var sortDir: SortDirection = .desc

    var activeCount: Int {
        rarities.count + sets.count
    }

    mutating func clearAll() {
        rarities = []
        sets = []
    }
}

extension RarityCode {
    /// Plural labels, as the mockup's filter sheet lists them.
    var label: String {
        switch self {
        case .C: "Communes"
        case .U: "Peu communes"
        case .R: "Rares"
        case .M: "Mythiques"
        case .S: "Spéciales"
        }
    }
}

extension SortField {
    /// Only the fields worth exposing on this screen; `language_code` sorts a collection that
    /// is almost entirely one language, so it stays out of the menu.
    static let collectionOptions: [SortField] = [.trend, .avg, .set_code, .added_at]

    var label: String {
        switch self {
        case .trend: "Valeur"
        case .avg: "Prix moyen"
        case .set_code: "Set"
        case .language_code: "Langue"
        case .added_at: "Ajout"
        }
    }
}

enum CollectionCopy {
    static func cardCount(_ count: Int) -> String {
        count > 1 ? "\(count) cartes" : "\(count) carte"
    }

    /// "3 joueurs la proposent" / "1 joueur la propose" — the owners-list section header.
    static func offerCount(_ count: Int) -> String {
        count > 1 ? "\(count) joueurs la proposent" : "\(count) joueur la propose"
    }

    /// "2 disponibles · 1 réservée" — the "Possesseurs" section header, which splits the
    /// copies that can be asked for from those already locked into an accepted trade.
    static func offerAvailability(available: Int, reserved: Int) -> String {
        let head = available > 1 ? "\(available) disponibles" : "\(available) disponible"
        guard reserved > 0 else { return head }
        return reserved > 1 ? "\(head) · \(reserved) réservées" : "\(head) · \(reserved) réservée"
    }
}
