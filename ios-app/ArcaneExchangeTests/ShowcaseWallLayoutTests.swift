import Testing

@testable import ArcaneExchange

struct ShowcaseWallLayoutTests {
    /// The showcase comes most expensive first: dealing it out keeps the best cards at the top
    /// of every column rather than all in the first one.
    @Test func dealsTheCardsOutAcrossTheColumnsInOrder() {
        let columns = ShowcaseWallLayout.columns(Array(1 ... 9), count: 3, minimumPerColumn: 1)
        #expect(columns == [[1, 4, 7], [2, 5, 8], [3, 6, 9]])
    }

    @Test func splitsThirtyCardsIntoThreeColumnsOfTen() {
        let columns = ShowcaseWallLayout.columns(Array(1 ... 30), count: 3, minimumPerColumn: 5)
        #expect(columns.map(\.count) == [10, 10, 10])
        #expect(columns.flatMap(\.self).sorted() == Array(1 ... 30))
    }

    @Test func leavesTheLastColumnsShorterWhenTheCardsDoNotDivideEvenly() {
        let columns = ShowcaseWallLayout.columns(Array(1 ... 7), count: 3, minimumPerColumn: 1)
        #expect(columns == [[1, 4, 7], [2, 5], [3, 6]])
    }

    /// A column shorter than the screen would show a hole while it drifts: a small showcase
    /// repeats its cards until every column is tall enough.
    @Test func repeatsTheCardsOfASmallShowcaseUntilEveryColumnIsTallEnough() {
        let columns = ShowcaseWallLayout.columns([1, 2, 3, 4], count: 3, minimumPerColumn: 3)
        #expect(columns == [[1, 4, 3], [2, 1, 4], [3, 2, 1]])
    }

    @Test func fillsEveryColumnFromASingleCard() {
        let columns = ShowcaseWallLayout.columns(["A"], count: 3, minimumPerColumn: 2)
        #expect(columns == [["A", "A"], ["A", "A"], ["A", "A"]])
    }

    /// No wall at all: the login screen keeps its plain background.
    @Test func hasNoColumnWithoutCards() {
        #expect(ShowcaseWallLayout.columns([Int](), count: 3, minimumPerColumn: 5).isEmpty)
    }
}
