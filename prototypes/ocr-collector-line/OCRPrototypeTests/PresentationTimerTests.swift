import Testing
@testable import OCRPrototype

struct PresentationTimerTests {
    let reading = CollectorReading(set: "mom", number: "123", language: "en", isToken: false)
    let other = CollectorReading(set: "mom", number: "124", language: "en", isToken: false)
    let lines = ["Elesh Norn", "R 0123", "MOM • EN"]

    @Test func retainsAReadingSeenOnTwoConsecutiveFramesAndReportsTheLatency() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: lines, reading: reading, at: 1.0) == nil)
        #expect(timer.observe(lines: lines, reading: reading, at: 1.5) == 0.5)
    }

    @Test func reportsOnlyOncePerPresentation() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 0)
        _ = timer.observe(lines: lines, reading: reading, at: 0.2)
        #expect(timer.observe(lines: lines, reading: reading, at: 0.4) == nil)
    }

    @Test func aDifferentReadingRestartsTheCountButNotTheClock() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 1.0)
        #expect(timer.observe(lines: lines, reading: other, at: 1.2) == nil)
        #expect(timer.observe(lines: lines, reading: other, at: 1.5) == 0.5)
    }

    @Test func aFrameWithoutReadingBreaksTheSequence() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 1.0)
        _ = timer.observe(lines: lines, reading: nil, at: 1.2)
        #expect(timer.observe(lines: lines, reading: reading, at: 1.4) == nil)
    }

    @Test func anIncompleteReadingIsNeverRetained() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: lines, reading: nil, at: 1.0) == nil)
        #expect(timer.observe(lines: lines, reading: nil, at: 1.5) == nil)
    }

    @Test func aNewCardAfterAnEmptyLensStartsANewMeasure() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 0)
        _ = timer.observe(lines: lines, reading: reading, at: 0.2)
        _ = timer.observe(lines: [], reading: nil, at: 0.5)
        #expect(timer.observe(lines: lines, reading: other, at: 5.0) == nil)
        #expect(timer.observe(lines: lines, reading: other, at: 5.25) == 0.25)
    }

    @Test func aNewCardSwappedInWithoutEmptyingTheLensIsRetained() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 0)
        _ = timer.observe(lines: lines, reading: reading, at: 0.2)
        _ = timer.observe(lines: lines, reading: nil, at: 0.4)
        _ = timer.observe(lines: lines, reading: other, at: 0.6)
        #expect(timer.observe(lines: lines, reading: other, at: 0.8) == 0.4)
    }

    @Test func aNewCardReadAtOnceIsMeasuredFromItsFirstFrame() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 0)
        _ = timer.observe(lines: lines, reading: reading, at: 0.2)
        _ = timer.observe(lines: lines, reading: other, at: 0.5)
        #expect(timer.observe(lines: lines, reading: other, at: 0.75) == 0.25)
    }

    @Test func theRetainedCardBlurredForAFrameIsNotRetainedAgain() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 0)
        _ = timer.observe(lines: lines, reading: reading, at: 0.2)
        _ = timer.observe(lines: lines, reading: nil, at: 0.4)
        #expect(timer.observe(lines: lines, reading: reading, at: 0.6) == nil)
        #expect(timer.observe(lines: lines, reading: reading, at: 0.8) == nil)
    }

    @Test func aBlurredFrameOfTheRetainedCardDoesNotStartTheNextCardsClock() {
        var timer = PresentationTimer()
        _ = timer.observe(lines: lines, reading: reading, at: 0)
        _ = timer.observe(lines: lines, reading: reading, at: 0.2)
        _ = timer.observe(lines: lines, reading: nil, at: 0.4)
        _ = timer.observe(lines: lines, reading: reading, at: 0.6)
        _ = timer.observe(lines: lines, reading: other, at: 1.0)
        #expect(timer.observe(lines: lines, reading: other, at: 1.25) == 0.25)
    }

    @Test func tooFewLinesIsNoCard() {
        var timer = PresentationTimer()
        #expect(timer.observe(lines: ["a"], reading: reading, at: 0) == nil)
        #expect(timer.observe(lines: ["a"], reading: reading, at: 0.2) == nil)
    }
}
