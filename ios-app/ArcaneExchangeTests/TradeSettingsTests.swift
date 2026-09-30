import Testing

@testable import ArcaneExchange

struct TradeSettingsTests {
    private func filter(_ rarity: String, isOpen: Bool, copies: Int64, proposed: Int64, kept: Int32 = 0) -> RarityFilter {
        RarityFilter(copies: copies, is_open: isOpen, kept_copies: kept, proposed: proposed, rarity: rarity)
    }

    @Test func splitsCopiesBetweenProposedKeptAndClosedRarities() {
        let ratio = TradeRatio(rarities: [
            filter("C", isOpen: true, copies: 100, proposed: 80, kept: 1),
            filter("R", isOpen: true, copies: 20, proposed: 5, kept: 2),
            filter("M", isOpen: false, copies: 30, proposed: 0),
        ])

        #expect(ratio.proposed == 85)
        #expect(ratio.kept == 35)
        #expect(ratio.excluded == 30)
        #expect(ratio.total == 150)
    }

    /// A closed rarity's `proposed` is zero server-side, but the band must not count it even if
    /// a stale row still carries one.
    @Test func ignoresWhatAClosedRarityClaimsToPropose() {
        let ratio = TradeRatio(rarities: [filter("R", isOpen: false, copies: 10, proposed: 4)])

        #expect(ratio.proposed == 0)
        #expect(ratio.excluded == 10)
        #expect(ratio.kept == 0)
    }

    @Test func collapsesToZeroWhenNothingIsOwned() {
        let ratio = TradeRatio(rarities: [])

        #expect(ratio.total == 0)
        #expect(ratio.share(0) == 0)
        #expect(ratio.summary == "0 % de la collection est proposée")
    }

    @Test func summarizesTheProposedShare() {
        let ratio = TradeRatio(rarities: [filter("C", isOpen: true, copies: 8, proposed: 2)])

        #expect(ratio.share(ratio.proposed) == 0.25)
        #expect(ratio.summary == "25 % de la collection est proposée")
    }

    @Test func labelsEveryVisibility() {
        #expect(CollectionVisibility.ordered.count == 3)
        #expect(CollectionVisibility.ordered.allSatisfy { !$0.label.isEmpty && !$0.detail.isEmpty })
        #expect(CollectionVisibility._public.label == "Publique")
    }

    @Test func pluralizesTheTradeSettingsCopy() {
        #expect(TradeSettingsCopy.copies(1) == "1 exemplaire")
        #expect(TradeSettingsCopy.cards(2) == "2 cartes")
        #expect(TradeSettingsCopy.proposed(3, isOpen: true) == "3 proposés")
        #expect(TradeSettingsCopy.proposed(1, isOpen: true) == "1 proposé")
        #expect(TradeSettingsCopy.proposed(7, isOpen: false) == "aucun proposé")
        #expect(TradeSettingsCopy.binderSelection(selected: 2, total: 5) == "2 sur 5")
        #expect(TradeSettingsCopy.openRarities(0) == "aucune ouverte")
        #expect(TradeSettingsCopy.openRarities(1) == "1 ouverte")
        #expect(TradeSettingsCopy.openRarities(4) == "4 ouvertes")
    }
}
