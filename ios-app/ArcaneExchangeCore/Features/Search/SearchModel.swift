import APIClient
import SwiftUI

package typealias UserSuggestion = Components.Schemas.UserSuggestionResponse

package extension UserSuggestion {
    /// "312 cartes échangeables" / "1 carte échangeable" — the player row's subtitle.
    var tradableCountLabel: String {
        card_count > 1
            ? "\(card_count) cartes échangeables"
            : "\(card_count) carte échangeable"
    }
}

/// The three search modes the mockup's segmented control offers.
package enum SearchMode: String, CaseIterable, Identifiable {
    case card
    case decklist
    case player

    package var id: Self {
        self
    }

    package var label: String {
        switch self {
        case .card: "Carte"
        case .decklist: "Decklist"
        case .player: "Joueur"
        }
    }

    package var prompt: Text {
        switch self {
        case .card: Text("Vampiric Tutor…")
        case .decklist: Text("Nom d'une carte…")
        case .player: Text("Pseudo d'un joueur…")
        }
    }
}

/// A submitted search, pushed onto the Rechercher tab's stack.
package struct SearchResultsRoute: Hashable {
    package enum Target: Hashable {
        /// Free-text search on card name or set (`GET /search/card?q=`).
        case card(query: String)
        /// One player's tradable collection (`GET /search/card?player_username=`).
        ///
        /// Just the handle: it is all `/search/card` is given, and it lets the trade screen
        /// open a partner's collection without inventing a `UserSuggestion` it never had.
        case player(username: String)
        /// Not wired: the API has no batch endpoint, it is one `/search/card` call per line.
        case decklist
    }

    package let target: Target

    package init(target: Target) {
        self.target = target
    }

    package var title: String {
        switch target {
        case let .card(query): query
        case let .player(username): "@\(username)"
        case .decklist: "Decklist"
        }
    }
}

/// Local, device-only recent card queries.
package enum SearchRecents {
    package static let limit = 8

    /// Prepends `query` (de-duplicated, case-insensitively), capped at `limit`.
    package static func adding(_ query: String, to list: [String]) -> [String] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return list }
        var next = list.filter { $0.caseInsensitiveCompare(trimmed) != .orderedSame }
        next.insert(trimmed, at: 0)
        return Array(next.prefix(limit))
    }
}

/// Local, device-only recently opened players — mirrors the web `useRecentPlayers`.
package enum RecentPlayers {
    package static let limit = 4

    package static func adding(_ player: UserSuggestion, to list: [UserSuggestion]) -> [UserSuggestion] {
        guard !player.username.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return list }
        var next = list.filter { $0.username.caseInsensitiveCompare(player.username) != .orderedSame }
        next.insert(player, at: 0)
        return Array(next.prefix(limit))
    }
}
