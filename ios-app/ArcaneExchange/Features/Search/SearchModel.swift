import APIClient
import SwiftUI

typealias UserSuggestion = Components.Schemas.UserSuggestionResponse

extension UserSuggestion {
    /// "312 cartes échangeables" / "1 carte échangeable" — the player row's subtitle.
    var tradableCountLabel: String {
        card_count > 1
            ? "\(card_count) cartes échangeables"
            : "\(card_count) carte échangeable"
    }
}

/// The three search modes the mockup's segmented control offers.
enum SearchMode: String, CaseIterable, Identifiable {
    case card
    case decklist
    case player

    var id: Self {
        self
    }

    var label: String {
        switch self {
        case .card: "Carte"
        case .decklist: "Decklist"
        case .player: "Joueur"
        }
    }

    var prompt: Text {
        switch self {
        case .card: Text("Vampiric Tutor…")
        case .decklist: Text("Nom d'une carte…")
        case .player: Text("Pseudo d'un joueur…")
        }
    }
}

/// A submitted search, pushed onto the Rechercher tab's stack.
struct SearchResultsRoute: Hashable {
    enum Target: Hashable {
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

    let target: Target

    var title: String {
        switch target {
        case let .card(query): query
        case let .player(username): "@\(username)"
        case .decklist: "Decklist"
        }
    }
}

/// Local, device-only recent card queries.
enum SearchRecents {
    static let storageKey = "search.recentCardQueries"
    static let limit = 8

    /// Prepends `query` (de-duplicated, case-insensitively), capped at `limit`.
    static func adding(_ query: String, to list: [String]) -> [String] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return list }
        var next = list.filter { $0.caseInsensitiveCompare(trimmed) != .orderedSame }
        next.insert(trimmed, at: 0)
        return Array(next.prefix(limit))
    }
}

/// Local, device-only recently opened players — mirrors the web `useRecentPlayers`.
enum RecentPlayers {
    static let storageKey = "search.recentPlayers"
    static let limit = 4

    static func adding(_ player: UserSuggestion, to list: [UserSuggestion]) -> [UserSuggestion] {
        guard !player.username.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return list }
        var next = list.filter { $0.username.caseInsensitiveCompare(player.username) != .orderedSame }
        next.insert(player, at: 0)
        return Array(next.prefix(limit))
    }
}
