import Testing

@testable import ArcaneExchange

struct SearchModeTests {
    @Test func labelsEveryMode() {
        #expect(SearchMode.allCases.map(\.label) == ["Carte", "Decklist", "Joueur"])
    }
}

struct SearchRecentsTests {
    @Test func prependsTheNewestQuery() {
        let list = SearchRecents.adding("Sol Ring", to: ["Vampiric Tutor"])
        #expect(list == ["Sol Ring", "Vampiric Tutor"])
    }

    @Test func deduplicatesCaseInsensitivelyAndMovesToFront() {
        let list = SearchRecents.adding("vampiric tutor", to: ["Sol Ring", "Vampiric Tutor"])
        #expect(list == ["vampiric tutor", "Sol Ring"])
    }

    @Test func ignoresBlankQueries() {
        #expect(SearchRecents.adding("   ", to: ["Sol Ring"]) == ["Sol Ring"])
    }

    @Test func capsTheListAtTheLimit() {
        let seed = (1 ... SearchRecents.limit).map { "card \($0)" }
        let list = SearchRecents.adding("newest", to: seed)
        #expect(list.count == SearchRecents.limit)
        #expect(list.first == "newest")
        #expect(!list.contains("card \(SearchRecents.limit)"))
    }
}

struct RecentPlayersTests {
    private func player(_ name: String, cards: Int64 = 0) -> UserSuggestion {
        UserSuggestion(card_count: cards, note: 5, username: name)
    }

    @Test func prependsAndCapsAtFour() {
        var list: [UserSuggestion] = []
        for name in ["a", "b", "c", "d", "e"] {
            list = RecentPlayers.adding(player(name), to: list)
        }
        #expect(list.map(\.username) == ["e", "d", "c", "b"])
    }

    @Test func deduplicatesOnUsernameCaseInsensitively() {
        let seed = [player("Mizzix"), player("golgari")]
        let list = RecentPlayers.adding(player("mizzix", cards: 12), to: seed)
        #expect(list.map(\.username) == ["mizzix", "golgari"])
        #expect(list.first?.card_count == 12)
    }
}

struct PlayerMonogramTests {
    @Test func takesInitialsFromTheFirstTwoChunks() {
        #expect(PlayerMonogram.initials(from: "mizzix_42") == "M4")
        #expect(PlayerMonogram.initials(from: "golgari.jo") == "GJ")
    }

    @Test func fallsBackToTheFirstTwoCharacters() {
        #expect(PlayerMonogram.initials(from: "x") == "X")
        #expect(PlayerMonogram.initials(from: "!!") == "!!")
    }
}
