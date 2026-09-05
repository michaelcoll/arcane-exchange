import APIClient
import Foundation

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

extension SortDirection {
    /// The chip's arrow, which is the only thing telling the two directions apart at a glance.
    var icon: String {
        switch self {
        case .desc: "arrow.down"
        case .asc: "arrow.up"
        }
    }

    var label: String {
        switch self {
        case .desc: "Décroissant"
        case .asc: "Croissant"
        }
    }
}

extension SortField {
    /// The two sorts that mean something on a collection grid: what a card is worth, and when
    /// it was added. `avg` duplicates `trend`, `set_code`/`language_code` are filter facets.
    static let collectionOptions: [SortField] = [.trend, .added_at]

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

    /// The value on a filter row of the "Filtrer" drawer: nothing picked means no restriction,
    /// so it reads as the whole set rather than "0".
    ///
    /// A zero `total` means the list behind the facet has not loaded yet — the count alone is
    /// still true, "2 sur 0" would not be.
    static func facetSelection(selected: Int, total: Int, noneSelected: String) -> String {
        guard selected > 0 else { return noneSelected }
        return total > 0 ? "\(selected) sur \(total)" : "\(selected)"
    }

    /// "Filtres · 3" on the grid's chip, plain "Filtres" while nothing narrows the list.
    static func filterChip(activeCount: Int) -> String {
        activeCount == 0 ? "Filtres" : "Filtres · \(activeCount)"
    }
}

/// The sets drawer's search field. Purely on-device: `GET /collection/stats` already returned
/// every set the collection holds, so narrowing that list needs no request.
enum SetSearch {
    /// Keeps the sets whose name or code matches, ignoring case and accents. A blank query
    /// keeps them all.
    static func filter(_ sets: [SetInfo], matching query: String) -> [SetInfo] {
        let needle = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !needle.isEmpty else { return sets }
        return sets.filter { matches($0.name, needle) || matches($0.code, needle) }
    }

    private static func matches(_ value: String, _ needle: String) -> Bool {
        value.range(of: needle, options: [.caseInsensitive, .diacriticInsensitive]) != nil
    }
}
