import Testing

@testable import ArcaneExchange

struct LoginCopyTests {
    /// French grouping is a narrow no-break space (U+202F), whatever the device locale.
    @Test func announcesTheProposedCopiesRoundedDownToTheHundred() {
        #expect(
            LoginCopy.tagline(proposedCopies: 1248)
                == "Plus de 1\u{202F}200 cartes vous attendent dans les classeurs des joueurs."
        )
    }

    @Test func announcesTheFirstHundredAsSoonAsItIsReached() {
        #expect(
            LoginCopy.tagline(proposedCopies: 100)
                == "Plus de 100 cartes vous attendent dans les classeurs des joueurs."
        )
        #expect(
            LoginCopy.tagline(proposedCopies: 199)
                == "Plus de 100 cartes vous attendent dans les classeurs des joueurs."
        )
    }

    @Test func groupsThousandsTheFrenchWay() {
        #expect(
            LoginCopy.tagline(proposedCopies: 1_234_567)
                == "Plus de 1\u{202F}234\u{202F}500 cartes vous attendent dans les classeurs des joueurs."
        )
    }

    /// A small number would discourage a visitor: the tagline says nothing of it.
    @Test(arguments: [0, 1, 99])
    func carriesNoNumberUnderAHundredProposedCopies(proposedCopies: Int) {
        #expect(
            LoginCopy.tagline(proposedCopies: proposedCopies)
                == "Des cartes vous attendent dans les classeurs des joueurs."
        )
    }

    /// Offline or a server error: the count is unknown.
    @Test func carriesNoNumberWhenTheStatsAreUnavailable() {
        #expect(
            LoginCopy.tagline(proposedCopies: nil)
                == "Des cartes vous attendent dans les classeurs des joueurs."
        )
    }
}
